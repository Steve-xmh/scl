//! 游戏资源下载模块，所有的游戏/模组/模组中文名称等数据的获取和安装都在这里

pub mod authlib;
pub mod curseforge;
pub mod fabric;
pub mod forge;
pub mod mcmod;
pub mod modrinth;
pub mod optifine;
pub mod structs;
pub mod vanilla;

use std::{fmt::Display, path::Path, str::FromStr};

use anyhow::Context;
use async_trait::async_trait;
pub use authlib::AuthlibDownloadExt;
pub use fabric::FabricDownloadExt;
pub use forge::ForgeDownloadExt;
pub use optifine::OptifineDownloadExt;
use serde::{Deserialize, Serialize};
pub use vanilla::VanillaDownloadExt;

use self::structs::VersionInfo;
use crate::{path::*, prelude::*, progress::*};

/// 游戏的下载来源，支持和 BMCLAPI 同格式的自定义镜像源
///
/// 通常国内的镜像源速度是比官方快的，但是更新不如官方的及时
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum DownloadSource {
    /// 全部使用原始来源下载
    Default,
    /// 全部使用 BMCLAPI 提供的镜像源下载
    ///
    /// 为了支持镜像源，在这里鼓励大家前去支持一下：<https://afdian.net/a/bangbang93>
    BMCLAPI,
    /// 全部使用 MCBBS 提供的镜像源下载
    MCBBS,
    /// 使用符合 BMCLAPI 镜像链接格式的自定义镜像源下载
    Custom(url::Url),
}

impl Default for DownloadSource {
    fn default() -> Self {
        Self::Default
    }
}

impl Display for DownloadSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DownloadSource::Default => "默认（官方）下载源",
                DownloadSource::BMCLAPI => "BMCLAPI 下载源",
                DownloadSource::MCBBS => "MCBBS 下载源",
                DownloadSource::Custom(_) => "自定义",
            }
        )
    }
}

impl FromStr for DownloadSource {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Offical" => Ok(Self::Default),
            "BMCLAPI" => Ok(Self::BMCLAPI),
            "MCBBS" => Ok(Self::MCBBS),
            s => {
                let url = s.parse::<url::Url>();
                if let Ok(url) = url {
                    Ok(Self::Custom(url))
                } else {
                    Ok(Self::Default)
                }
            }
        }
    }
}

/// 下载结构，用于存储下载所需的信息，并通过附带的扩展特质下载需要的东西
#[derive(Debug)]
pub struct Downloader<R> {
    /// 使用的下载源
    pub source: DownloadSource,
    /// 当前的 Minecraft 游戏目录路径
    pub(crate) minecraft_path: String,
    /// 当前的 Minecraft 依赖库目录路径
    pub(crate) minecraft_library_path: String,
    /// 当前的 Minecraft 版本文件夹目录路径
    pub(crate) minecraft_version_path: String,
    /// 当前的 Minecraft 资源文件夹目录路径
    pub(crate) minecraft_assets_path: String,
    /// 是否使用版本独立方式安装
    ///
    /// 这个会影响 Optifine 以模组形式的安装路径
    ///
    /// （会被安装在 版本/mods 文件夹里还是 .minecraft/mods 文件夹里）
    pub game_independent: bool,
    /// 是否验证已存在的文件是否正确
    pub verify_data: bool,
    /// 任意的 Java 运行时执行文件目录
    ///
    /// 在安装 Forge 时会使用
    pub java_path: String,
    /// 下载并发量
    parallel_amount: usize,
    /// 下载并发锁
    pub(crate) parallel_lock: inner_future::lock::Semaphore,
    /// 下载的进度报告对象
    pub reporter: Option<R>,
}

// let l = self.parallel_amount.acquire().await;

