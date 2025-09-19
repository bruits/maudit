use std::hash::Hash;
use std::{path::PathBuf, sync::OnceLock, time::Instant};

use base64::Engine;
use image::{GenericImageView, image_dimensions};
use log::debug;
use thumbhash::{rgba_to_thumb_hash, thumb_hash_to_average_rgba, thumb_hash_to_rgba};

use super::image_cache::ImageCache;
use crate::assets::{Asset, InternalAsset};
use crate::is_dev;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Png,
    Jpeg,
    WebP,
    Avif,
    Gif,
}

impl ImageFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::WebP => "webp",
            ImageFormat::Avif => "avif",
            ImageFormat::Gif => "gif",
        }
    }

    pub(crate) fn to_hash_value(&self) -> u32 {
        match self {
            ImageFormat::Png => 1,
            ImageFormat::Jpeg => 2,
            ImageFormat::WebP => 3,
            ImageFormat::Gif => 4,
            ImageFormat::Avif => 5,
        }
    }
}

impl From<ImageFormat> for image::ImageFormat {
    fn from(val: ImageFormat) -> Self {
        match val {
            ImageFormat::Png => image::ImageFormat::Png,
            ImageFormat::Jpeg => image::ImageFormat::Jpeg,
            ImageFormat::WebP => image::ImageFormat::WebP,
            ImageFormat::Avif => image::ImageFormat::Avif,
            ImageFormat::Gif => image::ImageFormat::Gif,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<ImageFormat>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Image {
    pub path: PathBuf,
    pub(crate) assets_dir: PathBuf,
    pub(crate) output_assets_dir: PathBuf,
    pub(crate) hash: String,
    pub(crate) options: Option<ImageOptions>,
}

impl Image {
    /// Get a placeholder for the image, which can be used for low-quality image placeholders (LQIP) or similar techniques.
    ///
    /// This uses the [ThumbHash](https://evanw.github.io/thumbhash/) algorithm to generate a very small placeholder image.
    pub fn placeholder(&self) -> ImagePlaceholder {
        get_placeholder(&self.path)
    }

    pub fn dimensions(&self) -> (u32, u32) {
        image_dimensions(&self.path).unwrap_or((0, 0))
    }
}

#[derive(Debug)]
pub struct ImagePlaceholder {
    pub thumbhash: Vec<u8>,
    pub thumbhash_base64: String,
    average_rgba_cache: OnceLock<Option<(u8, u8, u8, u8)>>,
    data_uri_cache: OnceLock<String>,
}

impl Clone for ImagePlaceholder {
    fn clone(&self) -> Self {
        Self {
            thumbhash: self.thumbhash.clone(),
            thumbhash_base64: self.thumbhash_base64.clone(),
            average_rgba_cache: OnceLock::new(),
            data_uri_cache: OnceLock::new(),
        }
    }
}

impl Default for ImagePlaceholder {
    fn default() -> Self {
        Self {
            thumbhash: Vec::new(),
            thumbhash_base64: String::new(),
            average_rgba_cache: OnceLock::new(),
            data_uri_cache: OnceLock::new(),
        }
    }
}

impl ImagePlaceholder {
    pub fn average_rgba(&self) -> Option<(u8, u8, u8, u8)> {
        *self.average_rgba_cache.get_or_init(|| {
            let start = Instant::now();
            let result = thumb_hash_to_average_rgba(&self.thumbhash)
                .ok()
                .map(|(r, g, b, a)| {
                    (
                        (r * 255.0) as u8,
                        (g * 255.0) as u8,
                        (b * 255.0) as u8,
                        (a * 255.0) as u8,
                    )
                });
            debug!("Average RGBA calculation took {:?}", start.elapsed());
            result
        })
    }

    pub fn data_uri(&self) -> &str {
        self.data_uri_cache.get_or_init(|| {
            let start = Instant::now();

            let rgba_start = Instant::now();
            let thumbhash_rgba = thumb_hash_to_rgba(&self.thumbhash).unwrap();
            debug!(
                "ThumbHash to RGBA conversion took {:?}",
                rgba_start.elapsed()
            );

            let png_start = Instant::now();
            let thumbhash_png = thumbhash_to_png(&thumbhash_rgba);
            debug!("PNG generation took {:?}", png_start.elapsed());

            let optimized_png = if is_dev() {
                thumbhash_png
            } else {
                let optimize_start = Instant::now();
                let result =
                    oxipng::optimize_from_memory(&thumbhash_png, &Default::default()).unwrap();
                debug!("PNG optimization took {:?}", optimize_start.elapsed());
                result
            };

            let encode_start = Instant::now();
            let base64 = base64::engine::general_purpose::STANDARD.encode(&optimized_png);
            let result = format!("data:image/png;base64,{}", base64);
            debug!("Data URI encoding took {:?}", encode_start.elapsed());

            debug!("Total data URI generation took {:?}", start.elapsed());
            result
        })
    }
}

fn get_placeholder(path: &PathBuf) -> ImagePlaceholder {
    // Check cache first
    if let Some(cached) = ImageCache::get_placeholder(path) {
        debug!("Using cached placeholder for {}", path.display());
        let thumbhash_base64 = base64::engine::general_purpose::STANDARD.encode(&cached.thumbhash);
        return ImagePlaceholder {
            thumbhash: cached.thumbhash,
            thumbhash_base64,
            average_rgba_cache: OnceLock::new(),
            data_uri_cache: OnceLock::new(),
        };
    }

    let total_start = Instant::now();

    let load_start = Instant::now();
    let image = image::open(path).ok().unwrap();
    let (width, height) = image.dimensions();
    let (width, height) = (width as usize, height as usize);
    debug!(
        "Image load took {:?} for {}",
        load_start.elapsed(),
        path.display()
    );

    // If width or height > 100, resize image down to max 100
    let (width, height, rgba) = if width.max(height) > 100 {
        let resize_start = Instant::now();
        let scale = 100.0 / width.max(height) as f32;
        let new_width = (width as f32 * scale).round() as usize;
        let new_height = (height as f32 * scale).round() as usize;

        let resized = image::imageops::resize(
            &image,
            new_width as u32,
            new_height as u32,
            image::imageops::FilterType::Nearest,
        );
        let result = (new_width, new_height, resized.into_raw());
        debug!(
            "Image resize took {:?} ({}x{} -> {}x{})",
            resize_start.elapsed(),
            width,
            height,
            new_width,
            new_height
        );
        result
    } else {
        let convert_start = Instant::now();
        let result = (width, height, image.to_rgba8().into_raw());
        debug!("Image RGBA conversion took {:?}", convert_start.elapsed());
        result
    };

    let thumbhash_start = Instant::now();
    let thumb_hash = rgba_to_thumb_hash(width, height, &rgba);
    debug!("ThumbHash generation took {:?}", thumbhash_start.elapsed());

    let encode_start = Instant::now();
    let thumbhash_base64 = base64::engine::general_purpose::STANDARD.encode(&thumb_hash);
    debug!("Base64 encoding took {:?}", encode_start.elapsed());

    debug!(
        "Total placeholder generation took {:?} for {}",
        total_start.elapsed(),
        path.display()
    );

    // Cache the result
    ImageCache::cache_placeholder(path, thumb_hash.clone());

    ImagePlaceholder {
        thumbhash: thumb_hash,
        thumbhash_base64,
        average_rgba_cache: OnceLock::new(),
        data_uri_cache: OnceLock::new(),
    }
}

/// Port of https://github.com/evanw/thumbhash/blob/a652ce6ed691242f459f468f0a8756cda3b90a82/js/thumbhash.js#L234
/// TODO: Do this some other way, not only is the code, well, unreadable, the result is also quite inefficient.
fn thumbhash_to_png(thumbhash_rgba: &(usize, usize, Vec<u8>)) -> Vec<u8> {
    let w = thumbhash_rgba.0 as u32;
    let h = thumbhash_rgba.1 as u32;
    let rgba = &thumbhash_rgba.2;

    let row = w * 4 + 1;
    let idat = 6 + h * (5 + row);

    let mut bytes = vec![
        137,
        80,
        78,
        71,
        13,
        10,
        26,
        10,
        0,
        0,
        0,
        13,
        73,
        72,
        68,
        82,
        0,
        0,
        (w >> 8) as u8,
        (w & 255) as u8,
        0,
        0,
        (h >> 8) as u8,
        (h & 255) as u8,
        8,
        6,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        (idat >> 24) as u8,
        ((idat >> 16) & 255) as u8,
        ((idat >> 8) & 255) as u8,
        (idat & 255) as u8,
        73,
        68,
        65,
        84,
        120,
        1,
    ];

    let table = [
        0u32, 498536548, 997073096, 651767980, 1994146192, 1802195444, 1303535960, 1342533948,
        3988292384, 4027552580, 3604390888, 3412177804, 2607071920, 2262029012, 2685067896,
        3183342108,
    ];

    let mut a = 1u32;
    let mut b = 0u32;
    let mut i = 0usize;
    let mut end = (row - 1) as usize;

    for y in 0..h {
        let filter_type = if y + 1 < h { 0 } else { 1 };
        bytes.extend_from_slice(&[
            filter_type,
            (row & 255) as u8,
            (row >> 8) as u8,
            (!row & 255) as u8,
            ((row >> 8) ^ 255) as u8,
            0,
        ]);

        b = (b + a) % 65521;
        while i < end {
            let u = rgba[i];
            bytes.push(u);
            a = (a + u as u32) % 65521;
            b = (b + a) % 65521;
            i += 1;
        }
        end += (row - 1) as usize;
    }

    bytes.extend_from_slice(&[
        (b >> 8) as u8,
        (b & 255) as u8,
        (a >> 8) as u8,
        (a & 255) as u8,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        73,
        69,
        78,
        68,
        174,
        66,
        96,
        130,
    ]);

    let ranges = [(12usize, 29usize), (37usize, 41 + idat as usize)];

    for (start, end_pos) in ranges {
        let mut c = !0u32;
        for &byte in &bytes[start..end_pos] {
            c ^= byte as u32;
            c = (c >> 4) ^ table[(c & 15) as usize];
            c = (c >> 4) ^ table[(c & 15) as usize];
        }
        c = !c;
        let mut end_idx = end_pos;
        bytes[end_idx] = (c >> 24) as u8;
        end_idx += 1;
        bytes[end_idx] = ((c >> 16) & 255) as u8;
        end_idx += 1;
        bytes[end_idx] = ((c >> 8) & 255) as u8;
        end_idx += 1;
        bytes[end_idx] = (c & 255) as u8;
    }

    bytes
}

impl InternalAsset for Image {
    fn assets_dir(&self) -> &PathBuf {
        &self.assets_dir
    }

    fn output_assets_dir(&self) -> &PathBuf {
        &self.output_assets_dir
    }
}

impl Asset for Image {
    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn hash(&self) -> String {
        self.hash.clone()
    }

    fn final_extension(&self) -> String {
        if let Some(options) = &self.options
            && let Some(format) = &options.format
        {
            format.extension()
        } else {
            self.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or_default()
        }
        .to_string()
    }
}
