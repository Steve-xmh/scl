//! Forge 模组加载器的下载模块
use std::{
    io::{Read, Write},
    process::Stdio,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

use anyhow::Context;
use async_trait::async_trait;
use inner_future::io::{AsyncBufReadExt, AsyncWriteExt};
use serde_json::Value;

use super::{structs::ForgeVersionsData, Downloader};
use crate::{
    download::{
        structs::{ForgeItemInfo, ForgePromoItem},
        DownloadSource,
    },
    prelude::*,
};

const FORGE_INSTALL_HELPER: &[u8] = include_bytes!("../../assets/forge-install-bootstrapper.jar");

#[cfg(target_os = "windows")]
const CLASS_PATH_SPAREATOR: &str = ";";
#[cfg(target_os = "linux")]
const CLASS_PATH_SPAREATOR: &str = ":";
#[cfg(target_os = "macos")]
const CLASS_PATH_SPAREATOR: &str = ":";

/// Forge 模组加载器的安装特质
///
/// 可以通过引入本特质和使用 [`crate::download::Downloader`] 来安装模组加载器
#[async_trait]
pub trait ForgeDownloadExt: Sync {
    /// 根据纯净版本号获取当前可用的所有 Forge 版本
    async fn get_avaliable_installers(&self, vanilla_version: &str)
        -> DynResult<ForgeVersionsData>;
    /// 运行安装 Forge 模组加载器的预安装步骤
    ///
    /// 一般是下载各种库和依赖
    async fn install_forge_pre(
        &self,
        version_id: &str,
        vanilla_version: &str,
        forge_version: &str,
    ) -> DynResult;
    /// 运行安装 Forge 模组加载器的后安装步骤
    ///
    /// 通常是修改安装器信息，然后执行安装器完成最后的安装步骤
    async fn install_forge_post(
        &self,
        version_name: &str,
        version_id: &str,
        forge_version: &str,
    ) -> DynResult;
    /// 将安装器的部分信息进行修改，如版本名称，下载源等
    async fn modify_forge_installer(
        &self,
        from_reader: std::fs::File,
        to_writer: std::fs::File,
        name: &str,
    ) -> DynResult;
}

#[async_trait]
impl<R: Reporter> ForgeDownloadExt for Downloader<R> {
    async fn get_avaliable_installers(
        &self,
        vanilla_version: &str,
    ) -> DynResult<ForgeVersionsData> {
        let (mut versions_data, mut version_promo) = futures::future::try_join(
            crate::http::retry_get(match self.source {
                DownloadSource::BMCLAPI => {
                    format!("https://bmclapi2.bangbang93.com/forge/minecraft/{vanilla_version}")
                }
                _ => format!("https://bmclapi2.bangbang93.com/forge/minecraft/{vanilla_version}"),
            }),
            crate::http::retry_get(match self.source {
                DownloadSource::BMCLAPI => "https://bmclapi2.bangbang93.com/forge/promos",
                _ => "https://bmclapi2.bangbang93.com/forge/promos",
            }),
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "下载 Forge {} 安装器版本元数据失败：{:?}",
                vanilla_version,
                e
            )
        })?;
        let (version_promo, mut info): (Vec<ForgePromoItem>, Vec<ForgeItemInfo>) =
            futures::future::try_join(version_promo.body_json(), versions_data.body_json())
                .await
                .map_err(|e| anyhow::anyhow!(e))?;

        info.sort_by(|a, b| {
            let a: u64 = {
                let mut r = 0;
                for (i, x) in a
                    .version
                    .split('.')
                    .map(|x| str::parse::<u16>(x).unwrap_or_default())
                    .enumerate()
                {
                    r |= (x as u64) << (64 - (i + 1) * 16);
                }
                r
            };
            let b: u64 = {
                let mut r = 0;
                for (i, x) in b
                    .version
                    .split('.')
                    .map(|x| str::parse::<u16>(x).unwrap_or_default())
                    .enumerate()
                {
                    r |= (x as u64) << (64 - (i + 1) * 16);
                }
                r
            };
            a.cmp(&b)
        });

        let recommended_version = version_promo
            .iter()
            .find(|a| a.name == format!("{vanilla_version}-recommended"))
            .and_then(|a| a.build.to_owned());
        let latest_version = version_promo
            .iter()
            .find(|a| a.name == format!("{vanilla_version}-latest"))
            .and_then(|a| a.build.to_owned());

        info.reverse(); // 调转顺序，从最新的开始
        Ok(ForgeVersionsData {
            recommended: recommended_version,
            latest: latest_version,
            all_versions: info,
        })
    }

    async fn install_forge_pre(
        &self,
        version_id: &str,
        vanilla_version: &str,
        forge_version: &str,
    ) -> DynResult {
        let r = self.reporter.fork();

        if vanilla_version
            .parse::<crate::semver::MinecraftVersion>()
            .map(|x| x.should_forge_use_override_installiation())
            .unwrap_or(false)
        {
            // 旧版本使用覆盖方式安装，下载其覆盖用 zip 压缩文件
            // should_forge_use_client_or_universal
            // 旧版本均使用 4 组版本号编号，所以不需要特判

            let client_or_universal = vanilla_version
                .parse::<crate::semver::MinecraftVersion>()
                .map(|x| x.should_forge_use_client_or_universal())
                .unwrap_or(false);

            let suffix = if client_or_universal {
                "client" // https://maven.minecraftforge.net/net/minecraftforge/forge/1.2.5-3.4.9.171/forge-1.2.5-3.4.9.171-client.zip
            } else {
                "universal" // https://maven.minecraftforge.net/net/minecraftforge/forge/1.3.2-4.3.5.318/forge-1.3.2-4.3.5.318-universal.zip
            };

            let full_path = format!(
                "{root}/net/minecraftforge/forge/{mc}-{forge}/forge-{mc}-{forge}-{suffix}.zip",
                root = self.minecraft_library_path.as_str(),
                mc = vanilla_version,
                forge = forge_version,
                suffix = suffix,
            );

            if std::path::Path::new(&full_path).is_file() {
                return Ok(());
            }
            inner_future::fs::create_dir_all(
                &full_path[..full_path.rfind('/').unwrap_or(full_path.len())],
            )
            .await?;

            r.set_message(format!("下载 Forge 安装覆盖包 {forge_version}"));
            r.add_max_progress(1.);

            let uris = [
                match self.source {
                    DownloadSource::Default => format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-{suffix}.zip"),
                    DownloadSource::BMCLAPI => format!("https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-{suffix}.zip"),
                    DownloadSource::MCBBS => format!("https://download.mcbbs.net/maven/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-{suffix}.zip"),
                    _ => format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-{suffix}.zip")
                },
                format!("https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-{suffix}.zip"),
                format!("https://download.mcbbs.net/maven/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-{suffix}.zip"),
                format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-{suffix}.zip"),
            ];

            crate::http::download(&uris, &full_path, 0)
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "下载 Forge {}-{} 安装覆盖包失败：{:?}",
                        version_id,
                        forge_version,
                        e
                    )
                })?;
        } else {
            let full_path = format!(
                "{root}/net/minecraftforge/forge/{mc}-{forge}/forge-{mc}-{forge}-installer.jar",
                root = self.minecraft_library_path.as_str(),
                mc = vanilla_version,
                forge = forge_version
            );
            tracing::trace!("Downloading Forge Installer {full_path}");
            if std::path::Path::new(&full_path).is_file() {
                return Ok(());
            }
            inner_future::fs::create_dir_all(
                &full_path[..full_path.rfind('/').unwrap_or(full_path.len())],
            )
            .await?;

            r.set_message(format!("下载 Forge 安装器 {forge_version}"));
            r.add_max_progress(1.);

            let build_id =
                &forge_version[forge_version.rfind('.').map(|x| x + 1).unwrap_or_default()..];

            let uris = if forge_version.split('.').count() == 3 {
                [
                    match self.source {
                        DownloadSource::Default => format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                        DownloadSource::BMCLAPI => format!("https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                        DownloadSource::MCBBS => format!("https://download.mcbbs.net/maven/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                        _ => format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar")
                    },
                    format!("https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                    format!("https://download.mcbbs.net/maven/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                    format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                ]
            } else {
                [
                    match self.source {
                        DownloadSource::Default => format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                        DownloadSource::BMCLAPI => format!("https://bmclapi2.bangbang93.com/forge/download/{build_id}"),
                        DownloadSource::MCBBS => format!("https://download.mcbbs.net/forge/download/{build_id}"),
                        _ => format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar")
                    },
                    format!("https://bmclapi2.bangbang93.com/forge/download/{build_id}"),
                    format!("https://download.mcbbs.net/forge/download/{build_id}"),
                    format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{vanilla_version}-{forge_version}/forge-{vanilla_version}-{forge_version}-installer.jar"),
                ]
            };

            crate::http::download(&uris, &full_path, 0)
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "下载 Forge {}-{} 安装器失败：{:?}",
                        version_id,
                        forge_version,
                        e
                    )
                })?;
        }

        r.add_progress(1.);
        Ok(())
    }

    async fn install_forge_post(
        &self,
        version_name: &str,
        version_id: &str,
        forge_version: &str,
    ) -> DynResult {
        let r = self.reporter.fork();

        // 1.5.2 以前的版本都使用覆盖 JAR 的方式安装 Forge
        if version_id
            .parse::<crate::semver::MinecraftVersion>()
            .map(|x| x.should_forge_use_override_installiation())
            .unwrap_or(false)
        {
            let client_or_universal = version_id
                .parse::<crate::semver::MinecraftVersion>()
                .map(|x| x.should_forge_use_client_or_universal())
                .unwrap_or(false);

            let suffix = if client_or_universal {
                "client"
            } else {
                "universal"
            };

            let full_path = format!(
                "{root}/net/minecraftforge/forge/{mc}-{forge}/forge-{mc}-{forge}-{suffix}.zip",
                root = self.minecraft_library_path.as_str(),
                mc = version_id,
                forge = forge_version,
                suffix = suffix,
            );

            let full_override_path = format!(
                "{root}/{version_name}/{version_name}.jar",
                root = self.minecraft_version_path.as_str(),
                version_name = version_name,
            );

            let full_temp_override_path = format!(
                "{root}/{version_name}/{version_name}.tmp.jar",
                root = self.minecraft_version_path.as_str(),
                version_name = version_name,
            );

            inner_future::unblock(move || -> DynResult {
                let full_file = std::fs::OpenOptions::new().read(true).open(&full_path)?;
                let mut full_file = zip::ZipArchive::new(full_file)?;

                let override_file = std::fs::OpenOptions::new()
                    .read(true)
                    .open(&full_override_path)?;
                let mut override_file = zip::ZipArchive::new(override_file)?;

                let temp_override_file = std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&full_temp_override_path)?;
                let mut temp_override_file = zip::ZipWriter::new(temp_override_file);

                for index in 0..override_file.len() {
                    if let Ok(entry) = override_file.by_index(index) {
                        if entry.name().starts_with("META-INF") {
                            continue;
                        } else if entry.is_file() {
                            temp_override_file.raw_copy_file(entry)?;
                        } else if entry.is_dir() {
                            // tracing::trace!("Added dir {}", entry.name());
                            temp_override_file.add_directory(entry.name(), Default::default())?;
                        }
                    }
                }

                for index in 0..full_file.len() {
                    if let Ok(entry) = full_file.by_index(index) {
                        if entry.name().starts_with("META-INF") {
                            continue;
                        } else if entry.is_file() {
                            temp_override_file.raw_copy_file(entry)?;
                        } else if entry.is_dir() {
                            // tracing::trace!("Added dir {}", entry.name());
                            temp_override_file.add_directory(entry.name(), Default::default())?;
                        }
                    }
                }

                temp_override_file.finish()?.sync_all()?;

                std::fs::remove_file(&full_override_path)?;
                std::fs::rename(full_temp_override_path, full_override_path)?;

                Ok(())
            })
            .await?;

            Ok(())
        } else {
            // 新版本均使用安装器安装

            // Install helper
            let installer_path = format!(
                "{}/com/bangbang93/forge-install-bootstrapper/0.0.0/forge-install-bootstrapper.jar",
                self.minecraft_library_path.as_str()
            );

            inner_future::fs::create_dir_all(
                std::path::Path::new(&installer_path).parent().unwrap(),
            )
            .await?;
            let mut file = inner_future::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&installer_path)
                .await?;
            file.write_all(FORGE_INSTALL_HELPER).await?;
            let _ = file.flush().await;
            let _ = file.sync_all().await;

            // TODO: 换成自己的安装代码
            // Run installer
            let full_path = format!(
                "{root}/net/minecraftforge/forge/{mc}-{forge}/forge-{mc}-{forge}-installer.jar",
                root = self.minecraft_library_path.as_str(),
                mc = version_id,
                forge = forge_version
            );
            let tmp_full_path = format!(
        "{root}/net/minecraftforge/forge/{mc}-{forge}/forge-{mc}-{forge}-installer.tmp.{tempid}.jar",
        root = self.minecraft_library_path.as_str(),
        mc = version_id,
        forge = forge_version,
        tempid = std::time::SystemTime::now().elapsed().unwrap_or_default().as_secs()
    );
            tracing::trace!("Writing temp forge installer from {full_path} to {tmp_full_path}");
            {
                let version_name = version_name.to_owned();
                let full_path = full_path.to_owned();
                let tmp_full_path = tmp_full_path.to_owned();
                let full_path_c = full_path.to_owned();
                let tmp_full_path_c = tmp_full_path.to_owned();
                let (from_file, to_file) = futures::future::try_join(
                    inner_future::unblock(move || {
                        std::fs::OpenOptions::new().read(true).open(full_path)
                    }),
                    inner_future::unblock(move || {
                        std::fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(tmp_full_path)
                    }),
                )
                .await?;
                tracing::trace!("Modifying");
                self.modify_forge_installer(from_file, to_file, &version_name)
                    .await
                    .with_context(|| {
                        format!(
                            "修改 Forge 模组安装器文件 {full_path_c} 到 {tmp_full_path_c} 时发生错误"
                        )
                    })?;
            }

            r.add_max_progress(2.);
            r.set_message("正在修改安装器参数".into());

            #[cfg(not(windows))]
            let mut cmd = inner_future::process::Command::new(&self.java_path);
            #[cfg(windows)]
            let mut cmd = {
                use inner_future::process::windows::CommandExt;
                let mut cmd = inner_future::process::Command::new(&self.java_path);
                cmd.creation_flags(0x08000000);
                cmd
            };

            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::inherit());
            cmd.stdin(Stdio::null());

            cmd.arg("-cp");
            cmd.arg(format!(
                "{}{}{}",
                &installer_path, CLASS_PATH_SPAREATOR, &tmp_full_path
            ));
            cmd.arg("com.bangbang93.ForgeInstaller");
            cmd.arg(self.minecraft_path.as_str()); // 安装位置

            r.add_progress(1.);
            r.set_message("运行 Forge 安装器安装 Forge".into());

            tracing::trace!("Start running installer bootstrapper {cmd:?}");

            let mut child = cmd.spawn()?;
            let install_succeed = AtomicBool::new(false);

            let ir = r.fork();
            ir.set_message("这需要一点时间……".into());
            let pr = r.fork();

            let mut delay_timer = Instant::now();

            if let Some(stdout) = child.stdout.take() {
                let mut stdout = inner_future::io::BufReader::new(stdout);
                let mut buf = String::with_capacity(256);
                loop {
                    if let Ok(len) = stdout.read_line(&mut buf).await {
                        if len == 0 {
                            break;
                        } else {
                            let line = buf[..len].trim();

                            let delayed = delay_timer.elapsed() > Duration::from_millis(16);

                            if line.starts_with("Patching ") {
                                // 数量太多可以缓一缓
                                if delayed {
                                    pr.set_message(line.to_owned());
                                }
                            } else if delayed {
                                pr.set_message(line.to_owned());
                            }
                            tracing::trace!("[FIB] {line}");

                            if let Some(class_name) = line.strip_prefix("Patching ") {
                                // 数量太多可以缓一缓
                                if delayed {
                                    ir.set_message(format!("正在修补类 {class_name}"));
                                }
                            } else if let Some(url) = line.strip_prefix("Downloading library from ")
                            {
                                ir.set_message(format!("正在下载依赖 {url}"));
                            } else if let Some(url) = line.strip_prefix("Following redirect: ") {
                                ir.set_message(format!("下载重定向至 {url}"));
                            } else if let Some(class_name) = line.strip_prefix("Reading patch ") {
                                ir.set_message(format!("正在读取修补信息 {class_name}"));
                            } else if line == "Task: DOWNLOAD_MOJMAPS" {
                                ir.set_message("正在下载源码对照表".into());
                            } else if line == "Task: MERGE_MAPPING" {
                                ir.set_message("正在合并源码对照表".into());
                            } else if line == "Injecting profile" {
                                ir.set_message("正在注入版本元数据".into());
                            } else if line == "true" {
                                install_succeed.store(true, std::sync::atomic::Ordering::SeqCst)
                            }

                            if delayed {
                                delay_timer = Instant::now();
                            }

                            buf.clear()
                        }
                    }
                }
            }

            drop(ir);
            drop(pr);

            let status = child.status().await?;
            r.add_progress(1.);
            r.remove_progress();
            inner_future::fs::remove_file(tmp_full_path).await?;
            if status.success() && install_succeed.load(std::sync::atomic::Ordering::SeqCst) {
                Ok(())
            } else {
                anyhow::bail!(
                    "执行 Forge {}-{} 安装器失败，运行器返回值：{}",
                    version_id,
                    forge_version,
                    status
                )
            }
        }
    }

    async fn modify_forge_installer(
        &self,
        from_reader: std::fs::File,
        to_writer: std::fs::File,
        name: &str,
    ) -> DynResult {
        tracing::trace!("Opening file");
        let mut file = zip::ZipArchive::new(std::io::BufReader::new(from_reader))
            .context("打开 Forge 安装器时发生错误")?;
        tracing::trace!("Opening file");
        let mut out_file = zip::ZipWriter::new(to_writer);
        tracing::trace!("Reading file");
        for index in 0..file.len() {
            if let Ok(mut entry) = file.by_index(index) {
                if entry.name().starts_with("META-INF") {
                    continue;
                }
                if entry.is_file() {
                    // tracing::trace!("Writting file {}", entry.name());
                    match entry.name() {
                        "install_profile.json" => {
                            let mut data = String::with_capacity(entry.size() as usize);
                            entry.read_to_string(&mut data)?;
                            let mut install_profile: Value = serde_json::from_str(&data)?;
                            if let Value::Object(obj) = &mut install_profile {
                                if let Some(Value::String(version)) = obj.get_mut("version") {
                                    *version = name.to_owned();
                                    tracing::trace!("已修改 version 字段为 {version}");
                                }
                                if let Some(Value::Object(obj)) = obj.get_mut("install") {
                                    if let Some(Value::String(target)) = obj.get_mut("target") {
                                        *target = name.to_owned();
                                        tracing::trace!("已修改 install.target 字段为 {target}");
                                    }
                                }
                                let replace_source = match self.source {
                                    DownloadSource::BMCLAPI => {
                                        "https://bmclapi2.bangbang93.com/maven"
                                    }
                                    DownloadSource::MCBBS => "https://download.mcbbs.net/maven",
                                    _ => "https://files.minecraftforge.net",
                                };
                                if let Some(Value::Array(array)) = obj.get_mut("libraries") {
                                    for (i, lib) in array.iter_mut().enumerate() {
                                        if let Value::Object(obj) = lib {
                                            obj.remove("serverreq");
                                            obj.insert("clientreq".into(), Value::Bool(true));
                                            if let Some(Value::Object(obj)) =
                                                obj.get_mut("downloads")
                                            {
                                                if let Some(Value::Object(obj)) =
                                                    obj.get_mut("artifact")
                                                {
                                                    if let Some(Value::String(down_url)) =
                                                        obj.get_mut("url")
                                                    {
                                                        if let Some(down_path) = down_url
                                                            .strip_prefix(
                                                                "https://maven.minecraftforge.net",
                                                            )
                                                        {
                                                            *down_url = format!(
                                                                "{replace_source}{down_path}"
                                                            );
                                                            tracing::trace!(
                                                                "已修改 libraries[{i}].download.artifact.url 字段"
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                            if let Some(Value::String(down_url)) =
                                                obj.get_mut("url")
                                            {
                                                if let Some(down_path) = down_url.strip_prefix(
                                                    "https://maven.minecraftforge.net",
                                                ) {
                                                    *down_url =
                                                        format!("{replace_source}{down_path}");
                                                    tracing::trace!(
                                                        "已修改 libraries[{i}].url 字段"
                                                    );
                                                }
                                                if let Some(down_path) = down_url.strip_prefix(
                                                    "https://files.minecraftforge.net",
                                                ) {
                                                    *down_url =
                                                        format!("{replace_source}{down_path}");
                                                    tracing::trace!(
                                                        "已修改 libraries[{i}].url 字段"
                                                    );
                                                }
                                            }
                                        }
                                        // https://files.minecraftforge.net/
                                    }
                                }
                                // 1.12.2 之前的镜像源
                                if let Some(Value::Object(obj)) = obj.get_mut("versionInfo") {
                                    if let Some(Value::Array(array)) = obj.get_mut("libraries") {
                                        for (i, lib) in array.iter_mut().enumerate() {
                                            if let Value::Object(obj) = lib {
                                                obj.remove("serverreq");
                                                obj.insert("clientreq".into(), Value::Bool(true));
                                                if let Some(Value::Object(obj)) =
                                                    obj.get_mut("downloads")
                                                {
                                                    if let Some(Value::Object(obj)) =
                                                        obj.get_mut("artifact")
                                                    {
                                                        if let Some(Value::String(down_url)) =
                                                            obj.get_mut("url")
                                                        {
                                                            if let Some(down_path) = down_url
                                                                .strip_prefix(
                                                                "https://maven.minecraftforge.net",
                                                            ) {
                                                                *down_url = format!(
                                                                    "{replace_source}{down_path}"
                                                                );
                                                                tracing::trace!(
                                                                    "已修改 libraries[{i}].download.artifact.url 字段"
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                                if let Some(Value::String(down_url)) =
                                                    obj.get_mut("url")
                                                {
                                                    if let Some(down_path) = down_url.strip_prefix(
                                                        "https://maven.minecraftforge.net/",
                                                    ) {
                                                        *down_url =
                                                            format!("{replace_source}{down_path}");
                                                        tracing::trace!("已修改 versionInfo.libraries[{i}].url 字段");
                                                    }
                                                    if let Some(down_path) = down_url.strip_prefix(
                                                        "https://files.minecraftforge.net",
                                                    ) {
                                                        *down_url =
                                                            format!("{replace_source}{down_path}");
                                                        tracing::trace!("已修改 versionInfo.libraries[{i}].url 字段");
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            #[cfg(debug_assertions)]
                            tracing::trace!(
                                "修改完毕：\n{}",
                                serde_json::to_string_pretty(&install_profile)?
                            );
                            let output = serde_json::to_vec_pretty(&install_profile)?;
                            out_file.start_file(entry.name(), Default::default())?;
                            out_file.write_all(&output)?;
                        }
                        _ => {
                            // tracing::trace!("Copied file {}", entry.name());
                            out_file.raw_copy_file(entry)?
                        }
                    }
                } else if entry.is_dir() {
                    // tracing::trace!("Added dir {}", entry.name());
                    out_file.add_directory(entry.name(), Default::default())?;
                }
            }
        }
        let mut to_writer = out_file.finish()?;
        let _ = to_writer.flush();
        let _ = to_writer.sync_all();
        // std::io::copy(&mut from_reader, &mut to_writer)?;
        // to_writer.sync_all()?;
        Ok(())
    }
}
