use base64::Engine;
use dyn_eq::DynEq;
use image::GenericImageView;
use rustc_hash::FxHashSet;
use std::hash::Hash;
use std::sync::OnceLock;
use std::{fs, path::PathBuf};
use thumbhash::{rgba_to_thumb_hash, thumb_hash_to_average_rgba, thumb_hash_to_rgba};

#[derive(Default)]
pub struct PageAssets {
    pub(crate) images: FxHashSet<Image>,
    pub(crate) scripts: FxHashSet<Script>,
    pub(crate) styles: FxHashSet<Style>,

    pub(crate) included_styles: Vec<Style>,
    pub(crate) included_scripts: Vec<Script>,

    pub(crate) assets_dir: PathBuf,
}

impl PageAssets {
    /// Add an image to the page assets, causing the file to be created in the output directory. The image is resolved relative to the current working directory.
    ///
    /// The image will not automatically be included in the page, but can be included through the `.url()` method on the returned `Image` object.
    ///
    /// Subsequent calls to this function using the same path will return the same image, as such, the value returned by this function can be cloned and used multiple times without issue.
    pub fn add_image_with_options<P>(&mut self, image_path: P, options: ImageOptions) -> Image
    where
        P: Into<PathBuf>,
    {
        let image_path = image_path.into();

        // Check if the image already exists in the assets, if so, return it
        if let Some(image) = self.images.iter().find_map(|asset| {
            asset.as_any().downcast_ref::<Image>().filter(|image| {
                image.path == image_path
                    && options == *image.options.as_ref().unwrap_or(&ImageOptions::default())
            })
        }) {
            return image.clone();
        }

        let image = Image {
            path: image_path.clone(),
            assets_dir: self.assets_dir.clone(),
            hash: calculate_hash(&image_path),
            options: if options == ImageOptions::default() {
                None
            } else {
                Some(options)
            },
            __cache_placeholder: OnceLock::new(),
        };

        self.images.insert(image.clone());

        image
    }

    pub fn add_image<P>(&mut self, image_path: P) -> Image
    where
        P: Into<PathBuf>,
    {
        self.add_image_with_options(image_path, ImageOptions::default())
    }

    /// Add a script to the page assets, causing the file to be created in the output directory. The script is resolved relative to the current working directory.
    ///
    /// The script will not automatically be included in the page, but can be included through the `.url()` method on the returned `Script` object.
    /// Alternatively, a script can be included automatically using the [PageAssets::include_script] method instead.
    ///
    /// Subsequent calls to this function using the same path will return the same script, as such, the value returned by this function can be cloned and used multiple times without issue.
    pub fn add_script<P>(&mut self, script_path: P) -> Script
    where
        P: Into<PathBuf>,
    {
        let path = script_path.into();
        let script = Script {
            path: path.clone(),
            assets_dir: self.assets_dir.clone(),
            hash: calculate_hash(&path),
        };

        self.scripts.insert(script.clone());

        script
    }

    /// Include a script in the page. The script is resolved relative to the current working directory.
    ///
    /// This method will automatically include the script in the `<head>` of the page, if it exists. If the page does not include a `<head>` tag, at this time this method will silently fail.
    ///
    /// Subsequent calls to this function using the same path will result in the same script being included multiple times.
    pub fn include_script<P>(&mut self, script_path: P)
    where
        P: Into<PathBuf>,
    {
        let path = script_path.into();
        let script = Script {
            path: path.clone(),
            assets_dir: self.assets_dir.clone(),
            hash: calculate_hash(&path),
        };

        self.scripts.insert(script.clone());
        self.included_scripts.push(script);
    }

    /// Add a style to the page assets, causing the file to be created in the output directory. The style is resolved relative to the current working directory.
    ///
    /// The style will not automatically be included in the page, but can be included through the `.url()` method on the returned `Style` object.
    /// Alternatively, a style can be included automatically using the [PageAssets::include_style] method instead.
    ///
    /// Subsequent calls to this method using the same path will return the same style, as such, the value returned by this method can be cloned and used multiple times without issue. this method is equivalent to calling `add_style_with_options` with the default `StyleOptions` and is purely provided for convenience.
    pub fn add_style<P>(&mut self, style_path: P) -> Style
    where
        P: Into<PathBuf>,
    {
        self.add_style_with_options(style_path, StyleOptions::default())
    }

