//! QuiltMC 下载源数据结构
use anyhow::Context;
use async_trait::async_trait;
use serde::Deserialize;

use super::Downloader;
use crate::{package::PackageName, prelude::*, version::structs::VersionMeta};

/// QuiltMC 加载器的版本元数据和其源码对照表的版本元数据
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct LoaderMetaItem {
    /// QuiltMC 加载器的元数据
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
}

/// QuiltMC 加载器的信息
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct LoaderStruct {
    /// QuiltMC 加载器对应的 Maven 仓库文件链接
    pub maven: String,
    /// QuiltMC 加载器的版本
    pub version: String,
}

/// QuiltMC 模组加载器的安装特质
///
/// 可以通过引入本特质和使用 [`crate::download::Downloader`] 来安装模组加载器
#[async_trait]
pub trait QuiltMCDownloadExt: Sync {
    /// 根据原版版本号获取该版本下可用的 QuiltMC 模组加载器
    async fn get_avaliable_loaders(&self, vanilla_version: &str) -> DynResult<Vec<LoaderMetaItem>>;
    // http://resources.download.minecraft.net
    /// 下载 QuiltMC 模组加载器所需的库文件
    async fn download_library(&self, name: &str, url: String) -> DynResult;
    /// 下载 QuiltMC 模组加载器，安装时需要配合 download_quiltmc_post 来合并版本元数据
    async fn download_quiltmc_pre(
        &self,
        version_name: &str,
        version_id: &str,
        loader_version: &str,
    ) -> DynResult;
    /// 将 QuiltMC 模组加载器的版本元数据与原版版本元数据合并以完成最后安装步骤
    async fn download_quiltmc_post(&self, version_name: &str) -> DynResult;
}

#[async_trait]
impl<R: Reporter> QuiltMCDownloadExt for Downloader<R> {
    async fn get_avaliable_loaders(&self, vanilla_version: &str) -> DynResult<Vec<LoaderMetaItem>> {
        let mut result = crate::http::retry_get(format!(
            "https://meta.quiltmc.org/v3/versions/loader/{vanilla_version}"
        ))
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "获取为原版 {} 可用的 QuiltMC Loader 版本失败 {:?}",
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

    async fn download_library(&self, name: &str, url: String) -> DynResult {
        let url = if url.is_empty() {
            "https://maven.quiltmc.org/repository/release"
        } else {
            url.trim_end_matches('/')
        };
        let package_name = name.parse::<PackageName>().unwrap();
        let full_path = package_name.to_maven_jar_path(self.minecraft_library_path.as_str());
        let r = self.reporter.sub();
        inner_future::fs::create_dir_all(
            &full_path[..full_path.rfind('/').unwrap_or(full_path.len())],
        )
        .await
        .unwrap_or_default();
        r.set_message(format!("正在下载 QuiltMC 支持库 {name}"));
        r.add_max_progress(1.);
        if std::path::Path::new(&full_path).is_file() {
            if self.verify_data {
                let mut file = inner_future::fs::OpenOptions::new()
                    .read(true)
                    .open(&full_path)
                    .await?;
                r.set_message(format!("正在获取数据摘要以验证完整性 {name}"));
                r.add_max_progress(1.);
                let sha1 = crate::http::retry_get_string(format!(
                    "{}.sha1",
                    package_name.to_maven_jar_path(url)
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
        let uris = [package_name.to_maven_jar_path(url)];
        crate::http::download(&uris, &full_path, 0)
            .await
            .map_err(|e| anyhow::anyhow!("下载 QuiltMC 依赖库失败：{:?}", e))?;
        Ok(())
    }

    async fn download_quiltmc_pre(
        &self,
        version_name: &str,
        version_id: &str,
        loader_version: &str,
    ) -> DynResult {
        let mut loader_meta_res = crate::http::retry_get(format!(
            "https://meta.quiltmc.org/v3/versions/loader/{version_id}/{loader_version}/profile/json"
        ))
        .await
        .map_err(|e| anyhow::anyhow!("获取 QuiltMC 版本元数据失败：{:?}", e))?;
        let res = loader_meta_res
            .body_bytes()
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        inner_future::fs::write(
            format!(
                "{}/{}/{}-quiltmc-loader.tmp.json",
                self.minecraft_version_path.as_str(),
                version_name,
                version_name
            ),
            &res,
        )
        .await?;
        let meta: VersionMeta =
            serde_json::from_slice(&res).context("无法解析 QuiltMC 版本元数据")?;
        let mut libraries_threads = Vec::with_capacity(meta.libraries.len());

        for lib in &meta.libraries {
            if !lib.name.is_empty() {
                let url = lib.url.as_ref().cloned().unwrap_or_default();
                libraries_threads.push(self.download_library(&lib.name, url));
            }
        }

        futures::future::try_join_all(libraries_threads).await?;

        Ok(())
    }

    async fn download_quiltmc_post(&self, version_name: &str) -> DynResult {
        // 将元数据与加载器的元数据进行合并
        tracing::trace!("合并元数据中");

        let vanilla_path = format!(
            "{}/{}/{}.json",
            self.minecraft_version_path.as_str(),
            version_name,
            version_name
        );
        let vanilla_meta = crate::prelude::inner_future::fs::read(&vanilla_path).await?;
        let loader_path = format!(
            "{}/{}/{}-quiltmc-loader.tmp.json",
            self.minecraft_version_path.as_str(),
            version_name,
            version_name
        );
        let loader_meta = crate::prelude::inner_future::fs::read(&loader_path).await?;
        inner_future::fs::remove_file(loader_path).await?;

        let mut vanilla_meta: VersionMeta =
            serde_json::from_slice(&vanilla_meta).context("无法解析原版版本元数据")?;
        let loader_meta: VersionMeta =
            serde_json::from_slice(&loader_meta).context("无法解析 QuiltMC 版本元数据")?;

        vanilla_meta += loader_meta;
        inner_future::fs::write(&vanilla_path, serde_json::to_vec(&vanilla_meta)?).await?;

        Ok(())
    }
}
