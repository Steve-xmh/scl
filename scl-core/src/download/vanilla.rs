//! 原版游戏的下载模块

use std::{borrow::Cow, collections::HashMap, path::Path};

use anyhow::Context;
use async_trait::async_trait;
use inner_future::{fs::create_dir_all, io::AsyncWriteExt};
use tracing::*;

use super::{
    structs::{AssetIndexes, VersionManifest},
    DownloadSource, Downloader,
};
use crate::{
    download::VersionInfo,
    prelude::*,
    progress::Reporter,
    version::structs::{Allowed, Library, VersionMeta},
};

/// 一个用于下载安装原版的扩展特质，可以使用 [`crate::download::Downloader`] 来安装
#[async_trait]
pub trait VanillaDownloadExt: Sync {
    /// 获取现在所有可下载版本
    async fn get_avaliable_vanilla_versions(&self) -> DynResult<VersionManifest>;

    /// 下载原版客户端 JAR 文件
    async fn download_vanilla_jar(&self, path: &str, save_path: &str, sha1: &str) -> DynResult;

    /// 下载一个依赖库，并存放到指定位置
    async fn download_library(&self, sha1: String, path: String, save_path: &str) -> DynResult;

    /// 下载一组依赖库，安装位置由特质实现而定
    async fn download_libraries(
        &self,
        libraries: &[Library],
    ) -> DynResult<HashMap<String, Vec<String>>>;

    /// 下载游戏资源索引
    async fn download_asset_index(
        &self,
        name: &str,
        url: &str,
        save_path: &str,
    ) -> DynResult<AssetIndexes>;

    /// 下载一个游戏素材，并存放到指定位置
    async fn download_asset(
        &self,
        sha1: &str,
        name: &str,
        save_path: &str,
        is_pre: bool,
        r: Option<impl Reporter>,
    ) -> DynResult;

    /// 下载一个游戏版本
    async fn download_vanilla(
        &self,
        version_name: &str,
        version_meta: &VersionMeta,
        is_repair: bool,
    ) -> DynResult;

    /// 安装一个游戏版本
    async fn install_vanilla(&self, version_name: &str, version_info: &VersionInfo) -> DynResult;
}

#[async_trait]
impl<R: Reporter> VanillaDownloadExt for Downloader<R> {
    async fn get_avaliable_vanilla_versions(&self) -> DynResult<VersionManifest> {
        let res = crate::http::retry_get_json(match self.source {
            DownloadSource::Default => {
                "https://piston-meta.mojang.com/mc/game/version_manifest.json"
            }
            DownloadSource::BMCLAPI => {
                "https://bmclapi2.bangbang93.com/mc/game/version_manifest.json"
            }
            DownloadSource::MCBBS => "https://download.mcbbs.net/mc/game/version_manifest.json",
            _ => "https://piston-meta.mojang.com/mc/game/version_manifest.json",
        })
        .await
        .map_err(|e| anyhow::anyhow!("获取可用原版列表失败：{:?}", e))?;
        Ok(res)
    }

    async fn download_vanilla_jar(&self, path: &str, save_path: &str, sha1: &str) -> DynResult {
        let l = self.parallel_lock.acquire().await;
        let r = self.reporter.sub();
        r.add_max_progress(1.);
        let name = &save_path[save_path.rfind(std::path::is_separator).unwrap_or(0) + 1..];
        r.set_message(format!("正在下载原版 {name}"));
        if std::path::Path::new(&save_path).is_file() {
            if self.verify_data {
                let mut file = inner_future::fs::OpenOptions::new()
                    .read(true)
                    .open(&save_path)
                    .await?;
                let current_sha1 = crate::utils::get_data_sha1(&mut file).await?;
                if sha1 == current_sha1 {
                    r.add_progress(1.);
                    return Ok(());
                }
            } else {
                r.add_progress(1.);
                return Ok(());
            }
        } else {
            inner_future::fs::create_dir_all(
                &save_path[..save_path
                    .rfind(std::path::is_separator)
                    .unwrap_or(save_path.len())],
            )
            .await?;
        }
        let path = path.parse::<url::Url>()?;
        let uris = [
            match self.source {
                DownloadSource::Default => {
                    format!("https://launcher.mojang.com{}", path.path())
                }
                DownloadSource::BMCLAPI => {
                    format!("https://bmclapi2.bangbang93.com{}", path.path())
                }
                DownloadSource::MCBBS => format!("https://download.mcbbs.net{}", path.path()),
                _ => format!("https://launcher.mojang.com{}", path.path()),
            },
            format!("https://bmclapi2.bangbang93.com{}", path.path()),
            format!("https://download.mcbbs.net{}", path.path()),
            format!("https://launcher.mojang.com{}", path.path()),
        ];
        crate::http::download(&uris, save_path, 0)
            .await
            .map_err(|e| anyhow::anyhow!("下载原版游戏 Jar 失败：{:?}", e))?;
        r.add_progress(1.);
        drop(l);
        Ok(())
    }

