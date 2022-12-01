//! HTTP 包装，虽然是内部使用但是你也可以使用这个来做点 HTTP 请求什么的
//!
//! 或者在二次开发的时候更换成你喜欢的版本

use std::{sync::Arc, time::Duration};

use alhc::*;
use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;

use crate::prelude::*;

static GLOBAL_CLIENT: Lazy<Arc<Client>> = Lazy::new(|| {
    let mut client = ClientBuilder::default().build();
    client.set_timeout(Duration::from_millis(10 * 1000));
    Arc::new(client)
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
        let res = get(uri)?.await.map(|x| x.recv());
        match res {
            Ok(res) => {
                let res = res.await;
                match res {
                    Ok(res) => {
                        if res.status_code() == 200 {
                            let tmp_dest_path = format!("{}.tmp", dest_path);
                            let _tmp_file = inner_future::fs::OpenOptions::new()
                                .create(true)
                                .write(true)
                                .truncate(true)
                                .open(&tmp_dest_path)
                                .await?;
                            if inner_future::fs::write(&tmp_dest_path, res.data())
                                .await
                                .is_ok()
                            {
                                inner_future::fs::rename(tmp_dest_path, dest_path).await?;
                                return Ok(());
                            }
                        } else {
                            println!("Error {:?} 状态码错误 {}", uri, res.status_code());
                        }
                    }
                    Err(e) => {
                        println!("Error {:?} {}", uri, e)
                    }
                }
            }
            Err(e) => {
                println!("Error {:?} {}", uri, e)
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
    match get(uri)?.await?.recv_json().await {
        Ok(data) => Ok(data),
        Err(e) => anyhow::bail!("{:?}", e),
    }
}

/// 重试获取数据
pub async fn retry_get_bytes(uri: impl AsRef<str>) -> DynResult<Vec<u8>> {
    match get(uri)?.await?.recv_bytes().await {
        Ok(data) => Ok(data),
        Err(e) => anyhow::bail!("{:?}", e),
    }
}

/// 重试获取字符串
pub async fn retry_get_string(uri: impl AsRef<str>) -> DynResult<String> {
    match get(uri)?.await?.recv_string().await {
        Ok(data) => Ok(data),
        Err(e) => anyhow::bail!("{:?}", e),
    }
}

/// 重试获取响应，当取得成功时返回
///
/// 你可能需要自行确认状态码是否成功
pub async fn retry_get(uri: impl AsRef<str>) -> DynResult<Response> {
    match get(uri)?.await {
        Ok(data) => Ok(data),
        Err(e) => anyhow::bail!("{:?}", e),
    }
}

/// 生成简单的 GET 请求
pub fn get(uri: impl AsRef<str>) -> DynResult<Request> {
    if let Ok(r) = GLOBAL_CLIENT.get(uri.as_ref()) {
        Ok(r)
    } else {
        anyhow::bail!("无法创建发送到 {} 的 GET 请求", uri.as_ref())
    }
}

/// 生成简单的 POST 请求
pub fn post(uri: impl AsRef<str>) -> DynResult<Request> {
    if let Ok(r) = GLOBAL_CLIENT.post(uri.as_ref()) {
        Ok(r)
    } else {
        anyhow::bail!("无法创建发送到 {} 的 POST 请求", uri.as_ref())
    }
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
    use anyhow::Context;
    use serde::{de::DeserializeOwned, Serialize};

    use super::RequestResult;
    use crate::prelude::DynResult;

    /// 获取 JSON 对象
    ///
    /// 返回的数据结构需要实现 [`serde::de::DeserializeOwned`]
    pub async fn get_data<D: DeserializeOwned>(uri: &str) -> DynResult<RequestResult<D>> {
        let result = super::get(uri)?
            .await?
            .recv_string()
            .await
            .with_context(|| anyhow::anyhow!("无法接收来自 {} 的响应", uri))?;
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
        let result = super::post(uri)?
            .header("Content-Type", "application/json; charset=utf-8")
            .body_string(serde_json::to_string(body)?)
            .await
            .with_context(|| anyhow::anyhow!("无法解析请求主体给 {}", uri))?
            .recv_string()
            .await
            .with_context(|| anyhow::anyhow!("无法接收来自 {} 的响应", uri))?;
        if let Ok(result) = serde_json::from_str(&result) {
            Ok(RequestResult::Ok(result))
        } else {
            let result = serde_json::from_str(&result)?;
            Ok(RequestResult::Err(result))
        }
    }
}
