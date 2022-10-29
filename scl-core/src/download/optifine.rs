//! Optifine 画质增强/性能优化模组的下载及安装模块
//!
//! 因 Optifine 并不提供一个稳定的下载方式，故此处会使用镜像源的额外 API 来获取版本下载信息

use async_trait::async_trait;
use inner_future::io::AsyncWriteExt;

use super::{structs::OptifineVersionMeta, Downloader};
use crate::{download::DownloadSource, prelude::*};

const OPTIFINE_INSTALL_HELPER: &[u8] = include_bytes!("../../assets/optifine-installer.jar");

#[cfg(target_os = "windows")]
const CLASS_PATH_SPAREATOR: &str = ";";
#[cfg(target_os = "linux")]
const CLASS_PATH_SPAREATOR: &str = ":";
#[cfg(target_os = "macos")]
const CLASS_PATH_SPAREATOR: &str = ":";

/// 一个用于下载 Optifine 模组下载安装的扩展特质，可以使用 [`crate::download::Downloader`] 来安装
#[async_trait]
pub trait OptifineDownloadExt: Sync {
    /// 根据纯净版本号获取当前可用的所有 Optifine 版本
    async fn get_avaliable_installers(
        &self,
        vanilla_version: &str,
    ) -> DynResult<Vec<OptifineVersionMeta>>;

    /// 下载 Optifine 版本
    async fn download_optifine(
        &self,
        vanilla_version: &str,
        optifine_type: &str,
        optifine_patch: &str,
        dest_path: &str,
    ) -> DynResult;

    /// 安装 Optifine 版本
    async fn install_optifine(
        &self,
        version_name: &str,
        vanilla_version: &str,
        optifine_type: &str,
        optifine_patch: &str,
        as_mod: bool,
    ) -> DynResult;
}

#[async_trait]
impl<R: Reporter> OptifineDownloadExt for Downloader<R> {
    async fn get_avaliable_installers(
        &self,
        vanilla_version: &str,
    ) -> DynResult<Vec<OptifineVersionMeta>> {
        let mut res: Vec<OptifineVersionMeta> = crate::http::retry_get_json(&match self.source {
            DownloadSource::MCBBS => {
                format!("https://download.mcbbs.net/optifine/{}", vanilla_version)
            }
            _ => format!(
                "https://bmclapi2.bangbang93.com/optifine/{}",
                vanilla_version
            ),
        })
        .await
        .map_err(|e| anyhow::anyhow!("获取可用Optifine版本列表失败：{:?}", e))?;
        res.reverse();
        Ok(res)
    }

    async fn download_optifine(
        &self,
        vanilla_version: &str,
        optifine_type: &str,
        optifine_patch: &str,
        dest_path: &str,
    ) -> DynResult {
        let r = self.reporter.sub();
        r.set_message(format!(
            "正在下载 Optifine {} {} {}",
            vanilla_version, optifine_patch, optifine_type
        ));
        let uris = [
            match self.source {
                DownloadSource::MCBBS => {
                    format!(
                        "https://download.mcbbs.net/optifine/{}/{}/{}",
                        vanilla_version, optifine_type, optifine_patch
                    )
                }
                _ => format!(
                    "https://bmclapi2.bangbang93.com/optifine/{}/{}/{}",
                    vanilla_version, optifine_type, optifine_patch
                ),
            },
            format!(
                "https://download.mcbbs.net/optifine/{}/{}/{}",
                vanilla_version, optifine_type, optifine_patch
            ),
            format!(
                "https://bmclapi2.bangbang93.com/optifine/{}/{}/{}",
                vanilla_version, optifine_type, optifine_patch
            ),
        ];
        crate::http::download(&uris, dest_path, 0).await?;
        Ok(())
    }

    async fn install_optifine(
        &self,
        version_name: &str,
        vanilla_version: &str,
        optifine_type: &str,
        optifine_patch: &str,
        as_mod: bool,
    ) -> DynResult {
        let r = self.reporter.sub();
        r.set_message(format!("正在给 {} 安装 Optifine", version_name));
        if as_mod {
            let mod_file_name = format!(
                "Optifine-{}-{}-{}.jar",
                vanilla_version, optifine_type, optifine_patch
            );
            let mod_path = if self.game_independent {
                std::path::Path::new(&self.minecraft_version_path)
                    .join(version_name)
                    .join("mods")
                    .join(mod_file_name)
            } else {
                std::path::Path::new(&self.minecraft_path)
                    .join("mods")
                    .join(mod_file_name)
            };
            inner_future::fs::create_dir_all(mod_path.parent().unwrap()).await?;
            self.download_optifine(
                vanilla_version,
                optifine_type,
                optifine_patch,
                mod_path.to_string_lossy().to_string().as_str(),
            )
            .await?;
        } else {
            // 使用安装器安装

            // 下载 Optifine
            let full_path = format!(
                "{root}/net/optifine/{mc}-{optifine_type}-{optifine_patch}/Optifine-{mc}-{optifine_type}-{optifine_patch}.jar",
                root = self.minecraft_library_path.as_str(),
                mc = vanilla_version,
                optifine_type = optifine_type,
                optifine_patch = optifine_patch,
            );
            inner_future::fs::create_dir_all(std::path::Path::new(&full_path).parent().unwrap())
                .await?;
            self.download_optifine(
                vanilla_version,
                optifine_type,
                optifine_patch,
                full_path.as_str(),
            )
            .await?;

            // Install helper
            let installer_path = format!(
                "{}/net/stevexmh/optifine-installer/0.0.0/optifine-installer.jar",
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
            file.write_all(OPTIFINE_INSTALL_HELPER).await?;
            let _ = file.flush().await;
            let _ = file.sync_all().await;

            #[cfg(not(windows))]
            let mut cmd = inner_future::process::Command::new(&self.java_path);
            #[cfg(windows)]
            let mut cmd = {
                use inner_future::process::windows::CommandExt;
                let mut cmd = inner_future::process::Command::new(&self.java_path);
                cmd.creation_flags(0x08000000);
                cmd
            };

            cmd.arg("-cp");
            cmd.arg(format!(
                "{}{}{}",
                &installer_path, CLASS_PATH_SPAREATOR, &full_path
            ));
            cmd.arg("net.stevexmh.OptifineInstaller");
            cmd.arg(self.minecraft_path.as_str()); // .minecraft
            cmd.arg(version_name); // 版本名称

            if cmd.status().await?.success() {
                return Ok(());
            } else {
                anyhow::bail!("Optifine 安装器执行失败");
            }
        }
        Ok(())
    }
}
