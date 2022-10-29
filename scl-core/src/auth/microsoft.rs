//! 用于处理微软登录的小型服务器

use serde::Deserialize;

use crate::{
    auth::{parse_head_skin, structs::AuthMethod},
    prelude::*,
};

/**
    Minecraft 官方启动器的微软登录链接

    通过捕获从此链接跳转过来的
    `https://login.live.com/oauth20_desktop.srf?code=[ANYCODE]&lc=1033`
    链接并传入 [`start_auth`] 来获取登录令牌
*/
pub const MICROSOFT_URL: &str = "https://login.live.com/oauth20_authorize.srf?client_id=00000000402b5328&response_type=code&scope=service%3A%3Auser.auth.xboxlive.com%3A%3AMBI_SSL&redirect_uri=https%3A%2F%2Flogin.live.com%2Foauth20_desktop.srf";

/**
  微软的登录令牌 API 接口
  用来请求或续期登录令牌
*/
pub const MICROSOFT_TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct OAuth20TokenResponse {
    // token_type: String,
    // expires_in: usize,
    // scope: String,
    pub error: String,
    pub access_token: String,
    pub refresh_token: String,
    // user_id: String,
    // foci: String,
}

#[derive(Debug, Clone, Deserialize)]
struct XBoxAuthResponse {
    #[serde(rename = "Token")]
    pub token: String,
    #[serde(rename = "DisplayClaims")]
    pub display_claims: XBoxAuthResponse1,
}

#[derive(Debug, Clone, Deserialize)]
struct XBoxAuthResponse1 {
    pub xui: Vec<XBoxAuthResponse2>,
}

