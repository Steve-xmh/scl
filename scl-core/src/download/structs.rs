//! 下载源数据结构
use std::collections::BTreeMap as Map;

use serde::Deserialize;

/// 当前的最新版本和全部版本信息
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct VersionManifest {
    /// 最新的正式版本和快照版本
    pub latest: LatestVersion,
    /// 所有版本
    pub versions: Vec<VersionInfo>,
}

/// 最新的正式版本和快照版本数据结构
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct LatestVersion {
    /// 最新的正式版本号
    pub release: String,
    /// 最新的快照版本号
    pub snapshot: String,
}

/// 其中一个游戏版本的信息
#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct VersionInfo {
    /// 该版本的版本号
    pub id: String,
    /// 该版本的类型，有可能是 `release` 正式版本或 `snapshot` 快照版本
    #[serde(rename = "type")]
    pub version_type: String,
    /// 该版本的版本元数据下载链接
    ///
    /// <https://launchermeta.mojang.com/v1/packages/e849f376647fd7160146d77002a3084efa8fb36f/21w08b.json>
    pub url: String,
    /// 该版本的更新日期
    pub time: String,
    /// 该版本的发布日期
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

/// 资源索引信息
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct AssetIndexes {
    /// 所有的资源文件哈希对照表，键为原文件路径
    pub objects: Map<String, AssetItem>,
}

/// 资源项目信息
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct AssetItem {
    /// 该资源文件的 SHA1 摘要值
    ///
    /// 通过将其拆解成开头两个字再和原摘要值组合即可取得该资源的下载链接
    pub hash: String,
    /// 该资源文件的大小，以字节为单位
    pub size: usize,
}

/// Forge 加载器的版本信息
#[derive(Clone, Debug)]
pub struct ForgeVersionsData {
    /// 推荐下载的加载器版本
    pub recommended: Option<ForgeItemInfo>,
    /// 最新的加载器版本
    pub latest: Option<ForgeItemInfo>,
    /// 所有加载器版本
    pub all_versions: Vec<ForgeItemInfo>,
}

/// 一个加载器的版本信息
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ForgeItemInfo {
    /// 该加载器的版本号
    ///
    /// 在 1.14 以前，Forge 使用四元数来记录版本，而后去掉了最后一个构建版本号
    pub version: String,
    /// 该加载器对应支持的原版版本号
    pub mcversion: String,
    /// 该加载器的下载文件列表，有安装器或通用版本或模组开发套件（MDK）
    pub files: Vec<ForgeFile>,
}

/// 被特殊标记的 Forge 版本
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ForgePromoItem {
    /// 该 Forge 模组加载器的版本名称
    pub name: String,
    /// 该加载器的指定版本信息
    pub build: Option<ForgeItemInfo>,
}

/// 该文件的下载信息
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ForgeFile {
    /// 文件的类型，有可能是安装器或通用版本或模组开发套件（MDK）
    pub category: String,
    /// 文件的格式，有可能是 EXE ZIP JAR 等
    pub format: String,
}

/// Optifine 的版本信息
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct OptifineVersionMeta {
    /// 该 Optifine 对应支持的原版版本
    pub mcversion: String,
    /// 该 Optifine 的类型， 1.6+ 后此处固定为 `HD_U`
    #[serde(rename = "type")]
    pub version_type: String,
    /// 该 Optifine 的修订版本号
    pub patch: String,
    /// 该 Optifine 的下载文件名称
    pub filename: String,
}
