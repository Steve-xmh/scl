//! 微软登录模块，通过设备码方式获取玩家的 Microsoft 账户令牌，进而获取 Minecraft 用户令牌

use std::fmt::Display;

use alhc::prelude::*;
use anyhow::Context;
use serde::Deserialize;

use super::structs::AuthMethod;
use crate::{password::Password, prelude::*};
pub mod leagcy;
use leagcy::*;

/// 使用设备流方式验证的微软账户验证对象
///
/// 使用这个对象前，你需要通过 Azure Active Directory
/// 注册一个应用，并将其客户端 ID 提供至此使用。
///
/// 具体请查阅 <https://wiki.vg/Microsoft_Authentication_Scheme>
pub struct MicrosoftOAuth<T> {
    client_id: T,
}

impl<T: Display> MicrosoftOAuth<T> {
    /// 通过客户端 ID 创建一个新的验证对象
    pub const fn new(client_id: T) -> Self {
        Self { client_id }
    }

    /// 获取一个设备码，将其展示给用户以完成浏览器验证
    pub async fn get_devicecode(&self) -> DynResult<DeviceCodeResponse> {
        let res: DeviceCodeResponse = crate::http::post(
            "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode?mkt=zh-CN",
        )?
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body_string(format!(
            "client_id={}&scope=XboxLive.signin%20offline_access",
            self.client_id
        ))
        .await?
        .recv_json()
        .await
        .context("请求设备码时发生错误")?;

        Ok(res)
    }

    /// 获取/验证设备码的验证情况
    pub async fn verify_device_code(&self, device_code: &str) -> DynResult<TokenResponse> {
        let res: TokenResponse =
            crate::http::post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")?
                .body_string(format!(
            "grant_type=urn:ietf:params:oauth:grant-type:device_code&client_id={}&device_code={}",
            self.client_id, device_code,
        ))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .await?
                .recv_json::<TokenResponse>()
                .await
                .context("请求设备码验证情况时发生错误")?;

        Ok(res)
    }

    /// 重新刷新令牌，获取新的访问令牌和刷新令牌
    async fn refresh_token(&self, refresh_token: &str) -> DynResult<TokenResponse> {
        let res: TokenResponse =
            crate::http::post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")?
                .body_string(format!(
                    "grant_type=refresh_token&client_id={}&refresh_token={}",
                    self.client_id, refresh_token,
                ))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .await?
                .recv_json::<TokenResponse>()
                .await
                .context("请求刷新令牌时发生错误")?;

        Ok(res)
    }

    async fn auth_xbox_live(&self, access_token: &str) -> DynResult<(String, String)> {
        tracing::debug!("正在验证 Xbox Live 账户");
        let xbox_auth_body = format!(
            "{\
            {\
                \"Properties\":{\
                    {\
                        \"AuthMethod\":\"RPS\",\
                        \"SiteName\":\"user.auth.xboxlive.com\",\
                        \"RpsTicket\":\"d={access_token}\"\
                    }\
                },\
                \"RelyingParty\":\"http://auth.xboxlive.com\",\
                \"TokenType\":\"JWT\"\
            }\
        }"
        );
        let xbox_auth_resp: XBoxAuthResponse =
            crate::http::post("https://user.auth.xboxlive.com/user/authenticate")?
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .body_string(xbox_auth_body)
                .await?
                .recv_json()
                .await
                .context("验证 Xbox Live 账户失败")?;
        let token = xbox_auth_resp.token.to_owned();
        if let Some(uhs) = xbox_auth_resp.display_claims.xui.first() {
            let uhs = uhs.uhs.to_owned();
            let xsts_body = format!(
                "{\
                {\
                    \"Properties\":{\
                        {\
                            \"SandboxId\":\"RETAIL\",\
                            \"UserTokens\":[\"{token}\"]\
                        }\
                    },\
                    \"RelyingParty\":\"rp://api.minecraftservices.com/\",\
                    \"TokenType\":\"JWT\"\
                }\
            }"
            );
            tracing::debug!("正在获取 XSTS");
            let xsts_resp: XBoxAuthResponse =
                crate::http::post("https://xsts.auth.xboxlive.com/xsts/authorize")?
                    .header("Content-Type", "application/json")
                    .header("Accept", "application/json")
                    .body_string(xsts_body)
                    .await?
                    .recv_json()
                    .await
                    .context("获取 XSTS 账户失败")?;
            let xsts_token = xsts_resp.token;
            Ok((uhs, xsts_token))
        } else {
            anyhow::bail!("获取 UserHash 失败")
        }
    }