    async fn download_library(&self, sha1: String, path: String, save_path: &str) -> DynResult {
        let l = self.parallel_lock.acquire().await;
        let r = self.reporter.sub();
        let full_path = format!("{save_path}/{path}");
        r.set_message(format!("正在下载原版库 {path}"));
        r.add_max_progress(1.);
        if std::path::Path::new(&full_path).is_file() {
            if self.verify_data {
                let mut file = inner_future::fs::OpenOptions::new()
                    .read(true)
                    .open(&full_path)
                    .await?;
                let current_sha1 = crate::utils::get_data_sha1(&mut file).await?;
                if sha1 == current_sha1 {
                    r.add_progress(1.);
                    return Ok(());
                }
            } else {
                r.add_progress(1.);
                return Ok(());
            }
        } else {
            inner_future::fs::create_dir_all(
                &full_path[..full_path
                    .rfind(std::path::is_separator)
                    .unwrap_or(full_path.len())],
            )
            .await?;
        }
        let default_uris = [
            match self.source {
                DownloadSource::Default => {
                    format!("https://libraries.minecraft.net/{path}")
                }
                DownloadSource::BMCLAPI => {
                    format!("https://bmclapi2.bangbang93.com/maven/{path}")
                }
                DownloadSource::MCBBS => format!("https://download.mcbbs.net/maven/{path}"),
                _ => format!("https://libraries.minecraft.net/{path}"),
            },
            format!("https://bmclapi2.bangbang93.com/maven/{path}"),
            format!("https://download.mcbbs.net/maven/{path}"),
            format!("https://libraries.minecraft.net/{path}"),
        ];
        crate::http::download(&default_uris, &full_path, 0)
            .await
            .map_err(|e| anyhow::anyhow!("下载库 {} 失败：{:?}", path, e))?;
        r.add_progress(1.);
        drop(l);
        Ok(())
    }

