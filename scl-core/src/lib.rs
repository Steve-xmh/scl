/*!
   <div align="left">
       <h1>SCL Core</h1>
       <span>
           一个 Minecraft 启动框架，作为作者的项目 ——
           <a href="https://steve-xmh.github.io/scl/">Sharp Craft Launcher</a>
           的主要启动框架。
       </span>
   </div>

   ## 功能/特点

   - 全异步操作，使用 smol 作为异步框架，快速且轻量
   - 全版本启动支持
   - 下载纯净游戏
   - 下载 Forge 模组安装器
   - 下载 Fabric 模组安装器
   - 下载 Optifine 模组
   - 自定义启动参数
   - 正版登录（Mojang, Microsoft（你需要自行获取到回调链接））
   - 多下载源（BMCLAPI MC）
   - Curseforge 模组检索/下载

   ## 部分引用的 JAR 的原仓库

   - [forge-install-bootstrapper.jar](https://github.com/Steve-xmh/forge-install-bootstrapper)
   - [log4j-patch-agent-1.0.jar](https://github.com/saharNooby/log4j-vulnerability-patcher-agent)
   - [optifine-installer.jar](https://github.com/Steve-xmh/optifine-installer)

*/

#![forbid(missing_docs)]
#![allow(async_fn_in_trait)]

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