#[derive(Debug, Clone, Deserialize)]
struct XBoxAuthResponse2 {
    pub uhs: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MinecraftStoreResponse {
    items: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct MinecraftXBoxLoginResponse {
    // pub username: String,
    pub access_token: String,
    // pub token_type: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct MinecraftXBoxProfileResponse {
    pub id: String,
    pub name: String,
    pub error: String,
    pub skins: Vec<MinecraftXBoxProfileResponse1>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct MinecraftXBoxProfileResponse1 {
    // pub id: String,
    pub state: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct XBoxPresenceRescord {
    // pub xuid: String,
}

/// 获取 XUID，用途不明，但是在新版本的 Minecraft 有发现需要使用这个 XUID 的地方
pub async fn get_xuid(userhash: &str, token: &str) -> DynResult<String> {
    let res = crate::http::get("https://userpresence.xboxlive.com/users/me?level=user")
        .header("Authorization", format!("XBL3.0 x={};{}", userhash, token))
        .header("x-xbl-contract-version", "3.2")
        .header("Accept", "application/json")
        .header("Accept-Language", "zh-CN")
        .header("Host", "userpresence.xboxlive.com")
        .recv_string()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(res)
}

/// 请求一个新令牌，或者续期一个令牌
///
/// 如果请求一个新令牌，则 credit 为从登录页面里传来的 code 请求字符串
///
/// 如果续期一个令牌，则 credit 为需要续期的旧令牌
pub async fn request_token(credit: &str, is_refresh: bool) -> DynResult<(String, String)> {
    let body = format!(
        "client_id=00000000402b5328&{}={}&grant_type={}&redirect_uri=https%3A%2F%2Flogin.live.com%2Foauth20_desktop.srf&scope=service%3A%3Auser.auth.xboxlive.com%3A%3AMBI_SSL",
        if is_refresh { "refresh_token" } else { "code" }, // Grant Tag
        credit,
        if is_refresh { "refresh_token" } else { "authorization_code" }, // Grant Type
    );
    let res: OAuth20TokenResponse = crate::http::post(MICROSOFT_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.as_bytes())
        .recv_json()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    anyhow::ensure!(
        res.error.is_empty(),
        "{}令牌失败: {}",
        if is_refresh { "刷新" } else { "请求" },
        res.error
    );
    Ok((res.access_token, res.refresh_token))
}

/// 根据微软登录传回的访问令牌 access_token 返回 user_hash 和 xsts_token
///
/// 传递给 [`get_mojang_access_token`] 进行下一步验证
pub async fn get_userhash_and_token(access_token: &str) -> DynResult<(String, String)> {
    // println!("Getting xbox auth body");
    let xbox_auth_body = format!("{{\"Properties\":{{\"AuthMethod\":\"RPS\",\"SiteName\":\"user.auth.xboxlive.com\",\"RpsTicket\":\"{}\"}},\"RelyingParty\":\"http://auth.xboxlive.com\",\"TokenType\":\"JWT\"}}", access_token);
    let xbox_auth_resp: XBoxAuthResponse =
        crate::http::post("https://user.auth.xboxlive.com/user/authenticate")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(xbox_auth_body.as_bytes())
            .await
            .map_err(|e| anyhow::anyhow!(e))?
            .body_json()
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
    let token = xbox_auth_resp.token.to_owned();
    if let Some(uhs) = xbox_auth_resp.display_claims.xui.first() {
        let uhs = uhs.uhs.to_owned();
        let xsts_body = format!("{{\"Properties\":{{\"SandboxId\":\"RETAIL\",\"UserTokens\":[\"{}\"]}},\"RelyingParty\":\"rp://api.minecraftservices.com/\",\"TokenType\":\"JWT\"}}", token);
        println!("Getting xbox xsts token");
        let xsts_resp: XBoxAuthResponse =
            crate::http::post("https://xsts.auth.xboxlive.com/xsts/authorize")
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .body(xsts_body.as_bytes())
                .await
                .map_err(|e| anyhow::anyhow!(e))?
                .body_json()
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
        let xsts_token = xsts_resp.token;
        Ok((uhs, xsts_token))
    } else {
        anyhow::bail!("获取 UserHash 失败")
    }
}

/// 通过 [`get_userhash_and_token`] 返回的 `userhash` 和 `xsts_token` 获取 Mojang 的访问令牌
///
/// 在拥有 Minecraft 游戏的情况下，此令牌可用于正版启动游戏
pub async fn get_mojang_access_token(uhs: &str, xsts_token: &str) -> DynResult<String> {
    if !uhs.is_empty() && !xsts_token.is_empty() {
        // println!("Getting mojang access token");
        let minecraft_xbox_body =
            format!("{{\"identityToken\":\"XBL3.0 x={};{}\"}}", uhs, xsts_token);
        let minecraft_xbox_resp: MinecraftXBoxLoginResponse =
            crate::http::post("https://api.minecraftservices.com/authentication/login_with_xbox")
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .body(minecraft_xbox_body.as_bytes())
                .await
                .map_err(|e| anyhow::anyhow!(e))?
                .body_json()
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
        // println!("Getting minecraft access token");
        let access_token = minecraft_xbox_resp.access_token;
        Ok(access_token)
    } else {
        Ok("".into())
    }
}

/// 刷新登录令牌，如刷新成功则可将更新后的用户继续用于正版启动
pub async fn refresh_auth(method: &mut AuthMethod) -> DynResult {
    match method {
        AuthMethod::Microsoft {
            access_token,
            refresh_token,
            ..
        } => {
            let (new_access_token, new_refresh_token) =
                request_token(refresh_token.as_str(), true).await?;
            let (uhs, xsts_token) = get_userhash_and_token(&new_access_token).await?;
            let new_access_token = get_mojang_access_token(&uhs, &xsts_token).await?;
            anyhow::ensure!(
                !new_access_token.is_empty(),
                "刷新令牌失败: {}",
                new_access_token
            );
            *access_token = new_access_token.into();
            *refresh_token = new_refresh_token.into();
        }
        _ => {
            anyhow::bail!("不支持的方法");
        }
    }
    Ok(())
}

/// 执行微软登录，需要形如 `https://login.live.com/oauth20_desktop.srf?code=[ANYCODE]&lc=1033` 的链接作为参数
pub async fn start_auth(_ctx: Option<impl Reporter>, url: &str) -> DynResult<AuthMethod> {
    let url = url.parse::<url::Url>()?;
    if let Some((_, code)) = url.query_pairs().find(|a| a.0 == "code") {
        let (access_token, refresh_token) = request_token(&code, false).await?;
        let (uhs, xsts_token) = get_userhash_and_token(&access_token).await?;
        let xuid = get_xuid(&uhs, &xsts_token).await?;
        let access_token = get_mojang_access_token(&uhs, &xsts_token).await?;
        if access_token.is_empty() {
            return Err(anyhow::anyhow!("获取令牌失败"));
        } else {
            let mcstore_resp: MinecraftStoreResponse =
                crate::http::get("https://api.minecraftservices.com/entitlements/mcstore")
                    .header("Authorization", &format!("Bearer {}", &access_token))
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?
                    .body_json()
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
            if mcstore_resp.items.is_empty() {
                anyhow::bail!(
                    "没有在已购项目中找到 Minecraft！请检查你的账户是否已购买 Minecraft！"
                );
            }
            let profile_resp: MinecraftXBoxProfileResponse =
                crate::http::get("https://api.minecraftservices.com/minecraft/profile")
                    .header("Authorization", &format!("Bearer {}", &access_token))
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?
                    .body_json()
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
            if profile_resp.error.is_empty() {
                if let Some(skin) = profile_resp.skins.iter().find(|a| a.state == "ACTIVE") {
                    let skin_data = crate::http::get(&skin.url)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?
                        .body_bytes()
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                    let (head_skin, hat_skin) = parse_head_skin(skin_data)?;
                    println!("Successfully authed!");
                    return Ok(AuthMethod::Microsoft {
                        access_token: access_token.into(),
                        refresh_token: refresh_token.into(),
                        xuid,
                        head_skin,
                        hat_skin,
                        player_name: profile_resp.name,
                        uuid: profile_resp.id,
                    });
                }
            } else {
                anyhow::bail!(
                    "没有在账户中找到 Minecraft 账户信息！请检查你的账户是否已购买 Minecraft！"
                );
            }
        }
    }
    anyhow::bail!("链接不合法");
}
