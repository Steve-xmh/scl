//! 模组文件的管理
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::Context;
use image::DynamicImage;

use crate::prelude::*;

/// 模组文件数据
/// 可通过 [`crate::version::structs::VersionInfo::get_mods`] 获取
#[derive(Debug, Clone, Default)]
pub struct Mod {
    file_name: String,
    path: PathBuf,
    enabled: bool,
}

/// 一个 Fabric 模组的图标集，其内的成员都是文件路径
///
/// 图标可以是单个也可以是多个
///
/// 在 SCL 里，如果提供多个则默认展示第一个
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FabricModIcon {
    /// 多个图标
    Multiply(HashMap<String, String>),
    /// 单个图标
    Single(String),
}

/// 一个 Fabric 模组的元数据信息
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FabricModMeta {
    /// 模组的名称
    pub name: String,
    /// 模组的介绍
    pub description: String,
    /// 模组的版本号
    pub version: String,
    /// 模组的图标集
    pub icon: Option<FabricModIcon>,
}

/// 一个 Forge 模组的元数据信息
///
/// 注：在对老版本 1.12.2 以前的模组需要使用 [`Vec<ForgeModMeta>`] 来反序列化模组信息
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ForgeModMeta {
    /// 模组的名称
    pub name: String,
    /// 模组的介绍
    pub description: String,
    /// 模组的版本号
    pub version: String,
    /// 模组的图标文件路径
    #[serde(rename = "logoFile")]
    pub logo_file: String,
}

/// 一个模组的元数据信息
///
/// 有可能是 Forge 模组或者 Fabric 模组
#[derive(Debug, Clone)]
pub enum ModMeta {
    /// 一个 Fabric 模组的元数据
    Fabric(FabricModMeta),
    /// 一个 Forge 模组的元数据
    Forge(ForgeModMeta),
}

impl ModMeta {
    /// 获取模组的名字
    pub fn name(&self) -> &str {
        match &self {
            ModMeta::Fabric(m) => m.name.as_str(),
            ModMeta::Forge(m) => m.name.as_str(),
        }
    }

    /// 获取模组的版本号
    pub fn version(&self) -> &str {
        match &self {
            ModMeta::Fabric(m) => m.version.as_str(),
            ModMeta::Forge(m) => m.version.as_str(),
        }
    }
}

/// 新版 Forge 模组的元数据信息
///
/// 新版采用了 TOML 文件记录元数据，且支持单包多模组，故此处的 [`NewForgeModMeta::mods`] 是一个数组
#[derive(Debug, Clone, Deserialize)]
pub struct NewForgeModMeta {
    /// 当前模组包包含的所有模组
    pub mods: Vec<ForgeModMeta>,
}

impl Mod {
    /// 通过模组文件路径获取当前模组的信息
    pub async fn from_path(path: impl AsRef<Path>) -> DynResult<Self> {
        let path = path.as_ref();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        let enabled = !file_name.ends_with(".disabled");
        Ok(Self {
            file_name,
            path: path.to_path_buf(),
            enabled,
        })
    }

    /// 模组是否已经启用（既是否以 `.jar.disabled` 结尾）
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 模组的文件名
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// 尝试获取一个可供显示的模组名称
    pub async fn display_name(&self) -> String {
        self.try_get_mod_name().await.unwrap_or_else(|_| {
            self.file_name()
                .trim_end_matches(".jar.disabled")
                .trim_end_matches(".jar")
                .to_owned()
        })
    }

