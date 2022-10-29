//! 一些常用的路径，用于跨平台统一
use once_cell::sync::Lazy;

type LazyString = Lazy<String>;

/// 默认的 Minecraft 主目录
///
/// 如果是 Windows 系统，这将指向当前工作目录的 `.minecraft` 文件夹
///
/// 如果是 Linux 系统，这将尝试指向当前主目录（Home Directory）的 `.minecraft` 文件夹
pub(crate) static MINECRAFT_PATH: LazyString = Lazy::new(|| {
    #[cfg(not(target_os = "linux"))]
    {
        ".minecraft".into()
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(dir) = dirs::home_dir() {
            dir.join(".minecraft").to_str().unwrap().to_string()
        } else {
            "~/.minecraft".into()
        }
    }
});

pub(crate) static MINECRAFT_ASSETS_PATH: LazyString =
    Lazy::new(|| format!("{}/assets", MINECRAFT_PATH.as_str()));

pub(crate) static MINECRAFT_LIBRARIES_PATH: LazyString =
    Lazy::new(|| format!("{}/libraries", MINECRAFT_PATH.as_str()));

pub(crate) static MINECRAFT_VERSIONS_PATH: LazyString =
    Lazy::new(|| format!("{}/versions", MINECRAFT_PATH.as_str()));