    /// Add a style to the page assets, causing the file to be created in the output directory. The style is resolved relative to the current working directory.
    ///
    /// The style will not automatically be included in the page, but can be included through the `.url()` method on the returned `Style` object.
    ///
    /// Subsequent calls to this method using the same path will return the same style, as such, the value returned by this method can be cloned and used multiple times without issue.
    pub fn add_style_with_options<P>(&mut self, style_path: P, options: StyleOptions) -> Style
    where
        P: Into<PathBuf>,
    {
        let path = style_path.into();
        let style = Style {
            path: path.clone(),
            assets_dir: self.assets_dir.clone(),
            hash: calculate_hash(&path),
            tailwind: options.tailwind,
        };

        self.styles.insert(style.clone());

        style
    }

    /// Include a style in the page
    ///
    /// This method will automatically include the style in the `<head>` of the page, if it exists. If the page does not include a `<head>` tag, at this time this method will silently fail.
    ///
    /// Subsequent calls to this method using the same path will result in the same style being included multiple times. This method is equivalent to calling `include_style_with_options` with the default `StyleOptions` and is purely provided for convenience.
    pub fn include_style<P>(&mut self, style_path: P)
    where
        P: Into<PathBuf>,
    {
        self.include_style_with_options(style_path, StyleOptions::default())
    }

    /// Include a style in the page
    ///
    /// This method will automatically include the style in the `<head>` of the page, if it exists. If the page does not include a `<head>` tag, at this time this method will silently fail.
    ///
    /// Subsequent calls to this method using the same path will result in the same style being included multiple times.
    pub fn include_style_with_options<P>(&mut self, style_path: P, options: StyleOptions)
    where
        P: Into<PathBuf>,
    {
        let path = style_path.into();
        let style = Style {
            path: path.clone(),
            assets_dir: self.assets_dir.clone(),
            hash: calculate_hash(&path),
            tailwind: options.tailwind,
        };

        self.styles.insert(style.clone());
        self.included_styles.push(style);
    }
}

#[allow(private_bounds)] // Users never interact with the internal trait, so it's fine
pub trait Asset: DynEq + InternalAsset + Sync + Send {
    fn build_path(&self) -> PathBuf {
        self.assets_dir().join(self.final_file_name())
    }
    fn url(&self) -> Option<String>;
    fn path(&self) -> &PathBuf;

    fn hash(&self) -> String {
        // This will be overridden by each implementation to return the cached hash
        String::new()
    }

    // TODO: I don't like these next two methods for scripts and styles, we should get this from Rolldown somehow, but I don't know how.
    // Our architecture is such that bundling runs after pages, so we can't know the final extension until then. We can't, and I don't want
    // to make it so we get assets beforehand because it'd make it less convenient and essentially cause us to act like a bundling framework.
    //
    // Perhaps it should be done as a post-processing step, like includes, but that'd require moving route finalization to after bundling,
    // which I'm not sure I want to do either. Plus, it'd be pretty slow if you have a layout on every page that includes a style/script (a fairly common case).
    //
    // An additional benefit would with that would also be to be able to avoid generating hashes for these files, but that's a smaller win.
    //
    // I don't know! - erika, 2025-09-01

    fn final_extension(&self) -> String {
        self.path()
            .extension()
            .map(|ext| ext.to_str().unwrap())
            .unwrap_or_default()
            .to_owned()
    }

    fn final_file_name(&self) -> String {
        let file_stem = self.path().file_stem().unwrap().to_str().unwrap();
        let extension = self.final_extension();

        if extension.is_empty() {
            format!("{}.{}", file_stem, self.hash())
        } else {
            format!("{}.{}.{}", file_stem, self.hash(), extension)
        }
    }
}

fn calculate_hash(path: &PathBuf) -> String {
    let content = fs::read(path).unwrap();

    // TODO: Consider using xxhash for both performance and to match Rolldown's hashing
    let mut hasher = blake3::Hasher::new();
    hasher.update(&content);
    hasher.update(path.to_string_lossy().as_bytes());
    let hash = hasher.finalize();

    // Take the first 5 characters of the hex string for a short hash like "al3hx"
    hash.to_hex()[..5].to_string()
}

