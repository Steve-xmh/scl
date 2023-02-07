//! CurseForge 模组下载的结构和接口
//!
//! 在使用这个模块提供的功能前，请先设定好 `CURSEFORGE_API_KEY` 环境变量为你 CurseForge 的开发者令牌，否则服务将无法使用

/*
    基本链接：https://addons-ecs.forgesvc.net/api/v2/addon/
    某个模组：https://addons-ecs.forgesvc.net/api/v2/addon/[MOD_ID]
    模组详情：https://addons-ecs.forgesvc.net/api/v2/addon/[MOD_ID]/description
    模组文件：https://addons-ecs.forgesvc.net/api/v2/addon/[MOD_ID]/files
    搜索模组：https://addons-ecs.forgesvc.net/api/v2/addon/search
            请求字符串： gameId = 432
                        gameVersion
                        sectionId = 6
                        searchFilter
                        categoryID
                        index
                        pageSize
                        sort:
                            FEATURED: 0
                            POPULARITY: 1
                            LAST_UPDATE: 2
                            NAME: 3
                            AUTHOR: 4
                            TOTAL_DOWNLOADS: 5
*/

use std::{
    fmt::Write as _,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use crate::prelude::*;

const API_KEY: Option<&str> = std::option_env!("CURSEFORGE_API_KEY");
const BASE_URL: &str = "https://api.curseforge.com/v1/";
const BASE_URL_SEARCH: &str = "https://api.curseforge.com/v1/mods/search?gameId=432&classId=6";

#[derive(Debug, Deserialize)]
struct Response<T> {
    pub data: T,
}

impl<T> Deref for Response<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Response<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

/// 一个模组资源信息
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModAsset {
    /// 此模组文件的文件 ID 编号
    pub id: i32,
    /// 此模组文件对应的模组 ID
    pub mod_id: i32,
    /// 模组文件的标题（不一定是文件名）
    pub title: String,
    /// 模组文件的介绍（一般是作者的更新记录什么的）
    pub description: String,
    /// 模组文件的缩略图
    pub thumbnail_url: String,
    /// 模组文件的下载链接
    pub url: String,
}

/// 一个模组的信息
#[derive(Debug, Deserialize)]
pub struct ModInfo {
    /// 模组的 ID
    pub id: u64,
    /// 模组的名称
    pub name: String,
    /// 模组的简短介绍
    pub summary: String,
    /// 模组的 Slug（一般是模组的字符串 ID）
    pub slug: String,
    /// 模组的 LOGO 图标
    pub logo: Option<ModAsset>,
}

/// 模组的所需依赖
///
/// TODO：完善模组依赖下载功能
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    // mod_id: i32,
    // relation_type: u8,
}

/// 一个模组文件信息
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFile {
    /// 模组文件的文件名
    pub file_name: String,
    /// 模组文件的下载链接
    pub download_url: String,
    /// 模组的所需依赖
    pub dependencies: Vec<Dependency>,
    /// 模组支持的游戏版本
    pub game_versions: Vec<String>,
}

/// 使用搜索 API 时的排序方式
#[derive(Debug, Clone, Copy)]
pub enum SearchSortMethod {
    /// 按推荐排序
    Featured,
    /// 按热门度排序
    Populatity,
    /// 按最新更新排序
    LastUpdate,
    /// 按名称排序
    Name,
    /// 按作者名称排序
    Author,
    /// 按总下载量排序
    TotalDownloads,
}

impl SearchSortMethod {
    fn to_query(self) -> u8 {
        match self {
            SearchSortMethod::Featured => 0,
            SearchSortMethod::Populatity => 1,
            SearchSortMethod::LastUpdate => 2,
            SearchSortMethod::Name => 3,
            SearchSortMethod::Author => 4,
            SearchSortMethod::TotalDownloads => 5,
        }
    }
}

impl Default for SearchSortMethod {
    fn default() -> Self {
        SearchSortMethod::Featured
    }
}

