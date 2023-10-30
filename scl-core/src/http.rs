//! HTTP 包装，虽然是内部使用但是你也可以使用这个来做点 HTTP 请求什么的
//!
//! 或者在二次开发的时候更换成你喜欢的版本

use std::{convert::TryInto, sync::Arc, time::Duration};

use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use surf::*;

use crate::prelude::*;

#[allow(dead_code)]
fn logger(
    req: Request,
    client: Client,
    next: middleware::Next,
) -> futures::future::BoxFuture<Result<Response>> {
    Box::pin(async move {
        let url = req.url().to_string();
        let should_log = std::env::var("SCL_HTTP_LOG")
            .map(|x| &x == "true")
            .unwrap_or(false);
        if should_log {
            tracing::trace!("[SCL-Core-HTTP] 正在请求 {url}");
        }
        let res = next.run(req, client).await?;
        if let Some(content_type) = res.content_type() {
            if should_log {
                tracing::trace!(
                    "[SCL-Core-HTTP] 请求 {} 完成 状态码：{} 响应类型：{}",
                    url,
                    res.status(),
                    content_type
                );
                if res.status().is_redirection() {
                    tracing::trace!(
                        "[SCL-Core-HTTP] 正在重定向至 {}",
                        res.header("Location").map(|x| x.as_str()).unwrap_or("")
                    );
                }
            }
        } else if should_log {
            tracing::trace!(
                "[SCL-Core-HTTP] 请求 {} 完成 状态码：{} 响应类型：无",
                url,
                res.status()
            );
            if res.status().is_redirection() {
                tracing::trace!(
                    "[SCL-Core-HTTP] 正在重定向至 {}",
                    res.header("Location").map(|x| x.as_str()).unwrap_or("")
                );
            }
        }
        Ok(res)
    })
}

static GLOBAL_CLIENT: Lazy<Arc<Client>> = Lazy::new(|| {
    let client = Config::new()
        .add_header(
            "User-Agent",
            "github.com/Steve-xmh/SharpCraftLauncher (stevexmh@qq.com)",
        )
        .unwrap()
        .set_timeout(Some(Duration::from_secs(30)));
    let client = if let Ok(mut proxy) = std::env::var("HTTP_PROXY") {
        let proxy = if proxy.ends_with('/') {
            proxy
        } else {
            proxy.push('/');
            proxy
        };
        if let Ok(uri) = url::Url::parse(&proxy) {
            tracing::trace!("Using http proxy: {uri}");
            client.set_base_url(uri)
        } else {
            client
        }
    } else {
        client
    };
    let client: Client = client.try_into().unwrap();
    Arc::new(client.with(middleware::Redirect::default()))
});

/// Future 重试调用函数，为下载文件失败重试而准备
///
/// 主要是 surf 库不带重试功能，中间件写了也有大堆问题。。。
pub async fn retry_future<O, F: std::future::Future<Output = O>>(
    max_retries: usize,
    future_builder: impl Fn() -> F,
    error_handler: impl Fn(&O) -> bool,
) -> DynResult<O> {
    let mut retries = 0;
    loop {
        retries += 1;
        let f = future_builder();
        let r = f.await;
        if error_handler(&r) || retries >= max_retries {
            return Ok(r);
        }
    }
}

/// 根据所给链接，依次尝试请求下载
///
/// 启发自 PCL1 源代码
///
/// TODO: 如 size 参数为非零值，则将会使用分片下载
pub async fn download(
    uris: &[impl AsRef<str> + std::fmt::Debug],
    dest_path: &str,
    _size: usize,
) -> DynResult {
    for uri in uris {
        // 尝试重试两次，都失败的话就换下一个链接
        let res = retry_future(5, || get(uri), surf::Result::is_ok).await;
        match res {
            Ok(Ok(res)) => {
                if res.status().is_success() {
                    let tmp_dest_path = format!("{dest_path}.tmp");
                    let tmp_file = inner_future::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(&tmp_dest_path)
                        .await?;
                    if inner_future::io::copy(res, tmp_file).await.is_ok() {
                        inner_future::fs::rename(tmp_dest_path, dest_path).await?;
                        return Ok(());
                    }
                } else {
                    tracing::trace!("Error {:?} 状态码错误 {}", uri, res.status());
                }
            }
            Ok(Err(e)) => {
                tracing::trace!("Error {uri:?} {e}")
            }
            Err(e) => {
                tracing::trace!("Error {uri:?} {e}")
            }
        }
    }
    anyhow::bail!(
        "轮询下载文件到 {} 失败，请检查你的网络连接，已尝试的链接 {:?}",
        dest_path,
        uris
    )
}