    /// 通过设备码验证获取到的 Microsoft 访问令牌获取 Minecraft 账户
    pub async fn start_auth(
        &self,
        access_token: &str,
        refresh_token: &str,
    ) -> DynResult<AuthMethod> {
        let (uhs, xsts_token) = self.auth_xbox_live(access_token).await?;

        tracing::debug!("正在获取 XUID");
        let xuid = leagcy::get_xuid(&uhs, &xsts_token).await?;

        tracing::debug!("正在获取 Mojang 访问令牌");
        let access_token = leagcy::get_mojang_access_token(&uhs, &xsts_token).await?;

        if access_token.is_empty() {
            anyhow::bail!("获取令牌失败")
        } else {
            tracing::debug!("正在检查是否拥有 Minecraft");
            let mcstore_resp: MinecraftStoreResponse =
                crate::http::get("https://api.minecraftservices.com/entitlements/mcstore")?
                    .header(
                        "Authorization",
                        &format!("Bearer {}", &access_token.as_string()),
                    )
                    .await?
                    .recv_json()
                    .await?;
            if mcstore_resp.items.is_empty() {
                anyhow::bail!(
                    "没有在已购项目中找到 Minecraft！请检查你的账户是否已购买 Minecraft！"
                );
            }
            tracing::debug!("正在获取 Minecraft 账户信息");
            let profile_resp: MinecraftXBoxProfileResponse =
                crate::http::get("https://api.minecraftservices.com/minecraft/profile")?
                    .header(
                        "Authorization",
                        &format!("Bearer {}", &access_token.as_string()),
                    )
                    .await?
                    .recv_json()
                    .await?;
            if profile_resp.error.is_empty() {
                if let Some(skin) = profile_resp.skins.iter().find(|a| a.state == "ACTIVE") {
                    tracing::debug!("正在解析皮肤: {}", skin.url);
                    let skin_data = crate::http::get(&skin.url)?
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?
                        .recv_bytes()
                        .await?;
                    let (head_skin, hat_skin) =
                        crate::auth::parse_head_skin(skin_data).context("解析皮肤数据失败")?;
                    tracing::debug!("微软账户验证成功！");
                    Ok(AuthMethod::Microsoft {
                        access_token,
                        refresh_token: refresh_token.to_string().into(),
                        xuid,
                        head_skin,
                        hat_skin,
                        player_name: profile_resp.name,
                        uuid: profile_resp.id,
                    })
                } else {
                    anyhow::bail!("皮肤获取失败！");
                }
            } else {
                anyhow::bail!(
                    "没有在账户中找到 Minecraft 账户信息！请检查你的账户是否已购买 Minecraft！"
                );
            }
        }
    }

    /// 刷新登录令牌，如刷新成功则可将更新后的用户继续用于正版启动
    pub async fn refresh_auth(&self, method: &mut AuthMethod) -> DynResult {
        if let AuthMethod::Microsoft {
            access_token,
            refresh_token,
            ..
        } = method
        {
            tracing::debug!("正在刷新令牌");
            let new_token = self.refresh_token(refresh_token.as_str()).await?;

            *refresh_token = new_token.refresh_token.into();

            let (uhs, xsts_token) = self.auth_xbox_live(&new_token.access_token).await?;

            tracing::debug!("正在获取 Mojang 访问令牌");
            let new_access_token = leagcy::get_mojang_access_token(&uhs, &xsts_token).await?;

            anyhow::ensure!(
                !new_access_token.is_empty(),
                "刷新令牌失败: {}",
                new_access_token
            );

            *access_token = new_access_token;
            Ok(())
        } else {
            anyhow::bail!("不支持的方法");
        }
    }
}

/// 请求设备码的响应结构
///
/// 关于此结构的详情可以查阅 [Microsoft 标识平台和 OAuth 2.0 设备权限授予流 - 设备授权请求](https://learn.microsoft.com/zh-cn/azure/active-directory/develop/v2-oauth2-device-code#device-authorization-request)
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct DeviceCodeResponse {
    /// 一个长字符串，用于验证客户端与授权服务器之间的会话。 客户端使用此参数从授权服务器请求访问令牌。
    pub device_code: String,
    /// 向用户显示的短字符串，用于标识辅助设备上的会话。
    pub user_code: String,
    /// 用户在登录时应使用 `user_code` 转到的 URI。
    pub verification_uri: String,
    /// `device_code` 和 `user_code` 过期之前的秒数。
    pub expires_in: usize,
    /// 在发出下一个轮询请求之前客户端应等待的秒数。
    pub interval: usize,
    /// 用户可读的字符串，包含面向用户的说明。
    /// 可以通过在请求中包含 `?mkt=xx-XX` 格式的查询参数并填充相应的语言区域性代码，将此字符串本地化。
    pub message: String,
    /// 错误信息，如果请求正常则此处是空字符串
    pub error: String,
}

/// 请求设备码身份验证的响应结构
///
/// 关于此结构的详情可以查阅 [Microsoft 标识平台和 OAuth 2.0 设备权限授予流 - 成功的身份验证响应](https://learn.microsoft.com/zh-cn/azure/active-directory/develop/v2-oauth2-device-code#successful-authentication-response)
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct TokenResponse {
    /// 总是为 `Bearer`。
    pub token_type: String,
    /// 如果返回访问令牌，则会列出该访问令牌的有效范围。
    pub scope: String,
    /// 包含的访问令牌有效的秒数。
    pub expires_in: usize,
    /// 针对请求的范围颁发。
    pub access_token: Password,
    /// 如果原始 `scope` 参数包含 `openid` 范围，则颁发。
    pub id_token: String,
    /// 如果原始 `scope` 参数包含 `offline_access`，则颁发。
    pub refresh_token: String,
    /// 错误信息，如果请求正常则此处是空字符串
    pub error: String,
}