    async fn download_asset_index(
        &self,
        name: &str,
        url: &str,
        save_path: &str,
    ) -> DynResult<AssetIndexes> {
        let r = self.reporter.sub();
        r.set_message(format!("正在下载原版资源索引 {name}"));
        let full_path = format!("{save_path}/indexes/{name}.json");
        inner_future::fs::create_dir_all(
            &full_path[..full_path
                .rfind(std::path::is_separator)
                .unwrap_or(full_path.len())],
        )
        .await?;
        let p = {
            let url = url.parse::<url::Url>()?;
            let p = url.path();
            p.to_owned()
        };
        let uris = [
            match self.source {
                DownloadSource::Default => {
                    format!("https://launchermeta.mojang.com{p}")
                }
                DownloadSource::BMCLAPI => {
                    format!("https://bmclapi2.bangbang93.com{p}")
                }
                DownloadSource::MCBBS => format!("https://download.mcbbs.net{p}"),
                _ => format!("https://launchermeta.mojang.com{p}"),
            },
            format!("https://bmclapi2.bangbang93.com{p}"),
            format!("https://download.mcbbs.net{p}"),
            format!("https://launchermeta.mojang.com{p}"),
        ];
        for uri in &uris {
            let res = crate::http::retry_get_bytes(uri).await;
            if let Ok(res) = res {
                inner_future::fs::write(full_path, &res).await?;
                return Ok(serde_json::from_slice(&res)?);
            }
        }
        anyhow::bail!("获取素材索引失败，已尝试的链接：{}", uris.join("\n"))
    }
    async fn download_asset(
        &self,
        sha1: &str,
        name: &str,
        save_path: &str,
        is_pre: bool,
        r: Option<impl Reporter>,
    ) -> DynResult {
        let sub_hash = &sha1[..2];
        let full_path = if is_pre {
            format!("{save_path}/../virtual/pre-1.6/{name}")
        } else {
            format!("{save_path}/{sub_hash}/{sha1}")
        };

        r.set_message(format!("正在下载原版资源 {name}"));
        let l = self.parallel_lock.acquire().await;
        if if is_pre {
            Path::new(&full_path).exists()
        } else {
            is_asset_exists(sha1, save_path)
        } {
            if self.verify_data {
                let mut file = inner_future::fs::OpenOptions::new()
                    .read(true)
                    .open(&full_path)
                    .await?;
                let current_sha1 = crate::utils::get_data_sha1(&mut file).await?;
                if sha1 == current_sha1 {
                    r.add_progress(1.);
                    return Ok(());
                }
            } else {
                r.add_progress(1.);
                return Ok(());
            }
        }
        inner_future::fs::create_dir_all(
            &full_path[..full_path
                .rfind(std::path::is_separator)
                .unwrap_or(full_path.len())],
        )
        .await?;

        let uris = [
            match self.source {
                DownloadSource::Default => {
                    format!("https://resources.download.minecraft.net/{sub_hash}/{sha1}")
                }
                DownloadSource::BMCLAPI => {
                    format!("https://bmclapi2.bangbang93.com/assets/{sub_hash}/{sha1}")
                }
                DownloadSource::MCBBS => {
                    format!("https://download.mcbbs.net/assets/{sub_hash}/{sha1}")
                }
                _ => format!("https://resources.download.minecraft.net/{sub_hash}/{sha1}"),
            },
            format!("https://bmclapi2.bangbang93.com/assets/{sub_hash}/{sha1}"),
            format!("https://download.mcbbs.net/assets/{sub_hash}/{sha1}"),
            format!("https://resources.download.minecraft.net/{sub_hash}/{sha1}"),
        ];
        crate::http::download(&uris, &full_path, 0)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "下载资源文件失败 {:?}，已尝试的链接：{}",
                    e,
                    uris.join("\n")
                )
            })?;
        r.add_progress(1.);
        drop(l);
        Ok(())
    }

    async fn download_libraries(
        &self,
        libraries: &[Library],
    ) -> DynResult<HashMap<String, Vec<String>>> {
        // Libraries
        let mut _libraries_size = 0;
        let mut native_jars_mapping: HashMap<String, Vec<String>> =
            HashMap::with_capacity(libraries.len());

        let lr = self.reporter.sub();
        lr.set_message("正在检索并安装需要安装的依赖库".into());

        // Minecraft 在早期和现在对记录原生库的方式不一样
        // 早期会记录一个 `natives` 字段，键代表平台 `windows`/`osx`/`linux`
        // 值代表在 `classifiers` 字段中对应的键，如 `natives-windows`
        // 然后即可查询到对应的原生库和下载链接
        // 现在直接记录在 `name` 字段中，如 `org.lwjgl:lwjgl:3.2.2:natives-windows`
        // 故只需检测是否可以通过 `:natives-` 分割字符串即可判定是否为原生库

        // SCL 对此的原生库的下载方式为，下载所有平台和架构的原生库，并按照后缀名分别解压到对应的文件夹中

        enum LibraryTask<'a> {
            Common {
                sha1: Cow<'a, str>,
                path: Cow<'a, str>,
            },
            Native {
                platform: Cow<'a, str>,
                sha1: Cow<'a, str>,
                path: Cow<'a, str>,
            },
        }

        let mut tasks = Vec::with_capacity(libraries.len() * 2);

        for lib in libraries.iter() {
            // 遍历老版本的原生库元数据
            if let Some(downloads) = &lib.downloads {
                if let Some(classifier) = &downloads.classifiers {
                    for (platform, meta) in classifier.iter() {
                        let target_platform = match platform.as_str() {
                            "natives-osx" => "natives-macos",
                            other => other,
                        };
                        tasks.push(LibraryTask::Native {
                            platform: target_platform.into(),
                            sha1: meta.sha1.as_str().into(),
                            path: meta.path.as_str().into(),
                        });
                    }
                }
                if let Some(artifact) = &downloads.artifact {
                    if let Some(i) = lib.name.find(":natives-") {
                        let (_, platform) = lib.name.split_at(i + 1);
                        tasks.push(LibraryTask::Native {
                            platform: platform.into(),
                            sha1: artifact.sha1.as_str().into(),
                            path: artifact.path.as_str().into(),
                        });
                    } else if lib.rules.is_allowed() {
                        tasks.push(LibraryTask::Common {
                            sha1: artifact.sha1.as_str().into(),
                            path: artifact.path.as_str().into(),
                        });
                    }
                }
            }
        }

        for task in &tasks {
            if let LibraryTask::Native {
                platform,
                sha1,
                path,
            } = task
            {
                debug!("原生库 {platform} {sha1} {path}");
                let p = format!("{}/{}", self.minecraft_library_path.as_str(), path);
                if let Some(native_jars) = native_jars_mapping.get_mut(&platform.to_string()) {
                    if !native_jars.contains(&p) {
                        native_jars.push(p);
                    }
                } else {
                    let mut native_jars = Vec::with_capacity(tasks.len());
                    native_jars.push(p);
                    native_jars_mapping.insert(platform.to_string(), native_jars);
                }
            }
        }

        let libraries_threads = tasks.into_iter().map(|x| match x {
            LibraryTask::Common { sha1, path } => self.download_library(
                sha1.to_string(),
                path.to_string(),
                self.minecraft_library_path.as_str(),
            ),
            LibraryTask::Native { sha1, path, .. } => self.download_library(
                sha1.to_string(),
                path.to_string(),
                self.minecraft_library_path.as_str(),
            ),
        });

        for v in futures::future::join_all(libraries_threads).await {
            v?;
        }

        lr.remove_progress();

        Ok(native_jars_mapping)
    }

    async fn download_vanilla(
        &self,
        version_name: &str,
        version_meta: &VersionMeta,
        is_repair: bool,
    ) -> DynResult {
        info!("开始下载原版游戏 {version_name}");
        let r = self.reporter.sub();
        let game_file = format!(
            "{}/{}/{}.jar",
            self.minecraft_version_path.as_str(),
            version_name,
            version_name
        );
        r.set_message(format!("正在下载原版游戏 {version_name}"));
        let main_jar = version_meta
            .downloads
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("无法获取下载清单"))?
            .get("client")
            .ok_or_else(|| anyhow::anyhow!("无法获取客户端下载元数据"))?;
        let main_jar_thread = self.download_vanilla_jar(&main_jar.url, &game_file, &main_jar.sha1);

        if is_repair {
            let lib_path = std::path::Path::new(self.minecraft_library_path.as_str());
            let lib_path = lib_path
                .join("org")
                .join("glavo")
                .join("1.0")
                .join("log4j-patch");
            if !lib_path.is_dir() {
                inner_future::fs::create_dir_all(&lib_path).await?;
            }
            let log4j_path = lib_path.join("log4j-patch-agent-1.0.jar");
            inner_future::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&log4j_path)
                .await?
                .write_all(crate::client::LOG4J_PATCH)
                .await?;
        }

        let libraries_thread = self.download_libraries(&version_meta.libraries);

        let is_pre = &version_meta.asset_index.as_ref().unwrap().id == "pre-1.6";

        // Assets
        let assets_index = self
            .download_asset_index(
                &version_meta.asset_index.as_ref().unwrap().id,
                &version_meta.asset_index.as_ref().unwrap().url,
                self.minecraft_assets_path.as_str(),
            )
            .await?;

        let mut assets_hashes = Vec::with_capacity(assets_index.objects.len());

        let ar = r.sub();

        let minecraft_assets_objects_path = format!("{}/objects", self.minecraft_assets_path);

        let amounts = assets_index
            .objects
            .iter()
            .filter(|a| {
                if assets_hashes.contains(&a.1.hash) {
                    false
                } else {
                    assets_hashes.push(a.1.hash.to_owned());
                    true
                }
            })
            .filter(|(path, obj)| {
                if is_pre {
                    !Path::new(&minecraft_assets_objects_path)
                        .parent()
                        .unwrap()
                        .join("virtual")
                        .join("pre-1.6")
                        .join(path)
                        .exists()
                } else {
                    !is_asset_exists(&obj.hash, &minecraft_assets_objects_path)
                }
            })
            .count();

        assets_hashes.clear();

        let assets_download_tasks = assets_index
            .objects
            .iter()
            .filter(|a| {
                if assets_hashes.contains(&a.1.hash) {
                    false
                } else {
                    assets_hashes.push(a.1.hash.to_owned());
                    true
                }
            })
            .filter(|(path, obj)| {
                if is_pre {
                    !Path::new(&minecraft_assets_objects_path)
                        .parent()
                        .unwrap()
                        .join("virtual")
                        .join("pre-1.6")
                        .join(path)
                        .exists()
                } else {
                    !is_asset_exists(&obj.hash, &minecraft_assets_objects_path)
                }
            });

        ar.set_message("下载资源文件".into());
        ar.add_max_progress(amounts as _);

        let assets_index_objects = assets_download_tasks.map(|(rpath, obj)| {
            self.download_asset(
                &obj.hash,
                rpath,
                &minecraft_assets_objects_path,
                is_pre,
                ar.sub(),
            )
        });

        // Wait all threads
        let native_jars = if is_repair {
            futures::future::join(
                libraries_thread,
                futures::future::join_all(assets_index_objects),
            )
            .await
            .0?
        } else {
            futures::future::join3(
                main_jar_thread,
                libraries_thread,
                futures::future::join_all(assets_index_objects),
            )
            .await
            .1?
        };

        let native_dir = format!(
            "{}/{}/natives",
            self.minecraft_version_path.as_str(),
            version_name
        );
        let nr = r.sub();
        nr.set_max_progress(native_jars.len() as f64);
        nr.set_message("正在解压原生库".into());
        for (platform, items) in native_jars.iter() {
            let native_dir = format!("{native_dir}/{platform}");
            for item in items {
                unzip_natives(item, &native_dir).await?;
            }
            nr.add_progress(1.);
        }
        r.remove_progress();

        info!("原版游戏 {version_name} 下载完成！");
        Ok(())
    }

    async fn install_vanilla(&self, version_name: &str, version_info: &VersionInfo) -> DynResult {
        self.reporter.set_max_progress(4.);
        self.reporter
            .set_message(format!("正在获取版本元数据 {version_name}"));

        create_dir_all(format!("{}/indexes", self.minecraft_assets_path)).await?;
        create_dir_all(format!("{}/objects", self.minecraft_assets_path)).await?;
        create_dir_all(self.minecraft_library_path.as_str()).await?;
        create_dir_all(format!(
            "{}/{}",
            self.minecraft_version_path.as_str(),
            version_name
        ))
        .await?;

        let version_file = format!(
            "{}/{}/{}.json",
            self.minecraft_version_path.as_str(),
            version_name,
            version_name
        );
        let url = version_info.url.parse::<url::Url>()?;
        let url_path = url.path();

        let res = crate::http::retry_get_bytes(match self.source {
            DownloadSource::Default => format!("https://launchermeta.mojang.com{url_path}"),
            DownloadSource::BMCLAPI => format!("https://bmclapi2.bangbang93.com{url_path}"),
            DownloadSource::MCBBS => format!("https://download.mcbbs.net{url_path}"),
            _ => format!("https://launchermeta.mojang.com{url_path}"),
        })
        .await
        .map_err(|e| anyhow::anyhow!("下载版本元数据失败：{:?}", e))?;

        inner_future::fs::write(&version_file, &res).await?;

        self.reporter
            .set_message(format!("正在下载游戏文件 {version_name}"));

        let mut version_meta: VersionMeta = serde_json::from_slice(&res)?;

        version_meta.fix_libraries();

        self.download_vanilla(version_name, &version_meta, false)
            .await?;

        Ok(())
    }
}

