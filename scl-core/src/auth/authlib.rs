//! 用于 authlib-injector 第三方登录的登录逻辑

use std::str::FromStr;

use anyhow::Context;
use base64::prelude::*;
use surf::StatusCode;

use crate::{
    auth::structs::{mojang::*, AuthMethod},
    http::RequestResult,
    password::Password,
    prelude::*,
};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ServerMetaLinks {
    pub homepage: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
struct ServerMeta {
    pub server_name: String,
    pub links: Option<ServerMetaLinks>,
}

#[derive(Debug, Default, Deserialize)]
struct APIMetaData {
    pub meta: ServerMeta,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RefreshBody {
    pub access_token: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub client_token: String,
    pub request_user: bool,
    pub selected_profile: Option<AvaliableProfile>,
}

async fn get_head_skin(api_location: &str, uuid: &str) -> DynResult<(Vec<u8>, Vec<u8>)> {
    let uri = format!("{api_location}sessionserver/session/minecraft/profile/{uuid}");
    let result: ProfileResponse = crate::http::no_retry::get(&uri)
        .await
        .map_err(|e| anyhow::anyhow!("发送获取皮肤请求到 {} 时发生错误：{:?}", uri, e))?
        .body_json()
        .await
        .map_err(|e| anyhow::anyhow!("接收获取皮肤响应到 {} 时发生错误：{:?}", uri, e))?;
    if let Some(prop) = result
        .properties
        .iter()
        .find(|a| a.name.as_str() == "textures")
    {
        let texture_raw = &prop.value;
        let texture_raw = BASE64_STANDARD.decode(texture_raw)?;
        let texture_data: ProfileTexture = serde_json::from_slice(&texture_raw)?;
        if let Some(textures) = texture_data.textures {
            if let Some(skin) = textures.skin {
                let skin_url = skin.url;
                crate::auth::parse_head_skin(
                    crate::http::no_retry::get(skin_url)
                        .recv_bytes()
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?,
                )
            } else {
                Ok(Default::default())
            }
        } else {
            Ok(Default::default())
        }
    } else {
        Ok(Default::default())
    }
}

/// 根据初次登陆/二次验证取得的用户令牌，刷新验证出可供正常游戏的登录令牌
///
/// 详情参考[启动器技术规范](https://github.com/yushijinhun/authlib-injector/wiki/Yggdrasil-%E6%9C%8D%E5%8A%A1%E7%AB%AF%E6%8A%80%E6%9C%AF%E8%A7%84%E8%8C%83#%E5%88%B7%E6%96%B0)
pub async fn refresh_token(
    auth_method: AuthMethod,
    client_token: &str,
    provide_selected_profile: bool,
) -> DynResult<AuthMethod> {
    if let AuthMethod::AuthlibInjector {
        api_location,
        server_name,
        server_homepage,
        server_meta,
        access_token,
        uuid,
        player_name,
        ..
    } = auth_method
    {
        let res: RequestResult<AuthenticateResponse> = dbg!(crate::http::no_retry::post_data(
            dbg!(&format!("{api_location}authserver/refresh")),
            dbg!(&RefreshBody {
                access_token: access_token.to_owned_string(),
                client_token: client_token.to_owned(),
                request_user: provide_selected_profile,
                selected_profile: if provide_selected_profile {
                    Some(AvaliableProfile {
                        name: player_name.to_owned(),
                        id: uuid.to_owned(),
                    })
                } else {
                    None
                },
            }),
        )
        .await
        .context("无法请求刷新令牌接口")?);

        match res {
            RequestResult::Ok(res) => {
                let selected_profile = res.selected_profile.unwrap_or_else(|| AvaliableProfile {
                    name: player_name.to_owned(),
                    id: uuid.to_owned(),
                });

                let (head_skin, hat_skin) =
                    get_head_skin(&api_location, &selected_profile.id).await?;

                let refreshed_method = AuthMethod::AuthlibInjector {
                    api_location: api_location.to_owned(),
                    server_name: server_name.to_owned(),
                    server_homepage,
                    server_meta,
                    access_token: res.access_token.into(),
                    uuid: selected_profile.id,
                    player_name: selected_profile.name,
                    head_skin,
                    hat_skin,
                };

                Ok(refreshed_method)
            }
            RequestResult::Err(a) => {
                if a.error_message.is_empty() {
                    match a.error.as_str() {
                        "ForbiddenOperationException" => anyhow::bail!("未授权的访问"),
                        "IllegalArgumentException" => anyhow::bail!("非法令牌绑定"),
                        _ => anyhow::bail!("未知原因：{}", a.error),
                    }
                } else {
                    anyhow::bail!("{}：{}", a.error, a.error_message)
                }
            }
        }
    } else {
        anyhow::bail!("此函数只支持 Authlib Injector 第三方登录")
    }
}

/// 使用指定的 Authlib 服务器地址和对应的账户密码开始进行 Authlib 第三方登录验证
///
/// 根据[启动器技术规范](https://github.com/yushijinhun/authlib-injector/wiki/%E5%90%AF%E5%8A%A8%E5%99%A8%E6%8A%80%E6%9C%AF%E8%A7%84%E8%8C%83)编写
///
/// 如果验证成功，则会返回这个账户旗下所有角色。
/// 如果用户名和角色名称一致，则只会返回那个角色。
pub async fn start_auth(
    _ctx: Option<impl Reporter>,
    authlib_host: &str,
    username: String,
    password: Password,
    client_token: &str,
) -> DynResult<Vec<AuthMethod>> {
    // 找到 API 地址，使用 ALI
    let api_location = {
        let a = crate::http::get(authlib_host)
            .await
            .map_err(|_| anyhow::anyhow!("无法请求 Authlib API 服务器：{}", authlib_host))?;
        if let Some(h) = a.header("X-Authlib-Injector-API-Location") {
            h.last().to_string()
        } else {
            authlib_host.to_owned()
        }
    };

    // 处理链接格式
    let api_location = {
        if api_location.starts_with("http") {
            url::Url::parse(&api_location)?
        } else {
            url::Url::parse(authlib_host)?.join(&api_location)?
        }
    };

    let api_location = api_location.to_string();
    let api_location = if api_location.ends_with('/') {
        api_location
    } else {
        format!("{api_location}/")
    };
    let api_location_url = url::Url::from_str(&api_location)?;

    let meta_res: RequestResult<APIMetaData> = crate::http::no_retry::get_data(&api_location)
        .await
        .map_err(|e| anyhow::anyhow!("无法接收 Authlib 服务器元数据响应：{:?}", e))?;

    let (server_name, server_homepage) = if let RequestResult::Ok(meta) = meta_res {
        let mut result = (String::new(), String::new());
        if meta.meta.server_name.is_empty() {
            result.0 = url::Url::from_str(&api_location)?
                .host()
                .ok_or_else(|| anyhow::anyhow!("无法取得 Authlib 服务器接口的 Host 部分"))?
                .to_string();
        } else {
            result.0 = meta.meta.server_name;
        }
        if let Some(server_homepage) = meta.meta.links.map(|a| a.homepage) {
            result.1 = server_homepage;
        } else {
            result.1 = api_location_url.origin().ascii_serialization();
        }
        result
    } else {
        (
            api_location_url
                .host()
                .ok_or_else(|| anyhow::anyhow!("无法取得 Authlib 服务器接口的 Host 部分"))?
                .to_string(),
            api_location_url.origin().ascii_serialization(),
        )
    };

    let server_meta = crate::http::no_retry::get(&api_location)
        .recv_bytes()
        .await
        .map_err(|e| anyhow::anyhow!("无法接收登录接口元数据：{:?}", e))?;
    let server_meta = BASE64_STANDARD.encode(server_meta);

    // 登录链接
    let auth_url = format!("{api_location}authserver/authenticate");
    let auth_body = AuthenticateBody {
        username: username.to_owned(),
        password: password.take_string(),
        client_token: client_token.to_owned(),
        ..Default::default()
    };
    let resp: RequestResult<AuthenticateResponse> =
        crate::http::no_retry::post_data(&auth_url, &auth_body)
            .await
            .map_err(|e| anyhow::anyhow!("无法解析登录接口回调：{} {:?}", auth_url, e))?;

    match resp {
        RequestResult::Ok(a) => {
            if let Some(selected_profile) = a.selected_profile {
                if selected_profile.name == username {
                    let (head_skin, hat_skin) =
                        get_head_skin(&api_location, &selected_profile.id).await?;
                    return Ok(vec![AuthMethod::AuthlibInjector {
                        api_location,
                        server_name,
                        server_homepage,
                        server_meta,
                        access_token: a.access_token.into(),
                        uuid: selected_profile.id,
                        player_name: selected_profile.name,
                        head_skin,
                        hat_skin,
                    }]);
                }
            }
            if !a.available_profiles.is_empty() {
                if let Some(profile) = a.available_profiles.iter().find(|x| x.name == username) {
                    let (head_skin, hat_skin) = get_head_skin(&api_location, &profile.id).await?;
                    return Ok(vec![AuthMethod::AuthlibInjector {
                        api_location,
                        server_name,
                        server_homepage,
                        server_meta,
                        access_token: a.access_token.into(),
                        uuid: profile.id.to_owned(),
                        player_name: profile.name.to_owned(),
                        head_skin,
                        hat_skin,
                    }]);
                }

                let skins_threads =
                    futures::future::join_all(a.available_profiles.into_iter().map(|x| async {
                        let (head_skin, hat_skin) = get_head_skin(&api_location, &x.id)
                            .await
                            .unwrap_or_else(|_| (vec![0; 2 * 4 * 64], vec![0; 2 * 4 * 64]));
                        AuthMethod::AuthlibInjector {
                            api_location: api_location.to_owned(),
                            server_name: server_name.to_owned(),
                            server_homepage: server_homepage.to_owned(),
                            server_meta: server_meta.to_owned(),
                            access_token: a.access_token.to_owned().into(),
                            uuid: x.id,
                            player_name: x.name,
                            head_skin,
                            hat_skin,
                        }
                    }))
                    .await;
                Ok(skins_threads)
            } else {
                anyhow::bail!("该账户没有可用的角色！")
            }
        }
        RequestResult::Err(a) => {
            if a.error_message.is_empty() {
                match a.error.as_str() {
                    "ForbiddenOperationException" => anyhow::bail!("未授权的访问"),
                    "IllegalArgumentException" => anyhow::bail!("非法令牌绑定"),
                    _ => anyhow::bail!("未知原因：{}", a.error),
                }
            } else {
                anyhow::bail!("{}：{}", a.error, a.error_message)
            }
        }
    }
}

/// 验证对应的访问令牌和当前启动器令牌是否可以用于现在进行游戏
pub async fn validate(
    api_location: &str,
    access_token: &str,
    client_token: &str,
) -> DynResult<bool> {
    let post_url = url::Url::parse(api_location)?.join("authserver/validate")?;
    let resp = crate::http::post(post_url)
        .body_json(&ValidateResponse {
            access_token: access_token.to_owned(),
            client_token: client_token.to_owned(),
        })
        .map_err(|_| anyhow::anyhow!("无法序列化请求"))?
        .await
        .map_err(|_| anyhow::anyhow!("无法请求 Authlib API 服务器：{}", api_location))?;
    Ok(resp.status() == StatusCode::NoContent)
}