impl<R> Downloader<R> {
    /// 设置安装的目录，传入一个 `.minecraft` 文件夹路径作为参数
    ///
    /// 游戏将会被安装到此处
    pub fn set_minecraft_path(&mut self, dot_minecraft_path: impl AsRef<Path>) {
        let dot_minecraft_path = dot_minecraft_path.as_ref().to_path_buf();
        self.minecraft_path = dot_minecraft_path.to_string_lossy().to_string();
        self.minecraft_library_path = dot_minecraft_path
            .join("libraries")
            .to_string_lossy()
            .to_string();
        self.minecraft_version_path = dot_minecraft_path
            .join("versions")
            .to_string_lossy()
            .to_string();
        self.minecraft_assets_path = dot_minecraft_path
            .join("assets")
            .to_string_lossy()
            .to_string();
    }

    /// Builder 模式的 [`Downloader::set_minecraft_path`]
    pub fn with_minecraft_path(mut self, dot_minecraft_path: impl AsRef<Path>) -> Self {
        self.set_minecraft_path(dot_minecraft_path);
        self
    }
}

impl<R: Reporter> Clone for Downloader<R> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            minecraft_path: self.minecraft_path.clone(),
            minecraft_library_path: self.minecraft_library_path.clone(),
            minecraft_version_path: self.minecraft_version_path.clone(),
            minecraft_assets_path: self.minecraft_assets_path.clone(),
            game_independent: self.game_independent,
            verify_data: self.verify_data,
            java_path: self.java_path.clone(),
            parallel_lock: if self.parallel_amount == 0 {
                inner_future::lock::Semaphore::new(usize::MAX)
            } else {
                inner_future::lock::Semaphore::new(self.parallel_amount)
            },
            reporter: self.reporter.clone(),
            parallel_amount: self.parallel_amount,
        }
    }
}

impl<R: Reporter> Downloader<R> {
    /// 设置一个进度报告对象，下载进度将会被上报给这个对象
    #[must_use]
    pub fn with_reporter(mut self, reporter: R) -> Self {
        self.reporter = Some(reporter);
        self
    }
    /// 设置一个下载源
    #[must_use]
    pub fn with_source(mut self, source: DownloadSource) -> Self {
        self.source = source;
        self
    }
    /// 设置一个 Java 运行时，安装 Forge 和 Optifine 时需要用到
    #[must_use]
    pub fn with_java(mut self, java_path: String) -> Self {
        self.java_path = java_path;
        self
    }
    /// 设置是否使用版本独立方式安装
    ///
    /// 这个会影响 Optifine 以模组形式的安装路径
    ///
    /// （会被安装在 版本/mods 文件夹里还是 .minecraft/mods 文件夹里）
    #[must_use]
    pub fn with_game_independent(mut self, game_independent: bool) -> Self {
        self.game_independent = game_independent;
        self
    }
    /// 设置下载时的并发量，如果为 0 则不限制
    #[must_use]
    pub fn with_parallel_amount(mut self, limit: usize) -> Self {
        self.parallel_amount = limit;
        if limit == 0 {
            self.parallel_lock = inner_future::lock::Semaphore::new(usize::MAX);
        } else {
            self.parallel_lock = inner_future::lock::Semaphore::new(limit);
        }
        self
    }
    /// 是否强制校验已下载的文件以确认是否需要重新下载
    ///
    /// 如不强制则仅检测文件是否存在
    #[must_use]
    pub fn with_verify_data(mut self) -> Self {
        self.verify_data = true;
        self
    }
}
impl<R: Reporter> Default for Downloader<R> {
    fn default() -> Self {
        Self {
            source: DownloadSource::Default,
            minecraft_path: MINECRAFT_PATH.to_owned(),
            minecraft_library_path: MINECRAFT_LIBRARIES_PATH.to_owned(),
            minecraft_version_path: MINECRAFT_VERSIONS_PATH.to_owned(),
            minecraft_assets_path: MINECRAFT_ASSETS_PATH.to_owned(),
            game_independent: false,
            verify_data: false,
            java_path: {
                #[cfg(windows)]
                {
                    "javaw.exe".into()
                }
                #[cfg(not(windows))]
                {
                    "java".into()
                }
            },
            reporter: None,
            parallel_amount: 64,
            parallel_lock: inner_future::lock::Semaphore::new(64),
        }
    }
}

