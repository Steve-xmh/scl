//! 登录验证数据结构
use serde::{Deserialize, Serialize};

use crate::password::Password;

/**
   账户类型枚举，需要提供一个账户种类方可启动游戏
*/
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    /// 离线账户登录
    Offline {
        /// 离线玩家的名称
        player_name: String,
        /// 离线玩家的统一标识，如果玩家是从其它启动器迁移到使用本启动模块的启动器的，需要提供这个以确保存档物品信息能够正确读取
        uuid: String,
    },
    /// Mojang (Yggdrasil) 账户登录
    Mojang {
        /// 登录令牌，将会作为启动参数的一部分传入游戏实例
        access_token: Password,
        /// 正版玩家的统一标识
        uuid: String,
        /// 正版玩家的名称
        player_name: String,
        /// 正版玩家的头部皮肤位图信息，格式为 RGBA，大小为 8x8，用于展示头像
        head_skin: Vec<u8>,
        /// 正版玩家的头发皮肤位图信息，格式为 RGBA，大小为 8x8，用于展示头像
        hat_skin: Vec<u8>,
    },
    /// 微软账户
    Microsoft {
        /// 登录令牌，将会作为启动参数的一部分传入游戏实例
        access_token: Password,
        /// 刷新令牌，校验/更新登录令牌时实现携带这个作为参数
        refresh_token: Password,
        /// 正版玩家的统一标识
        uuid: String,
        /// 正版玩家的 XBox 用户 ID，用途不明，但是在新版本的 Minecraft 有发现需要使用这个 XUID 的地方
        xuid: String,
        /// 正版玩家的名称
        player_name: String,
        /// 正版玩家的头部皮肤位图信息，格式为 RGBA，大小为 8x8，用于展示头像
        head_skin: Vec<u8>,
        /// 正版玩家的头发皮肤位图信息，格式为 RGBA，大小为 8x8，用于展示头像
        hat_skin: Vec<u8>,
    },
    /// 外置登录（Authlib-Injector）
    AuthlibInjector {
        /// 第三方登录 API 提供方的 API 链接，登录的请求将通过这个 API 发送
        api_location: String,
        /// 第三方登录 API 提供方的服务器名称，用于 GUI 显示
        server_name: String,
        /// 第三方登录 API 提供方的网页主页，用于 GUI 显示跳转
        server_homepage: String,
        /// 第三方登录 API 提供方的元数据，需要在启动时携带这个作为参数
        server_meta: String,
        /// 第三方登录令牌，将会作为启动参数的一部分传入游戏实例
        access_token: Password,
        /// 第三方正版玩家的统一标识
        uuid: String,
        /// 第三方正版玩家的名称
        player_name: String,
        /// 第三方正版玩家的头部皮肤位图信息，格式为 RGBA，大小为 8x8，用于展示头像
        head_skin: Vec<u8>,
        /// 第三方正版玩家的头发皮肤位图信息，格式为 RGBA，大小为 8x8，用于展示头像
        hat_skin: Vec<u8>,
    },
}

pub(crate) mod mojang {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct AuthenticateBody {
        pub agent: AuthenticateAgent,
        pub username: String,
        pub password: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        pub client_token: String,
        pub request_user: bool,
    }

    impl Default for AuthenticateBody {
        fn default() -> Self {
            Self {
                request_user: true,
                username: "".into(),
                password: "".into(),
                client_token: "".into(),
                agent: Default::default(),
            }
        }
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub(crate) struct AuthenticateAgent {
        pub name: String,
        pub version: usize,
    }

    impl Default for AuthenticateAgent {
        fn default() -> Self {
            Self {
                name: "Minecraft".into(),
                version: 1,
            }
        }
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct AuthenticateResponse {
        pub access_token: String,
        pub client_token: String,
        pub available_profiles: Vec<AvaliableProfile>,
        pub selected_profile: Option<AvaliableProfile>,
    }

    #[derive(Debug, Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct ValidateResponse {
        pub access_token: String,
        pub client_token: String,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub(crate) struct AvaliableProfile {
        pub name: String,
        pub id: String,
    }

    /// 来自 Mojang 或第三方正版登录传回的错误响应
    #[derive(Debug, Deserialize, PartialEq, Eq, Clone, Default)]
    #[serde(rename_all = "camelCase")]
    pub struct ErrorResponse {
        /// 错误的字符串 ID
        pub error: String,
        /// 错误的字符串信息，可以展示给用户以确认错误原因
        pub error_message: String,
        /// 错误的形成原因，大多数情况这里是空的
        #[serde(default)]
        pub cause: String,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
    pub(crate) struct ProfileResponse {
        pub id: String,
        pub name: String,
        pub properties: Vec<ProfilePropertie>,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
    pub(crate) struct ProfilePropertie {
        pub name: String,
        pub value: String,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct ProfileTexture {
        pub timestamp: u64,
        pub profile_id: String,
        pub profile_name: String,
        pub textures: Option<TextureData>,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
    pub(crate) struct TextureData {
        #[serde(rename = "SKIN")]
        pub skin: Option<SkinData>,
        #[serde(rename = "CAPE")]
        pub cape: Option<SkinData>,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
    pub(crate) struct SkinData {
        pub url: String,
    }
}
