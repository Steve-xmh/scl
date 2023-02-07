//! 所有的启动器元数据结构都在这里

use std::{
    collections::BTreeMap as Map,
    fmt,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use inner_future::stream::StreamExt;
use serde::{
    de::{self, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

use super::VersionType;
use crate::{
    package::PackageName,
    prelude::*,
    semver::MinecraftVersion,
    utils::{get_full_path, NATIVE_ARCH_LAZY},
};

/// 一个针对系统的规则
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct OSRule {
    /// 系统名称，通常是直接和 [`crate::utils::TARGET_OS`] 比对即可
    #[serde(default)]
    pub name: String,
    /// 系统版本号，依照平台而定
    #[serde(default)]
    pub version: String,
    /// 系统架构，通常是直接和 [`crate::utils::NATIVE_ARCH_LAZY`] 比对即可
    #[serde(default)]
    pub arch: String,
}

/// 一个规则
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct ApplyRule {
    /// 规则的类型，通常是 `allow` 或者 `disallow`
    pub action: String,
    /// 规则需要满足的操作系统类型
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<OSRule>,
    /// 规则需要满足的一些特殊情况
    ///
    /// 但是 SCL 并没有使用这部分来做判断
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Map<String, bool>>,
}

/// 一个用于检查规则是否满足条件的特质
///
/// [`ApplyRule`] 实现了这个特质
pub trait Allowed {
    /// 判断当前情况是否满足该规则
    fn is_allowed(&self) -> bool;
}

impl Allowed for [ApplyRule] {
    fn is_allowed(&self) -> bool {
        if self.is_empty() {
            true
        } else {
            let mut should_push = false;
            for rule in self {
                if rule.action == "disallow" {
                    if let Some(os) = &rule.os {
                        if !os.name.is_empty()
                            && os.name != crate::utils::TARGET_OS
                            && !os.arch.is_empty()
                            && os.arch != NATIVE_ARCH_LAZY.as_ref()
                        {
                            continue;
                        } else {
                            break;
                        }
                    } else {
                        continue;
                    }
                } else if rule.action == "allow" {
                    if let Some(os) = &rule.os {
                        if (!os.name.is_empty() && os.name != crate::utils::TARGET_OS)
                            || (!os.arch.is_empty() && os.arch != NATIVE_ARCH_LAZY.as_ref())
                        {
                            continue;
                        } else {
                            should_push = true;
                            break;
                        }
                    } else {
                        should_push = true; // 可能会有不允许的情况，继续寻找
                        continue;
                    }
                }
            }
            should_push
        }
    }
}

/// 特殊指派的参数
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct SpecificalArgument {
    /// 添加此参数需要满足的条件
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<ApplyRule>,
    /// 需要添加的参数
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(deserialize_with = "string_or_seq")]
    pub value: Vec<String>,
}

/// 游戏启动的一个参数
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum Argument {
    /// 无条件的参数，必须附加
    Common(String),
    /// 有条件的参数，需要根据条件附加
    Specify(SpecificalArgument),
}

#[test]
fn argument_test() {
    let text = serde_json::from_str::<Argument>(r#""test""#).unwrap();
    assert_eq!(text, Argument::Common("test".into()));
    let specify = serde_json::from_str::<Argument>(r#"{"rules":[],"value":"test"}"#).unwrap();
    assert_eq!(
        specify,
        Argument::Specify(SpecificalArgument {
            rules: vec![],
            value: vec!["test".into()]
        })
    );
}

/// 游戏启动参数
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
#[serde(default)]
pub struct Arguments {
    /// 用于添加到主类参数后面的游戏参数
    pub game: Vec<Argument>,
    /// 用于添加到 Class Path 前面的 JVM 参数
    pub jvm: Vec<Argument>,
}

/// 素材索引信息
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct AssetIndex {
    /// 素材的索引文件 ID，通常是当前的游戏版本号
    pub id: String,
    /// 素材的索引文件 SHA1 摘要
    pub sha1: String,
    /// 素材的索引文件大小，以字节为单位
    pub size: u32,
    /// 所有素材的总计大小，以字节为单位
    #[serde(rename = "totalSize")]
    pub total_size: u32,
    /// 素材的索引文件的下载链接
    pub url: String,
}

/// 一个下载项目结构
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct DownloadItem {
    /// 如果提供，则是下载的安装路径
    #[serde(default)]
    pub path: String,
    /// 该下载项目的文件 SHA1 摘要值
    pub sha1: String,
    /// 该下载项目的文件大小，以字节为单位
    pub size: usize,
    /// 该下载项目的下载链接，如果不提供则应该都是 Maven 的仓库路径，开头加上镜像源链接下载即可
    pub url: String,
}

/// 依赖库的下载信息结构
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct LibraryDownload {
    /// 需要下载的包，通常是 JAR 文件
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<DownloadItem>,
    /// 旧版原生库的分类下载信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classifiers: Option<Map<String, DownloadItem>>,
}

/// 一个依赖库结构
///
/// 通过访问 [`Library::rules`] 并调用 [`Allowed::is_allowed`] 来确认此依赖是否需要被下载/添加
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Library {
    /// 添加这个依赖库需要满足的规则
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<ApplyRule>,
    /// 需要下载的依赖库项目
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloads: Option<LibraryDownload>,
    /// 旧版本的依赖下载链接，常见于一些模组加载器里
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 旧版本的依赖原生库项目，新版本将直接把原生库放在了 [`Library::downloads`] 项目里直接作为 Class Path 的一部分导入了。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub natives: Option<Map<String, String>>,
    /// 这个依赖库的名称，通常是包名和版本号
    pub name: String,
}

/// 日志配置文件的下载信息
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct LoggingFile {
    /// 日志配置的 ID
    pub id: String,
    /// 日志配置文件的 SHA1 摘要
    pub sha1: String,
    /// 日志配置文件的大小，以字节为单位
    pub size: u32,
    /// 日志配置文件的下载链接
    pub url: String,
}

/// 日志处理方式，通常是 Log4J 的相关配置
///
/// 不过 SCL 并没有用这里的东西
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct LoggingConfig {
    /// 日志相关参数
    pub argument: String,
    /// 日志输出类型
    #[serde(rename = "type")]
    pub logger_type: String,
    /// 日志配置文件的下载结构
    pub file: LoggingFile,
}

/// SCL 的独立版本设置，此处的信息会
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
#[serde(default)]
pub struct SCLLaunchConfig {
    /// 最大内存，单位 MB，如不提供则为自动
    pub max_mem: Option<usize>,
    /// Java 运行时路径
    pub java_path: String,
    /// 是否使用版本独立
    pub game_independent: bool,
    /// 设定游戏窗口标题
    pub window_title: String,
    /// 额外的 JVM 参数，将会附加到 Class Path 前面
    pub jvm_args: String,
    /// 额外的游戏参数，将会附加到参数末尾
    pub game_args: String,
    /// 包装器执行文件路径，对于某些 Linux 用户有用，用于指定 Java 前的执行文件
    pub wrapper_path: String,
    /// 包装器执行文件参数，将会附加到包装器执行文件后
    pub wrapper_args: String,
}

/// 版本元数据里的日志方式
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Logging {
    /// 针对客户端的日志处理方式
    pub client: Option<LoggingConfig>,
}

/// 新版本的 Java 版本元数据
///
/// 如果存在则可以根据此数据选择对应的 Java 运行时
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct JavaVersion {
    /// 该 Java 运行时版本在官方启动器中的代号
    ///
    /// 通过该代号可以找到官方使用的 JVM 运行时
    pub component: String,
    /// 该 Java 运行时的主要版本号
    #[serde(rename = "majorVersion")]
    pub major_version: u8,
}

/// 版本元数据
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct VersionMeta {
    /// 继承某个版本
    #[serde(default)]
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: String,
    /// 1.12 以前的继承版本
    #[serde(default)]
    #[serde(rename = "clientVersion")]
    pub client_version: String,
    #[serde(default)]
    #[serde(rename = "javaVersion")]
    /// 新版本的 Java 版本元数据
    ///
    /// 如果存在则可以根据此数据选择对应的 Java 运行时
    pub java_version: Option<JavaVersion>,
    /// 游戏启动参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Arguments>,
    /// 游戏启动参数 旧版本
    #[serde(default)]
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: String,
    /// 资源索引元数据
    #[serde(rename = "assetIndex")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_index: Option<AssetIndex>,
    /// 该版本需要下载的主要游戏文件信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloads: Option<Map<String, DownloadItem>>,
    /// 该版本所需的依赖库列表
    #[serde(default)]
    pub libraries: Vec<Library>,
    /// 该版本元数据的日志输出配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<Logging>,
    /// 该版本元数据的主类
    #[serde(rename = "mainClass")]
    pub main_class: String,
    /// 该版本元数据的主类列表，因为有可能存在多个主类 JAR
    #[serde(skip)]
    pub main_jars: Vec<String>,
}

impl VersionMeta {
    pub(crate) fn fix_libraries(&mut self) {
        for library in &mut self.libraries {
            if library.rules.is_allowed()
                && library.downloads.is_none()
                && library.natives.is_none()
            {
                if let Ok(p) = library.name.parse::<PackageName>() {
                    let p = p.to_maven_jar_path("");
                    library.downloads = Some(LibraryDownload {
                        artifact: Some(DownloadItem {
                            path: p,
                            sha1: "".into(),
                            size: 0,
                            url: "".into(),
                        }),
                        classifiers: None,
                    })
                }
            }
        }
    }

    /// 根据元数据判断需要的最低 Java 运行时版本
    pub fn required_java_version(&self) -> u8 {
        if let Some(java_version) = &self.java_version {
            java_version.major_version
        } else if let Some(assets) = &self.asset_index {
            if let Ok((_, ver)) = crate::semver::parse_version(&assets.id) {
                ver.required_java_version()
            } else {
                8
            }
        } else if !self.inherits_from.is_empty() {
            if let Ok((_, ver)) = crate::semver::parse_version(&self.inherits_from) {
                ver.required_java_version()
            } else {
                8
            }
        } else {
            8
        }
    }
}

impl std::ops::AddAssign for VersionMeta {
    fn add_assign(&mut self, data: VersionMeta) {
        self.main_class = data.main_class.to_owned();
        self.minecraft_arguments = data.minecraft_arguments;
        self.libraries.extend_from_slice(&data.libraries);
        self.main_jars.extend_from_slice(&data.main_jars);
        if let Some(downloads) = &mut data.downloads.to_owned() {
            if let Some(self_downloads) = &mut self.downloads {
                self_downloads.append(downloads);
            } else {
                self.downloads = Some(downloads.to_owned());
            }
        }
        if let Some(arguments) = &data.arguments {
            if let Some(self_arguments) = &mut self.arguments {
                for a in arguments.jvm.iter() {
                    self_arguments.jvm.push(a.to_owned());
                }
                for a in arguments.game.iter() {
                    self_arguments.game.push(a.to_owned());
                }
            } else {
                self.arguments = Some(arguments.to_owned())
            }
        }
        if let Some(logging) = &data.logging {
            self.logging = Some(logging.to_owned());
        }
    }
}

/// 版本元数据结构
///
/// 当提供了 [`VersionInfo::version_base`] 和 [`VersionInfo::version`]
/// 两个字段的信息后可使用 [`VersionInfo::load`] 来读取其他数据
#[derive(Debug, Clone, Default)]
pub struct VersionInfo {
    /// 版本文件夹主目录
    pub version_base: String,
    /// 版本名称
    pub version: String,
    /// 读取成功之后的元数据
    pub meta: Option<VersionMeta>,
    /// 启动器配置
    pub scl_launch_config: Option<SCLLaunchConfig>,
    /// 猜测的版本类型
    pub version_type: VersionType,
    /// 猜测的原始 Minecraft 版本类型
    pub minecraft_version: MinecraftVersion,
    /// 猜测的所需最低 Java 版本，1.17 之后一般为 16+，其余的为 8+
    pub required_java: u8,
}

impl VersionInfo {
    /// 根据 [`VersionInfo::version_base`] 和 [`VersionInfo::version`] 读取版本元数据信息
    pub async fn load(&mut self) -> DynResult {
        let version_base_path = Path::new(&self.version_base);
        if version_base_path.is_dir() {
            let jar_path = version_base_path
                .join(&self.version)
                .join(format!("{}.jar", &self.version));
            let meta_path = version_base_path
                .join(&self.version)
                .join(format!("{}.json", &self.version));
            let scl_config_path = version_base_path.join(&self.version).join(".scl.json");
            if !meta_path.is_file() {
                anyhow::bail!(
                    "该版本 {} （游戏文件夹：{}） 缺失元数据文件",
                    &self.version,
                    &self.version_base
                )
            } else {
                if scl_config_path.is_file() {
                    // 加载启动器设置
                    let data = inner_future::fs::read_to_string(scl_config_path).await?;
                    let scl_config = serde_json::from_str(data.trim_start_matches('\u{feff}'))?; // 去掉可能存在的 BOM
                    self.scl_launch_config = Some(scl_config);
                }
                // 解析元文件，提取数据
                let data = inner_future::fs::read_to_string(meta_path).await?;
                let mut meta: VersionMeta =
                    serde_json::from_str(data.trim_start_matches('\u{feff}'))?; // 去掉可能存在的 BOM
                if jar_path.is_file() {
                    meta.main_jars.push(get_full_path(jar_path));
                }
                self.required_java = meta.required_java_version();
                if let Some(assets) = &meta.asset_index {
                    if let Ok((_, ver)) = crate::semver::parse_version(&assets.id) {
                        self.minecraft_version = ver;
                    }
                } else if !meta.inherits_from.is_empty() {
                    if let Ok((_, ver)) = crate::semver::parse_version(&meta.inherits_from) {
                        self.minecraft_version = ver;
                    }
                }
                self.meta = Some(meta);
                self.version_type = self.guess_version_type();
                Ok(())
            }
        } else {
            anyhow::bail!("游戏文件夹 {} 不是正确的文件夹", &self.version_base)
        }
    }

    /// 删除版本文件夹，约等于删除整个版本
    ///
    /// 但是注意本操作不会清理 assets 文件夹和 libraries 文件夹的内容
    pub async fn delete(self) {
        let version_base_path = Path::new(&self.version_base);
        if version_base_path.is_dir() {
            let version_path = version_base_path.join(&self.version);
            let _ = inner_future::fs::remove_dir_all(version_path).await;
        }
    }

    /// 重命名版本，如果目标版本名没有已有版本则会尝试重命名版本文件夹到该名称下
    pub async fn rename_version(&mut self, new_version_name: &str) -> DynResult {
        let version_base_path = Path::new(&self.version_base);
        if version_base_path.is_dir() {
            let version_path = version_base_path.join(&self.version);
            let version_jar_path = version_path.join(format!("{}.jar", self.version));
            let version_json_path = version_path.join(format!("{}.json", self.version));
            let new_version_path = version_base_path.join(new_version_name);
            let new_version_jar_path = version_path.join(format!("{}.jar", new_version_name));
            let new_version_json_path = version_path.join(format!("{}.json", new_version_name));
            if new_version_path.is_dir() {
                anyhow::bail!("目标版本名称已存在")
            } else {
                if version_jar_path.is_file() {
                    inner_future::fs::rename(version_jar_path, new_version_jar_path).await?;
                }
                if version_json_path.is_file() {
                    inner_future::fs::rename(version_json_path, new_version_json_path).await?
                };
                inner_future::fs::rename(version_path, new_version_path).await?;
                self.version = new_version_name.to_owned();
                Ok(())
            }
        } else {
            anyhow::bail!("文件夹不存在")
        }
    }

    /// 保存元数据和独立版本设置
    pub async fn save(&self) -> DynResult {
        let version_base_path = Path::new(&self.version_base);
        if version_base_path.is_dir() {
            let meta_path = version_base_path
                .join(&self.version)
                .join(format!("{}.json", &self.version));
            let scl_config_path = version_base_path.join(&self.version).join(".scl.json");
            // 这个不是刚需了
            // if !jar_path.is_file() {
            //     anyhow::bail!("版本 Jar 文件缺失")
            // }
            if !meta_path.is_file() {
                anyhow::bail!("版本 JSON 元数据文件缺失")
            } else {
                // 解析元文件，提取数据
                if let Some(meta) = &self.meta {
                    let file = std::fs::OpenOptions::new()
                        .truncate(true)
                        .write(true)
                        .open(meta_path);
                    if let Ok(file) = file {
                        match serde_json::to_writer(file, meta) {
                            Ok(_) => {}
                            Err(err) => {
                                anyhow::bail!("元数据解析失败：{}", err)
                            }
                        }
                    } else {
                        anyhow::bail!("无法打开版本元数据文件")
                    }
                }
                if let Some(scl_config) = &self.scl_launch_config {
                    let file = std::fs::OpenOptions::new()
                        .truncate(true)
                        .create(true)
                        .write(true)
                        .open(scl_config_path);
                    if let Ok(file) = file {
                        match serde_json::to_writer(file, scl_config) {
                            Ok(_) => {}
                            Err(err) => {
                                anyhow::bail!("SCL 配置文件写入失败：{}", err)
                            }
                        }
                    } else {
                        anyhow::bail!("无法打开 SCL 配置文件")
                    }
                } else if scl_config_path.is_file() {
                    inner_future::fs::remove_file(scl_config_path).await?;
                }
                Ok(())
            }
        } else {
            anyhow::bail!("版本文件夹未找到")
        }
    }

    /// 根据元数据猜测版本的种类
    pub fn guess_version_type(&self) -> VersionType {
        let mut has_optifine = false;
        let mut has_fabric = false;
        if let Some(meta) = &self.meta {
            for lib in &meta.libraries {
                if lib.name.starts_with("net.fabricmc:") {
                    // 也有可能是 QuiltMC
                    has_fabric = true;
                } else if lib.name.starts_with("net.minecraftforge:") {
                    return VersionType::Forge;
                } else if lib.name.starts_with("org.quiltmc:") {
                    return VersionType::QuiltMC;
                } else if lib.name.starts_with("optifine:") {
                    // 我们要优先 Forge 和 Fabric
                    has_optifine = true;
                }
            }
            if has_fabric {
                VersionType::Fabric
            } else if has_optifine {
                VersionType::Optifine
            } else {
                VersionType::Vanilla
            }
        } else {
            VersionType::Unknown
        }
    }

    /// 一个指向版本主目录的位置
    ///
    /// 如果没有开启版本独立，则会返回类似 `.minecraft` 的文件夹路径
    ///
    /// 如果开启版本独立，则会返回类似 `.minecraft/versions/版本名称` 的文件夹路径
    pub fn version_path(&self) -> PathBuf {
        if self
            .scl_launch_config
            .as_ref()
            .map(|x| x.game_independent)
            .unwrap_or(false)
        {
            // 版本独立
            let mut result = PathBuf::new();
            result.push(&self.version_base);
            result.push(&self.version);
            result
        } else {
            let mut result = PathBuf::new();
            result.push(&self.version_base);
            result.pop();
            result
        }
    }

    /// 读取该版本下的所有模组信息
    pub async fn get_mods(&self) -> DynResult<Vec<super::mods::Mod>> {
        let mods_path = self.version_path().join("mods");
        if !mods_path.is_dir() {
            return Ok(vec![]);
        }
        let mut files = inner_future::fs::read_dir(mods_path).await?;
        let mut results = vec![];
        while let Some(file) = files.try_next().await? {
            if file.path().is_file()
                && file
                    .path()
                    .file_name()
                    .map(|x| x.to_string_lossy().ends_with(".jar"))
                    == Some(true)
                || file
                    .path()
                    .file_name()
                    .map(|x| x.to_string_lossy().ends_with(".jar.disabled"))
                    == Some(true)
            {
                results.push(super::mods::Mod::from_path(file.path()).await?);
            }
        }
        Ok(results)
    }

    /// 根据版本实际情况获取最佳的最大内存用量，单位为 MB
    ///
    /// 代码参考自 <https://github.com/Hex-Dragon/PCL2/blob/f1310f18fda13b79b7a6189f02df15cd8300b28d/Plain%20Craft%20Launcher%202/Pages/PageSetup/PageSetupLaunch.xaml.vb#L327>
    pub async fn get_automated_maxium_memory(&self) -> u64 {
        let mem_status = crate::utils::get_mem_status();
        let mut free = dbg!(mem_status.free as i64);
        let mods = self.get_mods().await.unwrap_or_default();
        let (mem_min, mem_t1, mem_t2, mem_t3) = if !mods.is_empty() {
            (
                400 + mods.len() as i64 * 7,
                1500 + mods.len() as i64 * 10,
                3000 + mods.len() as i64 * 17,
                6000 + mods.len() as i64 * 34,
            )
        } else {
            (300, 1500, 2500, 4000)
        };
        // 预分配内存，阶段一，0 ~ T1，100%
        let mut result = 0;
        let mem_delta = mem_t1;
        free = (free - 100).max(0);
        result += free.min(mem_delta);
        free -= mem_delta + 100;
        if free < 100 {
            return result.max(mem_min) as _;
        }
        // 预分配内存，阶段二，T1 ~ T2，80%
        let mem_delta = mem_t2 - mem_t1;
        free = (free - 100).max(0);
        result += ((free as f64 * 0.8) as i64).min(mem_delta);
        free -= ((mem_delta as f64 / 0.8) as i64) + 100;
        if free < 100 {
            return result.max(mem_min) as _;
        }
        // 预分配内存，阶段三，T2 ~ T3，60%
        let mem_delta = mem_t2 - mem_t1;
        free = (free - 200).max(0);
        result += ((free as f64 * 0.6) as i64).min(mem_delta);
        free -= ((mem_delta as f64 / 0.6) as i64) + 200;
        if free < 100 {
            return result.max(mem_min) as _;
        }
        // 预分配内存，阶段四，T3 ~ T3 * 2，40%
        let mem_delta = mem_t3;
        free = (free - 300).max(0);
        result += ((free as f64 * 0.4) as i64).min(mem_delta);
        free -= ((mem_delta as f64 / 0.4) as i64) + 300;
        if free < 100 {
            return result.max(mem_min) as _;
        }
        result.max(mem_min) as _
    }

    /// 读取该版本下的所有世界存档信息
    ///
    /// 目前只是简单的扫一遍文件夹而已
    ///
    /// TODO: 做准确的解析过滤
    pub async fn get_saves(&self) -> DynResult<Vec<WorldSave>> {
        let saves_path = self.version_path().join("saves");
        let mut result = Vec::new();
        let mut files = inner_future::fs::read_dir(&saves_path).await?;
        while let Some(_file) = files.try_next().await? {
            result.push(WorldSave {})
        }
        Ok(result)
    }

    /// 读取该版本下的所有资源包信息
    ///
    /// 目前只是简单的扫一遍文件夹而已
    ///
    /// TODO: 做准确的解析过滤
    pub async fn get_resources_packs(&self) -> DynResult<Vec<ResourcesPack>> {
        let resourcepacks_path = self.version_path().join("resourcepacks");
        let texturepacks_path = self.version_path().join("texturepacks");
        let mut result = Vec::new();
        if resourcepacks_path.is_dir() {
            let mut files = inner_future::fs::read_dir(&resourcepacks_path).await?;
            while let Some(_file) = files.try_next().await? {
                result.push(ResourcesPack {})
            }
        }
        if texturepacks_path.is_dir() {
            let mut files = inner_future::fs::read_dir(&texturepacks_path).await?;
            while let Some(_file) = files.try_next().await? {
                result.push(ResourcesPack {})
            }
        }
        Ok(result)
    }
}

/// 一个世界存档的信息结构
#[derive(Debug)]
pub struct WorldSave {
    // TODO
}

/// 一个资源包的信息结构
#[derive(Debug)]
pub struct ResourcesPack {
    // TODO
}

fn string_or_seq<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrVec(PhantomData<Vec<String>>);

    impl<'de> Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or list of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value.into()])
        }

        fn visit_seq<S>(self, seq: S) -> Result<Self::Value, S::Error>
        where
            S: SeqAccess<'de>,
        {
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }
    }

    deserializer.deserialize_any(StringOrVec(PhantomData))
}
