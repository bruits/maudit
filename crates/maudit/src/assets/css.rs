use std::convert::Infallible;
use std::fs;
use std::path::{Path, PathBuf};

use lightningcss::bundler::{Bundler, FileProvider};
use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, StyleSheet};
use lightningcss::values::url::Url;
use lightningcss::visit_types;
use lightningcss::visitor::{Visit, Visitor};
use log::debug;

use super::{calculate_hash, make_filename};

/// Visitor that rewrites relative `url()` references in CSS.
///
/// For each relative URL, the referenced file is copied to `output_dir`
/// with a fingerprinted filename (e.g. `charter_regular.a1b2c.woff2`)
/// and the URL is rewritten to point to the copy.
struct AssetUrlVisitor {
    /// Directory containing the source CSS file (for resolving relative URLs)
    source_dir: PathBuf,
    /// Directory where output assets are written (e.g. `dist/_maudit`)
    output_dir: PathBuf,
}

impl<'i> Visitor<'i> for AssetUrlVisitor {
    type Error = Infallible;

    fn visit_types(&self) -> lightningcss::visitor::VisitTypes {
        visit_types!(URLS)
    }

    fn visit_url(&mut self, url: &mut Url<'i>) -> Result<(), Self::Error> {
        let url_str: &str = &url.url;

        // Skip data URLs and absolute URLs
        if url_str.starts_with("data:")
            || url_str.starts_with("http://")
            || url_str.starts_with("https://")
            || url_str.starts_with('/')
        {
            return Ok(());
        }

        // Resolve the referenced file relative to the source CSS directory
        let source_path = self.source_dir.join(url_str);

        if let Ok(source_path) = source_path.canonicalize()
            && source_path.is_file()
        {
            // Hash the file contents for fingerprinting
            let hash =
                calculate_hash(&source_path, None).expect("failed to hash CSS-referenced asset");

            let extension = source_path.extension().and_then(|e| e.to_str());
            let fingerprinted = make_filename(&source_path, &hash, extension);
            let dest_path = self.output_dir.join(&fingerprinted);

            if let Err(e) = fs::copy(&source_path, &dest_path) {
                debug!(
                    "Failed to copy asset {:?} to {:?}: {}",
                    source_path, dest_path, e
                );
                return Ok(());
            }

            debug!("Copied CSS asset {:?} -> {:?}", source_path, dest_path);

            url.url = fingerprinted.to_string_lossy().to_string().into();
        }

        Ok(())
    }
}

/// Bundle a CSS file, resolving `@import` statements, copying referenced assets,
/// and optionally minifying.
///
/// If `source_css` is provided, it is used as the CSS content instead of reading from the file
/// (useful for tailwind-processed output). In that case, `@import` resolution is skipped.
///
/// Referenced assets (fonts, images) are copied to `output_dir` with fingerprinted
/// filenames and their URLs are rewritten.
pub fn bundle_css(
    entry: &Path,
    source_css: Option<&str>,
    minify: bool,
    output_dir: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let source_dir = entry
        .parent()
        .ok_or_else(|| format!("CSS entry has no parent directory: {}", entry.display()))?
        .to_path_buf();

    let mut url_visitor = AssetUrlVisitor {
        source_dir,
        output_dir: output_dir.to_path_buf(),
    };

    let code = if let Some(css) = source_css {
        let mut stylesheet = StyleSheet::parse(css, ParserOptions::default())
            .map_err(|e| format!("Failed to parse CSS: {}", e))?;

        stylesheet.visit(&mut url_visitor).unwrap();

        if minify {
            stylesheet
                .minify(MinifyOptions::default())
                .map_err(|e| format!("Failed to minify CSS: {}", e))?;
        }

        stylesheet
            .to_css(PrinterOptions {
                minify,
                ..Default::default()
            })
            .map_err(|e| format!("Failed to serialize CSS: {}", e))?
            .code
    } else {
        let fs = FileProvider::new();
        let mut bundler = Bundler::new(&fs, None, ParserOptions::default());

        let mut stylesheet = bundler
            .bundle(entry)
            .map_err(|e| format!("Failed to bundle CSS file {}: {}", entry.display(), e))?;

        stylesheet.visit(&mut url_visitor).unwrap();

        if minify {
            stylesheet
                .minify(MinifyOptions::default())
                .map_err(|e| format!("Failed to minify CSS: {}", e))?;
        }

        stylesheet
            .to_css(PrinterOptions {
                minify,
                ..Default::default()
            })
            .map_err(|e| format!("Failed to serialize CSS: {}", e))?
            .code
    };

    Ok(code)
}
