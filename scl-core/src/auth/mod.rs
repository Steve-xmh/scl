/*!
    此模块为登录验证模块，开发者可以调用此处的函数获取不同种类账户验证之后的登录令牌。
*/

use std::io::Cursor;

use alhc::prelude::*;
use anyhow::Context;
use base64::prelude::*;
use image::{GenericImageView, Pixel};
use structs::mojang::{ProfileResponse, ProfileTexture};

use self::structs::AuthMethod;
use crate::{
    http::{no_retry::*, RequestResult},
    password::Password,
    prelude::*,
};

pub mod authlib;
pub mod microsoft;
pub mod structs;

/// 根据玩家名称生成一个固定的离线 UUID
///
/// 返回值可以通过传入 `format!("{:x}", uuid)` 来转换为十六进制字符串形式
///
/// 代码参考：
/// ```rust
/// # use scl_core::auth::generate_offline_uuid;
/// # fn main() {
/// assert_eq!(format!("{:x}", generate_offline_uuid("Steve")), "5627dd98e6be3c21b8a8e92344183641");
/// assert_eq!(format!("{:x}", generate_offline_uuid("Alex")), "36532b5ec4423dbba24cc7e55d0f979a");
/// # }
/// ```
/// 生成方式参考： <https://github.com/PrismarineJS/node-minecraft-protocol/blob/21240f8ab2fd41c76f50b64e3b3a945f50b25b5e/src/datatypes/uuid.js#L14>
pub fn generate_offline_uuid(player_name: &str) -> md5::Digest {
    let mut ctx = md5::Context::new();
    ctx.consume("OfflinePlayer:");
    ctx.consume(player_name);
    let mut result = ctx.compute().0;

    result[6] = (result[6] & 0x0f) | 0x30;
    result[8] = (result[8] & 0x3f) | 0x80;

    md5::Digest(result)
}

/// 提取一个皮肤位图的正面头部部分，用于 GUI 展示头像
///
/// 传入的皮肤大小必须是 32x64 或 64x64
pub fn parse_head_skin(result: Vec<u8>) -> DynResult<(Vec<u8>, Vec<u8>)> {
    let cursor = Cursor::new(result);
    let mut skin_data = Vec::with_capacity(2 * 4 * 64);
    let mut skin_hat_data = Vec::with_capacity(2 * 4 * 64);
    let skin = image::load(cursor, image::ImageFormat::Png)?;
    for y in 8..16 {
        for x in 8..16 {
            let pixel = skin.get_pixel(x, y).to_rgba();
            skin_data.push(pixel.0[0]);
            skin_data.push(pixel.0[1]);
            skin_data.push(pixel.0[2]);
            skin_data.push(pixel.0[3]);
        }
    }
    for y in 8..16 {
        for x in 40..48 {
            let pixel = skin.get_pixel(x, y).to_rgba();
            skin_hat_data.push(pixel.0[0]);
            skin_hat_data.push(pixel.0[1]);
            skin_hat_data.push(pixel.0[2]);
            skin_hat_data.push(pixel.0[3]);
        }
    }
    Ok((skin_data, skin_hat_data))
}

async fn get_head_skin(uuid: &str) -> DynResult<(Vec<u8>, Vec<u8>)> {
    // https://sessionserver.mojang.com/session/minecraft/profile/{uuid}
    let uri = format!("https://sessionserver.mojang.com/session/minecraft/profile/{uuid}");
    let result: ProfileResponse = crate::http::get(uri)?.await?.recv_json().await?;
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
                parse_head_skin(crate::http::get(skin_url)?.await?.recv_bytes().await?)
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

/// 进行 Mojang 正版验证
///
/// **此验证方式已经弃用**，请开发者建议用户迁移到 Microsoft 账户后使用 [`crate::auth::microsoft::start_auth`] 进行 Microsoft 正版验证
pub async fn auth_mojang(
    _ctx: Option<impl Reporter>,
    username: &str,
    password: &Password,
    client_token: &str,
) -> DynResult<AuthMethod> {
    // https://authserver.mojang.com/authenticate
    let body = structs::mojang::AuthenticateBody {
        username: username.into(),
        password: password.to_owned(),
        client_token: client_token.into(),
        ..Default::default()
    };
    let result: RequestResult<structs::mojang::AuthenticateResponse> =
        post_data("https://authserver.mojang.com/authenticate", &body).await?;
    match result {
        RequestResult::Ok(a) => {
            let selected_profile = if let Some(selected_profile) = a.selected_profile {
                selected_profile
            } else if let Some(profile) = a.available_profiles.into_iter().next() {
                profile // TODO: 选择所需要添加的多角色
            } else {
                anyhow::bail!("该账户没有可用的角色！")
            };
            let (head_skin, hat_skin) = get_head_skin(&selected_profile.id).await?;
            Ok(AuthMethod::Mojang {
                access_token: a.access_token,
                uuid: selected_profile.id,
                player_name: selected_profile.name,
                head_skin,
                hat_skin,
            })
        }
        RequestResult::Err(_) => Ok(AuthMethod::Offline {
            player_name: "".into(),
            uuid: "".into(),
        }),
    }
}

/// 刷新/续期访问令牌
pub async fn refresh_auth(am: &mut AuthMethod, client_token: &str) -> DynResult<bool> {
    if let &mut AuthMethod::Microsoft { .. } = am {
        return Ok(microsoft::leagcy::refresh_auth(am).await.is_ok());
    }
    match am {
        AuthMethod::Mojang { access_token, .. } => {
            let body = structs::mojang::ValidateResponse {
                access_token: access_token.to_owned(),
                client_token: client_token.to_owned(),
            };
            let result = crate::http::post("https://authserver.mojang.com/validate")?
                .body_bytes(serde_json::to_vec(&body)?)
                .header("Content-Type", "application/json")
                .await
                .context("发送用户信息请求失败，可能是网络问题")?
                .recv()
                .await?;
            if result.status_code() >= 200 && result.status_code() < 300 {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        AuthMethod::Microsoft { access_token, .. } => {
            // TODO: 增加正确的检测方式
            let profile_resp =
                crate::http::get("https://api.minecraftservices.com/minecraft/profile")?
                    .header("Authorization", &format!("Bearer {}", &access_token))
                    .await
                    .context("发送用户信息请求失败，可能是网络问题")?
                    .recv()
                    .await?;
            if profile_resp.status_code() >= 200 && profile_resp.status_code() < 300 {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        AuthMethod::AuthlibInjector { .. } => {
            if let Ok(new_am) =
                crate::auth::authlib::refresh_token(am.to_owned(), client_token, false).await
            {
                *am = new_am;
                Ok(true)
            } else {
                Ok(false)
            }
        }
        _ => Ok(true),
    }
}