/// 一个游戏安装特质，如果你并不需要单独安装其它部件，则可以单独引入这个特质来安装游戏
#[async_trait]
pub trait GameDownload<'a>: FabricDownloadExt + ForgeDownloadExt + VanillaDownloadExt {
    /// 根据参数安装一个游戏，允许安装模组加载器
    async fn download_game(
        &self,
        version_name: &str,
        vanilla: VersionInfo,
        fabric: &str,
        forge: &str,
        optifine: &str,
    ) -> DynResult;
}

#[async_trait]
impl<R: Reporter> GameDownload<'_> for Downloader<R> {
    async fn download_game(
        &self,
        version_name: &str,
        vanilla: VersionInfo,
        fabric: &str,
        forge: &str,
        optifine: &str,
    ) -> DynResult {
        self.reporter
            .set_message(format!("正在下载游戏 {}", version_name));

        let launcher_profiles_path =
            std::path::Path::new(&self.minecraft_path).join("launcher_profiles.json");

        if !launcher_profiles_path.exists() {
            inner_future::fs::create_dir_all(launcher_profiles_path.parent().unwrap()).await?;
            inner_future::fs::write(launcher_profiles_path, r##"{"profiles":{},"selectedProfile":null,"authenticationDatabase":{},"selectedUser":{"account":"00000111112222233333444445555566","profile":"66666555554444433333222221111100"}}"##).await?;
        }

        if !fabric.is_empty() {
            crate::prelude::inner_future::future::try_zip(
                self.install_vanilla(version_name, &vanilla),
                self.download_fabric_pre(version_name, &vanilla.id, fabric),
            )
            .await?;
            self.download_fabric_post(version_name).await?;
        } else if !forge.is_empty() {
            self.install_vanilla(&vanilla.id, &vanilla).await?; // Forge 安装需要原版，如果安装器没有解析到则会从官方源下载，速度很慢
            crate::prelude::inner_future::future::try_zip(
                self.install_vanilla(version_name, &vanilla),
                self.install_forge_pre(version_name, &vanilla.id, forge),
            )
            .await?;
            self.install_forge_post(version_name, &vanilla.id, forge)
                .await?;
        } else {
            self.install_vanilla(version_name, &vanilla).await?;
        }
        if !optifine.is_empty() {
            if forge.is_empty() && fabric.is_empty() {
                self.install_vanilla(&vanilla.id, &vanilla).await?; // Optifine 安装需要原版，如果安装器没有解析到则会从官方源下载，速度很慢
            }
            let (optifine_type, optifine_patch) =
                optifine.split_at(optifine.find(' ').context("Optifine 版本字符串不合法！")?);
            self.install_optifine(
                version_name,
                &vanilla.id,
                optifine_type,
                &optifine_patch[1..],
                !forge.is_empty() || !fabric.is_empty(),
            )
            .await?;
        }

        // 这俩都需要安装器，而安装后会生成一个新的版本元数据
        // 因此需要最后扫描一遍生成出来的版本元数据依赖，再进行一次下载
        if !optifine.is_empty() || !forge.is_empty() {
            let mut version_info = crate::version::structs::VersionInfo {
                version_base: self.minecraft_version_path.to_owned(),
                version: version_name.to_owned(),
                ..Default::default()
            };

            if version_info
                .load()
                .await
                .context("无法读取安装完成后的版本元数据！")
                .is_ok()
            {
                if let Some(meta) = &mut version_info.meta {
                    meta.fix_libraries();
                    self.download_libraries(&meta.libraries).await?;
                }
            }
        }
        Ok(())
    }
}
