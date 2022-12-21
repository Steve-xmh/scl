//! Modrinth 的模组检索和下载

use image::DynamicImage;

use crate::prelude::*;

/// 一个模组搜索结果的信息
#[derive(Debug, Deserialize)]
pub struct ModResult {
    /// 这个不是真正的模组 ID，而是服务器中记录的数字 ID
    #[serde(deserialize_with = "deserialize_null_default")]
    #[serde(default)]
    pub mod_id: String,
    /// 用于短链接的模组名，大部分应该都是模组 ID
    #[serde(deserialize_with = "deserialize_null_default")]
    pub slug: String,
    /// 模组的图标链接，有可能为空
    #[serde(deserialize_with = "deserialize_null_default")]
    pub icon_url: String,
    /// 模组的标题或名称
    pub title: String,
    /// 模组的简介
    pub description: String,
}

/// 一个模组文件信息
#[derive(Debug, Deserialize)]
pub struct ModFile {
    /// 此模组文件的下载链接
    pub url: String,
    /// 此模组文件的文件名称
    pub filename: String,
    /// 是否是主要的推荐下载项目
    pub primary: bool,
}

/// 一个模组文件的信息
#[derive(Debug, Deserialize)]
pub struct ModVersion {
    /// 文件列表
    pub files: Vec<ModFile>,
    /// 模组文件支持的游戏版本
    pub game_versions: Vec<String>,
    /// 模组文件所需的模组加载器，通常是 `Forge` 或者 `Fabric`
    pub loaders: Vec<String>,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// 模组的搜索结果响应数据
#[derive(Debug, Deserialize)]
pub struct ModSearchResult {
    /// 搜索命中的模组列表
    pub hits: Vec<ModResult>,
}

/// 搜索参数，将其传入到 [`self::search_mods`] 方法以搜索模组
#[derive(Default)]
pub struct SearchParams {
    /// 搜索关键词
    pub search_filter: String,
    /// 搜索结果的页码
    pub index: u64,
    /// 搜索结果的单页项目数量，最大为 100
    pub page_size: u64,
}

/// 根据模组 ID 获取可以下载的模组文件
pub async fn get_mod_files(modid: &str) -> DynResult<Vec<ModVersion>> {
    crate::http::retry_get_json(format!(
        "https://api.modrinth.com/v2/project/{}/version",
        modid
    ))
    .await
}

/// 根据模组 ID 获取模组信息
pub async fn get_mod_info(modid: &str) -> DynResult<ModResult> {
    crate::http::retry_get_json(format!("https://api.modrinth.com/v2/project/{}", modid)).await
}

/// 根据模组 ID 获取模组图标
///
/// 如果图标不存在则返回一个 1x1 的透明像素图片
pub async fn get_mod_icon(modid: &str) -> DynResult<DynamicImage> {
    let info = get_mod_info(modid).await?;
    get_mod_icon_by_url(&info.icon_url).await
}

/// 根据模组图片直链获取模组图标
///
/// 如果图标不存在则返回一个 1x1 的透明像素图片
pub async fn get_mod_icon_by_url(url: &str) -> DynResult<DynamicImage> {
    if url.is_empty() {
        let mut img = image::RgbaImage::new(1, 1);
        img.put_pixel(0, 0, image::Rgba([0xFF, 0xFF, 0xFF, 0]));
        return Ok(image::DynamicImage::ImageRgba8(img));
    }
    let data = crate::http::get(url)
        .recv_bytes()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    // Modrinth 允许的图片格式有： .bmp .gif .jpeg .png .svg .svgz .webp .rgb
    if url.ends_with(".webp") {
        // 使用 webp 读取
        let img = webp::Decoder::new(&data)
            .decode()
            .ok_or_else(|| anyhow::anyhow!("can't load webp file"))?;
        match img.len() / (img.width() as usize * img.height() as usize) {
            3 => {
                // rgb
                let img = image::ImageBuffer::from_raw(img.width(), img.height(), img.to_owned())
                    .ok_or_else(|| anyhow::anyhow!("can't load rgb webp file"))?;
                Ok(image::DynamicImage::ImageRgb8(img))
            }
            4 => {
                // rgba
                let img = image::ImageBuffer::from_raw(img.width(), img.height(), img.to_owned())
                    .ok_or_else(|| anyhow::anyhow!("can't load rgba webp file"))?;
                Ok(image::DynamicImage::ImageRgba8(img))
            }
            _ => anyhow::bail!("unknown webp data struct"),
        }
    } else if url.ends_with(".svg") || url.ends_with(".svgz") {
        // 因为 resvg 版本冲突，故不处理
        let mut img = image::RgbaImage::new(1, 1);
        img.put_pixel(0, 0, image::Rgba([0xFF, 0xFF, 0xFF, 0]));
        Ok(image::DynamicImage::ImageRgba8(img))
    } else if let Ok(img) = image::load_from_memory_with_format(
        &data,
        image::ImageFormat::from_path(url).map_err(|e| anyhow::anyhow!(e))?,
    ) {
        Ok(img)
    } else {
        let mut img = image::RgbaImage::new(1, 1);
        img.put_pixel(0, 0, image::Rgba([0xFF, 0xFF, 0xFF, 0]));
        Ok(image::DynamicImage::ImageRgba8(img))
    }
}

/// 根据搜索参数搜索模组
pub async fn search_mods(
    SearchParams {
        search_filter,
        index,
        page_size,
    }: SearchParams,
) -> DynResult<Vec<ModResult>> {
    let search_filter = urlencoding::encode(&search_filter);
    let r: ModSearchResult = crate::http::get(format!(
        "https://api.modrinth.com/v2/search?offset={}&limit={}&query={}",
        (index - 1) * page_size,
        page_size,
        search_filter
    ))
    .recv_json()
    .await
    .map_err(|e| anyhow::anyhow!(e))?;
    Ok(r.hits
        .into_iter()
        .map(|mut a| {
            a.mod_id = a.mod_id.trim_start_matches("local-").to_owned();
            a
        })
        .collect())
}