    /// 模组的文件所在路径
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// 尝试获取模组的元数据信息
    pub async fn try_get_mod_meta(&self) -> DynResult<ModMeta> {
        let path = self.path.to_owned();
        inner_future::unblock(move || -> DynResult<ModMeta> {
            let mut z = zip::ZipArchive::new(
                std::fs::OpenOptions::new()
                    .read(true)
                    .open(&path)
                    .with_context(|| format!("无法打开模组文件 {}", path.to_string_lossy()))
                    .with_context(|| format!("无法读取模组文件 {}", path.to_string_lossy()))?,
            )?;
            if let Ok(meta) = z.by_name("fabric.mod.json").and_then(|r| {
                serde_json::from_reader::<_, FabricModMeta>(r)
                    .map_err(|_| zip::result::ZipError::InvalidArchive("fabric.mod.json"))
            }) {
                // fabric.mod.json https://github.com/FabricMC/fabric-example-mod/blob/1.19/src/main/resources/fabric.mod.json
                Ok(ModMeta::Fabric(meta))
            } else if let Ok(meta) = z.by_name("mods.toml").and_then(|mut r| {
                // mods.toml https://docs.minecraftforge.net/en/1.19.x/gettingstarted/structuring/
                let mut buf = Vec::with_capacity(r.size() as _);
                r.read_to_end(&mut buf).unwrap_or_default();
                toml::from_slice::<NewForgeModMeta>(&buf)
                    .map_err(|_| zip::result::ZipError::InvalidArchive("mods.toml"))
            }) {
                Ok(ModMeta::Forge(meta.mods.first().cloned().ok_or_else(
                    || anyhow::anyhow!("Can't get mod info from mcmod.info"),
                )?))
            } else if let Ok(meta) = z.by_name("mcmod.info").and_then(|r| {
                // mcmod.info https://docs.minecraftforge.net/en/1.12.x/gettingstarted/structuring/
                serde_json::from_reader::<_, Vec<ForgeModMeta>>(r)
                    .map_err(|_| zip::result::ZipError::InvalidArchive("mcmod.info"))
            }) {
                Ok(ModMeta::Forge(meta.first().cloned().ok_or_else(|| {
                    anyhow::anyhow!("Can't get mod info from mcmod.info")
                })?))
            } else {
                anyhow::bail!("Mod name not found")
            }
        })
        .await
    }

    /// [`Mod::try_get_mod_meta`] 的语法糖，尝试根据模组元数据获取模组名称
    pub async fn try_get_mod_name(&self) -> DynResult<String> {
        Ok(self.try_get_mod_meta().await?.name().to_owned())
    }

    /// [`Mod::try_get_mod_meta`] 的语法糖，尝试根据模组元数据获取模组的其中一个图标
    pub async fn try_get_mod_icon(&self) -> DynResult<DynamicImage> {
        let icon_path = match self.try_get_mod_meta().await? {
            ModMeta::Fabric(meta) => match meta.icon {
                Some(FabricModIcon::Multiply(icon)) => {
                    icon.iter().next().map(|x| x.0.to_owned()).ok_or_else(|| {
                        anyhow::anyhow!("Can't find icon from such multiply fabric mod icons")
                    })?
                }
                Some(FabricModIcon::Single(icon)) => icon,
                None => anyhow::bail!("Can't find icon from such multiply fabric mod icons"),
            },
            ModMeta::Forge(meta) => meta.logo_file,
        };
        let path = self.path.to_owned();
        inner_future::unblock(move || -> DynResult<DynamicImage> {
            let mut z = zip::ZipArchive::new(std::fs::OpenOptions::new().read(true).open(path)?)?;
            let mut r = z.by_name(icon_path.trim_start_matches(['/', '\\']))?;
            let mut buf = Vec::with_capacity(r.size() as _);
            r.read_to_end(&mut buf)?;
            Ok(image::load_from_memory(&buf)?)
        })
        .await
    }

    /// 如果模组是禁用的（既以 `.jar.disabled` 结尾），启用该模组（既还原成 `.jar` 结尾）
    ///
    /// 如果重命名失败则会返回错误
    pub async fn enable(&mut self) -> DynResult {
        if self.file_name.ends_with(".disabled") && !self.enabled {
            let mut path = self.path.clone();
            path.set_file_name(&self.file_name[..self.file_name.len() - 9]);
            inner_future::fs::rename(&self.path, &path).await?;
            self.file_name = path.file_name().unwrap().to_string_lossy().to_string();
            self.path = path;
            self.enabled = true;
        }
        Ok(())
    }

    /// 如果模组是启用的（既以 `.jar` 结尾），禁用该模组（既重命名为以 `.jar.disabled` 结尾）
    ///
    /// 如果重命名失败则会返回错误
    pub async fn disable(&mut self) -> DynResult {
        if !self.file_name.ends_with(".disabled") && self.enabled {
            let mut path = self.path.clone();
            path.set_file_name(&self.file_name);
            path.set_extension(format!(
                "{}.disabled",
                path.extension()
                    .map(|x| x.to_string_lossy())
                    .unwrap_or_default()
            ));
            inner_future::fs::rename(&self.path, &path).await?;
            self.file_name = path.file_name().unwrap().to_string_lossy().to_string();
            self.path = path;
            self.enabled = false;
        }
        Ok(())
    }

    /// 删除该模组，**此操作不可撤销**
    pub async fn remove(self) -> DynResult {
        inner_future::fs::remove_file(self.path).await?;
        Ok(())
    }
}