/// 重试获取 JSON 对象
///
/// 返回的数据结构需要实现 [`serde::de::DeserializeOwned`]
pub async fn retry_get_json<D: DeserializeOwned>(uri: impl AsRef<str>) -> DynResult<D> {
    let res = retry_future(5, || get(uri.as_ref()).recv_json(), surf::Result::is_ok).await;
    let err = match res {
        Ok(Ok(body)) => return Ok(body),
        Ok(Err(e)) => {
            anyhow::anyhow!("{}", e)
        }
        Err(e) => e,
    };
    anyhow::bail!(
        "轮询请求链接 {} 失败，请检查你的网络连接：{}",
        uri.as_ref(),
        err
    )
}

/// 重试获取数据
pub async fn retry_get_bytes(uri: impl AsRef<str>) -> DynResult<Vec<u8>> {
    let res = retry_future(5, || get(uri.as_ref()).recv_bytes(), surf::Result::is_ok).await;
    let err = match res {
        Ok(Ok(body)) => return Ok(body),
        Ok(Err(e)) => {
            anyhow::anyhow!("{}", e)
        }
        Err(e) => e,
    };
    anyhow::bail!(
        "轮询请求链接 {} 失败，请检查你的网络连接：{}",
        uri.as_ref(),
        err
    )
}

/// 重试获取字符串
pub async fn retry_get_string(uri: impl AsRef<str>) -> DynResult<String> {
    let res = retry_future(5, || get(uri.as_ref()).recv_string(), surf::Result::is_ok).await;
    let err = match res {
        Ok(Ok(body)) => return Ok(body),
        Ok(Err(e)) => {
            anyhow::anyhow!("{}", e)
        }
        Err(e) => e,
    };
    anyhow::bail!(
        "轮询请求链接 {} 失败，请检查你的网络连接：{}",
        uri.as_ref(),
        err
    )
}

/// 重试获取响应，当取得成功时返回
///
/// 你可能需要自行确认状态码是否成功
pub async fn retry_get(uri: impl AsRef<str>) -> DynResult<Response> {
    let res = retry_future(5, || get(uri.as_ref()), surf::Result::is_ok).await;
    let err = match res {
        Ok(Ok(body)) => return Ok(body),
        Ok(Err(e)) => {
            anyhow::anyhow!(
                "{}: {}",
                e,
                e.backtrace().map(|x| x.to_string()).unwrap_or_default()
            )
        }
        Err(e) => e,
    };
    anyhow::bail!(
        "轮询请求链接 {} 失败，请检查你的网络连接：{}",
        uri.as_ref(),
        err
    )
}

/// 生成简单的 GET 请求
pub fn get(uri: impl AsRef<str>) -> RequestBuilder {
    GLOBAL_CLIENT.get(uri)
}

/// 生成简单的 POST 请求
pub fn post(uri: impl AsRef<str>) -> RequestBuilder {
    GLOBAL_CLIENT.post(uri)
}

/// 针对 Mojang 验证 API 的响应结构
#[derive(Debug, Clone)]
pub enum RequestResult<T> {
    /// 返回的结构是成功的，此处为实际数据
    Ok(T),
    /// 返回的结构是错误的，此处为错误信息结构
    Err(crate::auth::structs::mojang::ErrorResponse),
}

/// 不会进行重试的 HTTP 请求模块
pub mod no_retry {
    use serde::{de::DeserializeOwned, Serialize};
    pub use surf::get;

    use super::RequestResult;
    use crate::prelude::DynResult;

    /// 获取 JSON 对象
    ///
    /// 返回的数据结构需要实现 [`serde::de::DeserializeOwned`]
    pub async fn get_data<D: DeserializeOwned>(uri: &str) -> DynResult<RequestResult<D>> {
        let result = surf::get(uri)
            .middleware(surf::middleware::Redirect::default())
            .recv_string()
            .await
            .map_err(|e| anyhow::anyhow!("无法接收来自 {} 的响应：{:?}", uri, e))?;
        if let Ok(result) = serde_json::from_str(&result) {
            Ok(RequestResult::Ok(result))
        } else {
            let result = serde_json::from_str(&result)?;
            Ok(RequestResult::Err(result))
        }
    }

    /// 带请求体去获取 JSON 对象
    ///
    /// 传入的请求体需要实现 [`serde::ser::Serialize`] 和 [`std::fmt::Debug`]
    ///
    /// 返回的数据结构需要实现 [`serde::de::DeserializeOwned`]
    pub async fn post_data<D: DeserializeOwned, S: Serialize + std::fmt::Debug>(
        uri: &str,
        body: &S,
    ) -> DynResult<RequestResult<D>> {
        let result = surf::post(uri)
            .header("Content-Type", "application/json; charset=utf-8")
            .body_json(body)
            .map_err(|e| anyhow::anyhow!("无法解析请求主体给 {}：{:?}", uri, e))?
            .recv_string()
            .await
            .map_err(|e| anyhow::anyhow!("无法接收来自 {} 的响应：{:?}", uri, e))?;
        if let Ok(result) = serde_json::from_str(&result) {
            Ok(RequestResult::Ok(result))
        } else {
            let result = serde_json::from_str(&result)?;
            Ok(RequestResult::Err(result))
        }
    }
}
