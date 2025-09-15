use std::{fs::File, io::BufWriter, path::Path};

use image::ImageReader;
use webp::{Encoder, WebPMemory};

use crate::assets::{Asset, Image, ImageFormat, ImageOptions};

pub fn process_image(image: &Image, dest_path: &Path, image_options: &ImageOptions) {
    let mut img = ImageReader::open(image.path()).unwrap().decode().unwrap();

    let new_format = image_options.format.clone().unwrap_or(ImageFormat::Webp);
    let new_width = image_options.width.unwrap_or(img.width());
    let new_height = image_options.height.unwrap_or(img.height());

    if new_width != img.width() || new_height != img.height() {
        img = img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);
    }

    // image doesn't support lossy WebP encoding, so we'll use webp directly for that to avoid huge files
    // TODO: Add a feature so that people can choose not to depend on libwebp
    // TODO: Add a way for people to choose lossless WebP encoding, despite the larger file sizes
    if new_format == ImageFormat::Webp {
        let encoder: Encoder = Encoder::from_image(&img).unwrap();
        let webp: WebPMemory = encoder.encode(80f32); // TODO: Allow configuring quality
        std::fs::write(dest_path, &*webp).unwrap();
    } else {
        let file = File::create(dest_path).unwrap();

        let mut writer = BufWriter::new(file);
        img.write_to(&mut writer, new_format.into())
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to process image from {} to {}: {}",
                    image.path().to_string_lossy(),
                    dest_path.to_string_lossy(),
                    e
                )
            });
    }
}
