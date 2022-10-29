//! Fabric 下载源数据结构
use async_trait::async_trait;
use serde::Deserialize;

use super::{DownloadSource, Downloader};
use crate::{package::PackageName, prelude::*, version::structs::VersionMeta};

/// Fabric 加载器的版本元数据和其源码对照表的版本元数据
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct LoaderMetaItem {
    /// Fabric 加载器的元数据
    pub loader: LoaderStruct,
    /// 源码对照表的元数据
    pub intermediary: IntermediaryStruct,
}

/// 源码对照表的信息
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct IntermediaryStruct {
    /// 源码对照表对应的 Maven 仓库文件链接
    pub maven: String,
    /// 源码对照表的版本
    pub version: String,
    /// 是否是稳定版本
    pub stable: bool,
}

/// Fabric 加载器的信息
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct LoaderStruct {
    /// Fabric 加载器对应的 Maven 仓库文件链接
    pub maven: String,
    /// Fabric 加载器的版本
    pub version: String,
    /// 是否是稳定版本
    pub stable: bool,
}

/// Fabric 模组加载器的安装特质
///
/// 可以通过引入本特质和使用 [`crate::download::Downloader`] 来安装模组加载器
#[async_trait]
pub trait FabricDownloadExt: Sync {
    /// 根据原版版本号获取该版本下可用的 Fabric 模组加载器
    async fn get_avaliable_loaders(&self, vanilla_version: &str) -> DynResult<Vec<LoaderMetaItem>>;
    // http://resources.download.minecraft.net
    /// 下载 Fabric 模组加载器所需的库文件
    async fn download_library(&self, name: &str) -> DynResult;
    /// 下载 Fabric 模组加载器，安装时需要配合 download_fabric_post 来合并版本元数据
    async fn download_fabric_pre(
        &self,
        version_name: &str,
        version_id: &str,
        loader_version: &str,
    ) -> DynResult;
    /// 将 Fabric 模组加载器的版本元数据与原版版本元数据合并以完成最后安装步骤
    async fn download_fabric_post(&self, version_name: &str) -> DynResult;
}

#[async_trait]
impl<R: Reporter> FabricDownloadExt for Downloader<R> {
    async fn get_avaliable_loaders(&self, vanilla_version: &str) -> DynResult<Vec<LoaderMetaItem>> {
        let mut result = crate::http::retry_get(match self.source {
            DownloadSource::Default => format!(
                "https://meta.fabricmc.net/v2/versions/loader/{}",
                vanilla_version
            ),
            DownloadSource::BMCLAPI => format!(
                "https://bmclapi2.bangbang93.com/fabric-meta/v2/versions/loader/{}",
                vanilla_version
            ),
            DownloadSource::MCBBS => format!(
                "https://download.mcbbs.net/fabric-meta/v2/versions/loader/{}",
                vanilla_version
            ),
            _ => format!(
                "https://meta.fabricmc.net/v2/versions/loader/{}",
                vanilla_version
            ),
        })
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "获取为原版 {} 可用的 Fabric Loader 版本失败 {:?}",
                vanilla_version,
                e
            )
        })?;
        if result.status().is_success() {
            let result = result.body_json().await.map_err(|e| anyhow::anyhow!(e))?;
            Ok(result)
        } else {
            Ok(vec![])
        }
    }

    async fn download_library(&self, name: &str) -> DynResult {
        let package_name = name.parse::<PackageName>().unwrap();
        let full_path = package_name.to_maven_jar_path(self.minecraft_library_path.as_str());
        let r = self.reporter.sub();
        inner_future::fs::create_dir_all(
            &full_path[..full_path.rfind('/').unwrap_or(full_path.len())],
        )
        .await
        .unwrap_or_default();
        r.set_message(format!("正在下载 Fabric 支持库 {}", name));
        r.add_max_progress(1.);
        if std::path::Path::new(&full_path).is_file() {
            if self.verify_data {
                let mut file = inner_future::fs::OpenOptions::new()
                    .read(true)
                    .open(&full_path)
                    .await?;
                r.set_message(format!("正在获取数据摘要以验证完整性 {}", name));
                r.add_max_progress(1.);
                let sha1 = crate::http::retry_get_string(format!(
                    "{}.sha1",
                    package_name.to_maven_jar_path(match self.source {
                        DownloadSource::BMCLAPI => "https://bmclapi2.bangbang93.com/maven",
                        DownloadSource::MCBBS => "https://download.mcbbs.net/maven",
                        _ => "https://maven.fabricmc.net",
                    })
                ))
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
        let uris = [
            package_name.to_maven_jar_path(match self.source {
                DownloadSource::BMCLAPI => "https://bmclapi2.bangbang93.com/maven",
                DownloadSource::MCBBS => "https://download.mcbbs.net/maven",
                _ => "https://maven.fabricmc.net",
            }),
            package_name.to_maven_jar_path("https://bmclapi2.bangbang93.com/maven"),
            package_name.to_maven_jar_path("https://download.mcbbs.net/maven"),
            package_name.to_maven_jar_path("https://maven.fabricmc.net"),
        ];
        crate::http::download(&uris, &full_path, 0)
            .await
            .map_err(|e| anyhow::anyhow!("下载 Fabric 依赖库失败：{:?}", e))?;
        Ok(())
    }

    async fn download_fabric_pre(
        &self,
        version_name: &str,
        version_id: &str,
        loader_version: &str,
    ) -> DynResult {
        let mut loader_meta_res = crate::http::retry_get(format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
            version_id, loader_version
        ))
        .await
        .map_err(|e| anyhow::anyhow!("获取 Fabric 版本元数据失败：{:?}", e))?;
        let res = loader_meta_res
            .body_bytes()
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        inner_future::fs::write(
            format!(
                "{}/{}/{}-fabric-loader.tmp.json",
                self.minecraft_version_path.as_str(),
                version_name,
                version_name
            ),
            &res,
        )
        .await?;
        let meta: VersionMeta = serde_json::from_slice(&res)?;
        let mut libraries_threads = Vec::with_capacity(meta.libraries.len());

        for lib in &meta.libraries {
            if !lib.name.is_empty() {
                libraries_threads.push(self.download_library(&lib.name));
            }
        }

        futures::future::try_join_all(libraries_threads).await?;
        // net.fabricmc:sponge-mixin:0.9.2+mixin.0.8.2
        // https://maven.fabricmc.net/net/fabricmc/sponge-mixin/0.9.2+mixin.0.8.2/sponge-mixin-0.9.2+mixin.0.8.2.jar

        Ok(())
    }

    async fn download_fabric_post(&self, version_name: &str) -> DynResult {
        // 将元数据与加载器的元数据进行合并

        let vanilla_path = format!(
            "{}/{}/{}.json",
            self.minecraft_version_path.as_str(),
            version_name,
            version_name
        );
        let vanilla_meta = crate::prelude::inner_future::fs::read(&vanilla_path).await?;
        let loader_path = format!(
            "{}/{}/{}-fabric-loader.tmp.json",
            self.minecraft_version_path.as_str(),
            version_name,
            version_name
        );
        let loader_meta = crate::prelude::inner_future::fs::read(&loader_path).await?;
        inner_future::fs::remove_file(loader_path).await?;

        let mut vanilla_meta: VersionMeta = serde_json::from_slice(&vanilla_meta)?;
        let loader_meta: VersionMeta = serde_json::from_slice(&loader_meta)?;

        vanilla_meta += loader_meta;
        inner_future::fs::write(&vanilla_path, serde_json::to_vec(&vanilla_meta)?).await?;

        Ok(())
    }
}
