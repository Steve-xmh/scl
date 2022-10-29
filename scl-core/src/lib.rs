/*!
    SharpCraftLauncher 的启动核心库，用于 Minacraft 的登录/启动/模组下载/版本管理等游戏操作

    ## 功能/特点

    - 全异步操作，使用 smol 作为异步框架，快速且轻量
    - 全版本启动支持
    - 下载纯净游戏
    - 下载 Forge 模组安装器
    - 下载 Fabric 模组安装器
    - 自定义启动参数
    - 正版登录（Mojang, Microsoft（你需要自行获取到回调链接））
    - 多下载源（BMCLAPI MCBBS MC）
    - Curseforge 模组检索/下载
    - 完整的开发文档（咱甚至用了 `#![forbid(missing_docs)]`！）
*/

#![forbid(missing_docs)]

pub mod auth;
pub mod client;
pub mod download;
pub mod http;
pub mod java;
pub mod password;
pub mod progress;
pub mod semver;
pub mod utils;
pub mod version;

pub(crate) mod package;
pub(crate) mod path;
pub(crate) mod prelude;