fn is_asset_exists(hash: &str, save_path: &str) -> bool {
    let sub_hash = &hash[..2];
    let full_path = format!("{save_path}/{sub_hash}/{hash}");
    std::path::Path::new(&full_path).is_file()
}

const NATIVE_EXTS: &[&str] = &["dll", "so", "dylib", "jnilib"];

/// 解压指定 ZIP 压缩文件的内容到指定文件夹
///
/// 如果文件夹不存在则创建
pub async fn unzip_natives(unzip_file: &str, unzip_dir: &str) -> DynResult {
    let unzip_file = unzip_file.to_owned();
    let unzip_dir = unzip_dir.to_owned();
    inner_future::unblock(move || -> DynResult {
        let file = std::fs::File::open(&unzip_file)?;
        let dir = std::path::PathBuf::from(unzip_dir);
        let mut archive = zip::ZipArchive::new(file)
            .with_context(|| format!("解压原生库 {unzip_file} 时发生错误"))?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let p = match file.enclosed_name().and_then(|p| p.file_name()) {
                Some(p) => p.to_owned(),
                None => continue,
            };
            if let Some(ext) = Path::new(&p).extension() {
                if !NATIVE_EXTS.contains(&ext.to_str().unwrap_or_default()) {
                    continue;
                }
            } else {
                continue;
            }
            let save_path = dir.join(&p);
            let save_dir = save_path.parent().unwrap();
            let _ = std::fs::create_dir_all(save_dir);
            debug!(
                "解压原生库 {} 到 {}",
                p.to_string_lossy(),
                save_path.display()
            );
            let mut output = std::fs::File::create(save_path)?;
            std::io::copy(&mut file, &mut output)?;
        }
        Ok(())
    })
    .await
}