trait InternalAsset {
    fn assets_dir(&self) -> PathBuf;
}

impl Hash for dyn Asset {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path().hash(state);
    }
}

dyn_eq::eq_trait_object!(Asset);

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
            image::imageops::FilterType::Triangle,
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
    let optimized_png = oxipng::optimize_from_memory(&thumbhash_png, &Default::default()).unwrap();

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

#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Script {
    pub path: PathBuf,
    pub(crate) assets_dir: PathBuf,
    pub(crate) hash: String,
}

impl InternalAsset for Script {
    fn assets_dir(&self) -> PathBuf {
        self.assets_dir.clone()
    }
}

impl Asset for Script {
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
        let current_extension = self
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default();

        match current_extension {
            "ts" => "js",
            ext => ext,
        }
        .to_string()
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct StyleOptions {
    pub tailwind: bool,
}

#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Style {
    pub path: PathBuf,
    pub(crate) assets_dir: PathBuf,
    pub(crate) hash: String,
    pub(crate) tailwind: bool,
}

impl InternalAsset for Style {
    fn assets_dir(&self) -> PathBuf {
        self.assets_dir.clone()
    }
}

impl Asset for Style {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn setup_temp_dir() -> PathBuf {
        // Create a temporary directory and test files
        let temp_dir = env::temp_dir().join("maudit_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        std::fs::write(temp_dir.join("style.css"), "body { background: red; }").unwrap();
        std::fs::write(temp_dir.join("script.js"), "console.log('Hello, world!');").unwrap();
        std::fs::write(temp_dir.join("image.png"), b"").unwrap();
        temp_dir
    }

    #[test]
    fn test_add_style() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };
        page_assets.add_style(temp_dir.join("style.css"));

        assert!(page_assets.styles.len() == 1);
    }

    #[test]
    fn test_include_style() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

        page_assets.include_style(temp_dir.join("style.css"));

        assert!(page_assets.styles.len() == 1);
        assert!(page_assets.included_styles.len() == 1);
    }

    #[test]
    fn test_add_script() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

        page_assets.add_script(temp_dir.join("script.js"));
        assert!(page_assets.scripts.len() == 1);
    }

    #[test]
    fn test_include_script() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

        page_assets.include_script(temp_dir.join("script.js"));

        assert!(page_assets.scripts.len() == 1);
        assert!(page_assets.included_scripts.len() == 1);
    }

    #[test]
    fn test_add_image() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

        page_assets.add_image(temp_dir.join("image.png"));
        assert!(page_assets.images.len() == 1);
    }

    #[test]
    fn test_asset_has_leading_slash() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

        let image = page_assets.add_image(temp_dir.join("image.png"));
        assert_eq!(image.url().unwrap().chars().next(), Some('/'));

        let script = page_assets.add_script(temp_dir.join("script.js"));
        assert_eq!(script.url().unwrap().chars().next(), Some('/'));

        let style = page_assets.add_style(temp_dir.join("style.css"));
        assert_eq!(style.url().unwrap().chars().next(), Some('/'));
    }

    #[test]
    fn test_asset_url_include_hash() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

        let image = page_assets.add_image(temp_dir.join("image.png"));
        let image_hash = image.hash.clone();
        assert!(image.url().unwrap().contains(&image_hash));

        let script = page_assets.add_script(temp_dir.join("script.js"));
        let script_hash = script.hash.clone();
        assert!(script.url().unwrap().contains(&script_hash));

        let style = page_assets.add_style(temp_dir.join("style.css"));
        let style_hash = style.hash.clone();
        assert!(style.url().unwrap().contains(&style_hash));
    }

    #[test]
    fn test_asset_path_include_hash() {
        let temp_dir = setup_temp_dir();
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

        let image = page_assets.add_image(temp_dir.join("image.png"));
        let image_hash = image.hash.clone();
        assert!(image.build_path().to_string_lossy().contains(&image_hash));

        let script = page_assets.add_script(temp_dir.join("script.js"));
        let script_hash = script.hash.clone();
        assert!(script.build_path().to_string_lossy().contains(&script_hash));

        let style = page_assets.add_style(temp_dir.join("style.css"));
        let style_hash = style.hash.clone();
        assert!(style.build_path().to_string_lossy().contains(&style_hash));
    }
}