/// 搜索参数，将其传入到 [`self::search_mods`] 方法以搜索模组
#[derive(Default)]
pub struct SearchParams {
    /// 搜索支持指定游戏版本的模组
    pub game_version: String,
    /// 当前的搜索页码
    pub index: u64,
    /// 当前搜索的每页项目数量
    pub page_size: u64,
    /// 模组类型 ID
    pub category_id: u64,
    /// 搜索的关键字
    pub search_filter: String,
    /// 搜索结果的排序方式
    pub sort: SearchSortMethod,
}

/// 根据关键词从 Curseforge 搜索模组列表
pub async fn search_mods(
    SearchParams {
        game_version,
        index,
        page_size,
        category_id,
        search_filter,
        sort,
    }: SearchParams,
) -> DynResult<Vec<ModInfo>> {
    let mut base_url = BASE_URL_SEARCH.to_string();
    let _ = write!(&mut base_url, "&sort={}", sort.to_query());
    if !search_filter.is_empty() {
        let _ = write!(
            &mut base_url,
            "&searchFilter={}",
            urlencoding::encode(&search_filter)
        );
    }
    if !game_version.is_empty() {
        let _ = write!(&mut base_url, "&gameVersion={game_version}");
    }
    if index > 0 {
        let _ = write!(&mut base_url, "&index={index}");
    }
    if page_size > 0 && page_size <= 30 {
        let _ = write!(&mut base_url, "&pageSize={page_size}");
    } else {
        let _ = write!(&mut base_url, "&pageSize={}", 20);
    }
    if category_id > 0 {
        let _ = write!(&mut base_url, "&categoryID={category_id}");
    }
    println!("Searching by {base_url}");
    let data: Response<Vec<ModInfo>> = crate::http::get(&base_url)
        .header("x-api-key", API_KEY.unwrap_or_default())
        .await
        .map_err(|e| anyhow::anyhow!(e))?
        .body_json()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(data.data)
}

/// 通过模组在 Curseforge 的 ID 获取详情信息
pub async fn get_mod_info(modid: u64) -> DynResult<ModInfo> {
    let data: Response<ModInfo> = crate::http::get(&(format!("{BASE_URL}mods/{modid}")))
        .header("x-api-key", API_KEY.unwrap_or_default())
        .await
        .map_err(|e| anyhow::anyhow!(e))?
        .body_json()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(data.data)
}

/// 获取模组在 Curseforge 的 ID 获取可下载的模组文件列表
pub async fn get_mod_files(modid: u64) -> DynResult<Vec<ModFile>> {
    let data: Response<Vec<ModFile>> = crate::http::get(&format!("{BASE_URL}mods/{modid}/files"))
        .header("x-api-key", API_KEY.unwrap_or_default())
        .await
        .map_err(|e| anyhow::anyhow!(e))?
        .body_json()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(data.data)
}

/// 获取模组在 Curseforge 的 ID 获取模组的图标
pub async fn get_mod_icon(mod_info: &ModInfo) -> DynResult<image::DynamicImage> {
    if let Some(logo) = &mod_info.logo {
        let data = crate::http::get(&logo.thumbnail_url)
            .await
            .map_err(|e| anyhow::anyhow!(e))?
            .body_bytes()
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        if let Ok(img) = image::load_from_memory(&data) {
            Ok(img)
        } else {
            anyhow::bail!("Can't load mod icon image")
        }
    } else {
        anyhow::bail!("Mod icon image is empty")
    }
}

/// 获取模组在 Curseforge 的 ID 获取模组的图标
pub async fn get_mod_icon_by_id(modid: u64) -> DynResult<image::DynamicImage> {
    let mod_info = get_mod_info(modid).await?;
    get_mod_icon(&mod_info).await
}

/// 下载模组
pub async fn download_mod(
    _ctx: Option<impl Reporter>,
    _name: &str,
    url: &str,
    dest: PathBuf,
) -> DynResult {
    let mut file = inner_future::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(format!("{}.tmp", dest.to_str().unwrap()))
        .await?;
    let res = crate::http::get(url)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    inner_future::io::copy(res, &mut file).await?;
    inner_future::fs::rename(format!("{}.tmp", dest.to_str().unwrap()), dest).await?;
    Ok(())
}
