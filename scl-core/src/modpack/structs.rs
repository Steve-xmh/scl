//! 所有的整合包元数据结构都在这里

use serde::Deserialize;

/// 整合包文件中的元数据文件根结构
///
/// 该文件通常命名为 `manifest.json` 存放在压缩文件内的根目录
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackManifest {
    /// 整合包的类别，对于 Minecraft 来说通常是 minecraftModpack
    pub manifest_type: String,
    /// 整合包元数据版本，截至本模块开发时都是 1
    pub manifest_version: usize,
    /// 整合包的名称
    pub name: String,
    /// 整合包的版本
    pub version: String,
    /// 整合包的作者
    pub author: String,
    /// 整合包对应的 Minecraft 信息
    ///
    /// 包括原版版本和模组加载器版本
    pub minecraft: ModpackMinecraftMeta,
    /// 需要联网获取的项目文件
    ///
    /// 一般都是模组
    pub files: Vec<ProjectFile>,
    /// 在安装的最后再次覆盖安装目录文件的文件夹在压缩文件中的路径
    ///
    /// 如果指定且存在，那么该文件夹中的所有文件都会被解压覆盖到安装目录下
    pub overrides: String,
}

/// 整合包元数据中提到的 Minecraft 信息
///
/// 包括原版版本和模组加载器版本
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackMinecraftMeta {
    /// 原版版本号
    pub version: String,
    /// 所需的模组加载器
    pub mod_loaders: Vec<ModLoader>,
}

/// 整合包元数据提到的模组加载器
///
/// CurseForge 上用的最多的应该就只有 Forge 了
///
/// Fabric 的整合包似乎也是用了奇怪的办法用 Forge 启动过来的
#[derive(Debug, Clone, Deserialize)]
pub struct ModLoader {
    /// 模组加载器的 ID
    ///
    /// 会和版本号一起提供，例如 RLCraft 中的 `forge-14.23.5.2860`
    pub id: String,
    /// 是否是主要模组加载器
    pub primary: bool,
}

/// 整合包元数据中提到的需要联网获取的项目文件
///
/// 一般都是模组
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectFile {
    /// CurseForge 上的项目 ID
    #[serde(rename = "projectID")]
    pub project_id: usize,
    /// 项目下对应的文件 ID
    #[serde(rename = "fileID")]
    pub file_id: usize,
    /// 是否是必需文件
    pub required: bool,
}
