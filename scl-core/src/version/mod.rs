//! 游戏版本的解析

use std::path::Path;

pub mod mods;
pub mod structs;

use inner_future::stream::StreamExt;

use self::structs::VersionInfo;
use crate::prelude::*;

/// 一个游戏版本的信息
pub struct Version {
    /// 版本名称，通常和其文件夹同名
    pub name: String,
    /// 版本类型，一般通过 [`crate::version::structs::VersionInfo::guess_version_type`] 猜测出，用于展示版本类型和判断是否需要版本独立
    pub version_type: VersionType,
    /// 版本的创建日期，实际是该文件夹的创建日期
    pub created_date: std::time::SystemTime,
    /// 版本的上一次游玩日期，实际是该文件夹的上一次访问日期
    pub access_date: std::time::SystemTime,
}

impl Default for Version {
    fn default() -> Self {
        Self {
            name: String::new(),
            version_type: VersionType::Unknown,
            created_date: std::time::SystemTime::UNIX_EPOCH,
            access_date: std::time::SystemTime::UNIX_EPOCH,
        }
    }
}

/// 通过指定的版本文件夹，搜索所有可启动的游戏版本
pub async fn get_avaliable_versions(
    version_directory_path: impl AsRef<Path>,
) -> DynResult<Vec<Version>> {
    let mut version_info = VersionInfo {
        version_base: version_directory_path
            .as_ref()
            .to_string_lossy()
            .to_string(),
        ..Default::default()
    };
    if version_directory_path.as_ref().is_dir() {
        let mut entries = inner_future::fs::read_dir(version_directory_path.as_ref()).await?;
        let mut result = Vec::with_capacity(32);
        while let Some(entry) = entries.try_next().await? {
            let (created_date, access_date) = if let Ok(metadata) = entry.metadata().await {
                (
                    metadata
                        .created()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                    metadata
                        .accessed()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                )
            } else {
                (
                    std::time::SystemTime::UNIX_EPOCH,
                    std::time::SystemTime::UNIX_EPOCH,
                )
            };
            if entry.file_type().await?.is_dir() {
                let entry = entry
                    .path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
                version_info.version = entry.to_owned();
                if version_info.load().await.is_ok() {
                    result.push(Version {
                        name: entry,
                        version_type: version_info.guess_version_type(),
                        created_date,
                        access_date,
                    });
                } else {
                    result.push(Version {
                        name: entry,
                        version_type: VersionType::Unknown,
                        created_date,
                        access_date,
                    });
                }
            }
        }
        Ok(result)
    } else {
        Ok(vec![])
    }
}

/// 版本类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionType {
    /// 纯净版本
    Vanilla,
    /// Forge 版本
    Forge,
    /// NeoForge 版本
    NeoForge,
    /// Fabric 版本
    Fabric,
    /// QuiltMC 版本
    QuiltMC,
    /// Optifine 画质增强版本
    ///
    /// 如果其是通过其它模组加载器加载的（Forge 或 Fabric），则优先为模组加载器版本
    Optifine,
    /// 未知版本
    Unknown,
}

impl Default for VersionType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// 当解析出错时，此处为错误枚举值
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadVersionInfoError {
    /// 没有提供游戏版本文件夹路径或不存在
    VersionBaseMissing,
    /// 没有提供游戏本体文件路径或不存在
    MainJarFileMissing,
    /// 没有提供版本元数据文件路径或不存在
    VersionFileMissing,
    /// 无法打开版本元数据文件
    VersionFileOpenFail,
    /// 无法打开启动器配置文件
    SCLConfigFileOpenFail,
    /// 元数据解析错误，内容有误
    ParseError(String),
}
