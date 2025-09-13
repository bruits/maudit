use std::hash::Hash;
use std::{path::PathBuf, sync::OnceLock};

use base64::Engine;
use image::GenericImageView;
use thumbhash::{rgba_to_thumb_hash, thumb_hash_to_average_rgba, thumb_hash_to_rgba};

use crate::assets::{Asset, InternalAsset};
use crate::is_dev;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
    Avif,
    Gif,
}

impl ImageFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Webp => "webp",
            ImageFormat::Avif => "avif",
            ImageFormat::Gif => "gif",
        }
    }

    pub(crate) fn to_hash_value(&self) -> u32 {
        match self {
            ImageFormat::Png => 1,
            ImageFormat::Jpeg => 2,
            ImageFormat::Webp => 3,
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
            ImageFormat::Webp => image::ImageFormat::WebP,
            ImageFormat::Avif => image::ImageFormat::Avif,
            ImageFormat::Gif => image::ImageFormat::Gif,
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct ImageOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<ImageFormat>,
}

#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Image {
    pub path: PathBuf,
    pub(crate) assets_dir: PathBuf,
    pub(crate) hash: String,
    pub(crate) options: Option<ImageOptions>,
    pub(crate) __cache_placeholder: OnceLock<ImagePlaceholder>,
}

impl Hash for Image {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
        self.assets_dir.hash(state);
        self.hash.hash(state);
        self.options.hash(state);
    }
}

impl Image {
    /// Get a placeholder for the image, which can be used for low-quality image placeholders (LQIP) or similar techniques.
    ///
    /// This uses the [ThumbHash](https://evanw.github.io/thumbhash/) algorithm to generate a very small placeholder image.
    pub fn placeholder(&self) -> &ImagePlaceholder {
        self.__cache_placeholder
            .get_or_init(|| get_placeholder(&self.path).unwrap_or_default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Eq)]
pub struct ImagePlaceholder {
    pub thumbhash: Vec<u8>,
    pub thumbhash_base64: String,
    pub average_rgba: Option<(u8, u8, u8, u8)>,
    pub data_uri: String,
}

fn get_placeholder(path: &PathBuf) -> Option<ImagePlaceholder> {
    let image = image::open(path).ok()?;
    let (width, height) = image.dimensions();
    let (width, height) = (width as usize, height as usize);

    // If width or height > 100, resize image down to max 100
    let (width, height, rgba) = if width.max(height) > 100 {
        let scale = 100.0 / width.max(height) as f32;
        let new_width = (width as f32 * scale).round() as usize;
        let new_height = (height as f32 * scale).round() as usize;

        let resized = image::imageops::resize(
            &image,
            new_width as u32,
            new_height as u32,
            image::imageops::FilterType::Nearest,
        );
        (new_width, new_height, resized.into_raw())
    } else {
        (width, height, image.to_rgba8().into_raw())
    };

    let thumb_hash = rgba_to_thumb_hash(width, height, &rgba);
    let average_rgba = thumb_hash_to_average_rgba(&thumb_hash)
        .ok()
        .map(|(r, g, b, a)| {
            (
                (r * 255.0) as u8,
                (g * 255.0) as u8,
                (b * 255.0) as u8,
                (a * 255.0) as u8,
            )
        });

    let thumbhash_rgba = thumb_hash_to_rgba(&thumb_hash).ok().unwrap();
    let thumbhash_png = thumbhash_to_png(&thumbhash_rgba);
    let optimized_png = if is_dev() {
        thumbhash_png
    } else {
        oxipng::optimize_from_memory(&thumbhash_png, &Default::default()).unwrap()
    };

    let base64 = base64::engine::general_purpose::STANDARD.encode(&optimized_png);
    let data_uri = format!("data:image/png;base64,{}", base64);

    let thumbhash_base64 = base64::engine::general_purpose::STANDARD.encode(&thumb_hash);

    Some(ImagePlaceholder {
        thumbhash: thumb_hash,
        thumbhash_base64,
        average_rgba,
        data_uri,
    })
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
    fn assets_dir(&self) -> PathBuf {
        self.assets_dir.clone()
    }
}

impl Asset for Image {
    fn url(&self) -> Option<String> {
        format!(
            "/{}/{}",
            self.assets_dir().to_string_lossy(),
            self.final_file_name()
        )
        .into()
    }

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
