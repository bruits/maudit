use crate::errors::AssetError;
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

/// Result of bundling a CSS file, containing the output code and metadata
/// about referenced assets that were copied to the output directory.
pub struct BundleCssOutput {
    /// The final CSS code (bundled, url-rewritten, optionally minified).
    pub code: String,
    /// Fingerprinted filenames of assets copied to the output directory
    /// (e.g. `charter_regular.a1b2c.woff2`). Used for stale file cleanup.
    pub copied_asset_filenames: Vec<String>,
    /// Canonical paths of source files referenced via `url()` in the CSS.
    /// Used to detect when referenced assets change between builds.
    pub source_dependencies: Vec<PathBuf>,
}

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
    /// Fingerprinted filenames of assets that were copied to the output directory.
    copied_filenames: Vec<String>,
    /// Canonical source paths of referenced assets (for dependency tracking).
    source_deps: Vec<PathBuf>,
    /// Errors encountered during visitation (collected because the Visitor trait
    /// uses `Infallible` as its error type).
    errors: Vec<AssetError>,
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
            let hash = match calculate_hash(&source_path, None) {
                Ok(h) => h,
                Err(e) => {
                    self.errors.push(e);
                    return Ok(());
                }
            };

            let extension = source_path.extension().and_then(|e| e.to_str());
            let fingerprinted = make_filename(&source_path, &hash, extension);
            let dest_path = self.output_dir.join(&fingerprinted);

            if let Err(e) = fs::copy(&source_path, &dest_path) {
                self.errors.push(AssetError::CopyFailed {
                    source_path,
                    dest_path,
                    source: e,
                });
                return Ok(());
            }

            debug!("Copied CSS asset {:?} -> {:?}", source_path, dest_path);

            let filename = fingerprinted.to_string_lossy().to_string();
            self.copied_filenames.push(filename.clone());
            self.source_deps.push(source_path);

            url.url = filename.into();
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
) -> Result<BundleCssOutput, Box<dyn std::error::Error>> {
    let source_dir = entry
        .parent()
        .ok_or_else(|| format!("CSS entry has no parent directory: {}", entry.display()))?
        .to_path_buf();

    let mut url_visitor = AssetUrlVisitor {
        source_dir,
        output_dir: output_dir.to_path_buf(),
        copied_filenames: Vec::new(),
        source_deps: Vec::new(),
        errors: Vec::new(),
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
        let provider = FileProvider::new();
        let mut bundler = Bundler::new(&provider, None, ParserOptions::default());

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

    if let Some(err) = url_visitor.errors.into_iter().next() {
        return Err(err.into());
    }

    Ok(BundleCssOutput {
        code,
        copied_asset_filenames: url_visitor.copied_filenames,
        source_dependencies: url_visitor.source_deps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_css_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();

        // Create a simple CSS file
        fs::write(
            dir.path().join("style.css"),
            "body { color: red; }",
        )
        .unwrap();

        // Create a font file referenced by CSS
        fs::write(dir.path().join("font.woff2"), b"fake-font-data").unwrap();

        // Create a CSS file that references the font
        fs::write(
            dir.path().join("with_url.css"),
            "body { background: url(font.woff2); }",
        )
        .unwrap();

        // Create CSS files for @import testing
        fs::write(
            dir.path().join("_partial.css"),
            "h1 { font-size: 2em; }",
        )
        .unwrap();
        fs::write(
            dir.path().join("main.css"),
            "@import \"_partial.css\";\nbody { color: blue; }",
        )
        .unwrap();

        dir
    }

    #[test]
    fn test_bundle_basic_css() {
        let dir = setup_css_dir();
        let output_dir = tempfile::tempdir().unwrap();

        let result = bundle_css(
            &dir.path().join("style.css"),
            None,
            false,
            output_dir.path(),
        )
        .unwrap();

        assert!(result.code.contains("color"));
        assert!(result.copied_asset_filenames.is_empty());
        assert!(result.source_dependencies.is_empty());
    }

    #[test]
    fn test_bundle_with_import() {
        let dir = setup_css_dir();
        let output_dir = tempfile::tempdir().unwrap();

        let result = bundle_css(
            &dir.path().join("main.css"),
            None,
            false,
            output_dir.path(),
        )
        .unwrap();

        // The bundled output should contain both the partial and the main styles
        assert!(result.code.contains("font-size"), "should contain @imported partial");
        assert!(result.code.contains("color"), "should contain main styles");
    }

    #[test]
    fn test_bundle_rewrites_url_and_copies_asset() {
        let dir = setup_css_dir();
        let output_dir = tempfile::tempdir().unwrap();

        let result = bundle_css(
            &dir.path().join("with_url.css"),
            None,
            false,
            output_dir.path(),
        )
        .unwrap();

        // url() should have been rewritten to a fingerprinted filename
        assert!(!result.code.contains("url(font.woff2)"), "original url should be rewritten");
        assert_eq!(result.copied_asset_filenames.len(), 1);
        assert!(result.copied_asset_filenames[0].contains("font"));
        assert!(result.copied_asset_filenames[0].ends_with(".woff2"));

        // The asset file should have been copied to the output directory
        let copied = output_dir.path().join(&result.copied_asset_filenames[0]);
        assert!(copied.exists(), "asset should be copied to output dir");
        assert_eq!(fs::read(&copied).unwrap(), b"fake-font-data");

        // Source dependencies should track the original font path
        assert_eq!(result.source_dependencies.len(), 1);
        assert!(result.source_dependencies[0].ends_with("font.woff2"));
    }

    #[test]
    fn test_bundle_minification() {
        let dir = setup_css_dir();
        let output_dir = tempfile::tempdir().unwrap();

        let not_minified = bundle_css(
            &dir.path().join("style.css"),
            None,
            false,
            output_dir.path(),
        )
        .unwrap();

        let minified = bundle_css(
            &dir.path().join("style.css"),
            None,
            true,
            output_dir.path(),
        )
        .unwrap();

        assert!(
            minified.code.len() <= not_minified.code.len(),
            "minified output should not be longer"
        );
        // Minified should still contain the actual style value
        assert!(minified.code.contains("red"));
    }

    #[test]
    fn test_bundle_with_source_css() {
        let dir = setup_css_dir();
        let output_dir = tempfile::tempdir().unwrap();

        // Simulate Tailwind output being passed as source_css
        let tailwind_css = "body { background: url(font.woff2); margin: 0; }";

        let result = bundle_css(
            &dir.path().join("with_url.css"),
            Some(tailwind_css),
            false,
            output_dir.path(),
        )
        .unwrap();

        // Should contain the tailwind output, not the original file content
        assert!(result.code.contains("margin"), "should use source_css content");
        // url() should still be rewritten and asset copied
        assert_eq!(result.copied_asset_filenames.len(), 1);
        assert!(output_dir.path().join(&result.copied_asset_filenames[0]).exists());
    }

    #[test]
    fn test_bundle_skips_absolute_and_data_urls() {
        let dir = setup_css_dir();
        let output_dir = tempfile::tempdir().unwrap();

        let css = r#"
            .a { background: url(data:image/png;base64,abc); }
            .b { background: url(https://example.com/img.png); }
            .c { background: url(http://example.com/img.png); }
            .d { background: url(/absolute/path.png); }
        "#;

        let result = bundle_css(
            &dir.path().join("style.css"),
            Some(css),
            false,
            output_dir.path(),
        )
        .unwrap();

        // None of these should trigger asset copying
        assert!(result.copied_asset_filenames.is_empty());
        assert!(result.source_dependencies.is_empty());
    }
}
