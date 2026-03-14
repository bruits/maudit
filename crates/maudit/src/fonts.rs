use std::{
    fmt,
    fs::{self},
    path::{Path, PathBuf},
};

use crate::{
    AssetHashingStrategy,
    assets::{HashAssetType, HashConfig, calculate_hash, make_filename, make_final_path, make_final_url},
};

/// A font family with one or more variants to be registered and made available via CSS custom properties.
///
/// ## Example
/// ```rust
/// use maudit::fonts::{FontFamily, FontVariant, FontWeight, FontStyle, FontDisplay};
///
/// FontFamily {
///     family: "Charter".into(),
///     css_variable: "--font-charter".into(),
///     fallbacks: vec!["Bitstream Charter".into(), "Cambria".into(), "serif".into()],
///     display: FontDisplay::Swap,
///     variants: vec![
///         FontVariant {
///             file: "./assets/fonts/charter_regular.woff2".into(),
///             weight: FontWeight::Single(400),
///             style: FontStyle::Normal,
///             unicode_range: None,
///         },
///     ],
/// };
/// ```
pub struct FontFamily {
    /// The CSS font-family name used in `@font-face` declarations.
    pub family: String,
    /// CSS custom property name, e.g. `"--font-charter"`.
    /// Use in CSS as `font-family: var(--font-charter)`.
    pub css_variable: String,
    /// Fallback fonts appended to the CSS variable value.
    pub fallbacks: Vec<String>,
    /// The `font-display` strategy. Defaults to [`FontDisplay::Swap`].
    pub display: FontDisplay,
    /// One or more font variants (file + weight + style combinations).
    pub variants: Vec<FontVariant>,
}

/// A single font variant pointing to a specific font file with its weight and style.
pub struct FontVariant {
    /// Path to the font file (`.woff2`, `.woff`, `.ttf`, `.otf`), resolved relative to the current working directory.
    pub file: PathBuf,
    /// Font weight for this variant.
    pub weight: FontWeight,
    /// Font style for this variant.
    pub style: FontStyle,
    /// Optional CSS `unicode-range` value, e.g. `"U+0000-00FF, U+0131"`.
    pub unicode_range: Option<String>,
}

/// Font weight, either a single value or a range (for variable fonts).
pub enum FontWeight {
    /// A single weight, e.g. `400`.
    Single(u16),
    /// A weight range for variable fonts, e.g. `Range(200, 900)`.
    Range(u16, u16),
}

impl fmt::Display for FontWeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontWeight::Single(w) => write!(f, "{}", w),
            FontWeight::Range(lo, hi) => write!(f, "{} {}", lo, hi),
        }
    }
}

/// Font style.
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

impl fmt::Display for FontStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontStyle::Normal => write!(f, "normal"),
            FontStyle::Italic => write!(f, "italic"),
            FontStyle::Oblique => write!(f, "oblique"),
        }
    }
}

/// The `font-display` CSS descriptor.
pub enum FontDisplay {
    Swap,
    Block,
    Fallback,
    Optional,
    Auto,
}

impl fmt::Display for FontDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontDisplay::Swap => write!(f, "swap"),
            FontDisplay::Block => write!(f, "block"),
            FontDisplay::Fallback => write!(f, "fallback"),
            FontDisplay::Optional => write!(f, "optional"),
            FontDisplay::Auto => write!(f, "auto"),
        }
    }
}

impl Default for FontDisplay {
    fn default() -> Self {
        FontDisplay::Swap
    }
}

fn font_format(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("woff2") => "woff2",
        Some("woff") => "woff",
        Some("ttf") => "truetype",
        Some("otf") => "opentype",
        _ => "woff2",
    }
}

fn css_quote_family(name: &str) -> String {
    let generic = [
        "serif",
        "sans-serif",
        "monospace",
        "cursive",
        "fantasy",
        "system-ui",
        "ui-serif",
        "ui-sans-serif",
        "ui-monospace",
        "ui-rounded",
        "math",
        "emoji",
        "fangsong",
    ];

    if generic.contains(&name) {
        name.to_string()
    } else {
        format!("\"{}\"", name)
    }
}

/// A font file that needs to be copied to the output directory.
#[derive(Debug)]
pub(crate) struct FontAsset {
    /// Source path (canonical) to the font file.
    pub source: PathBuf,
    /// Destination path in the output directory.
    pub dest: PathBuf,
}

/// Result of generating inline font CSS.
#[derive(Debug)]
pub(crate) struct FontCssResult {
    /// The CSS string containing `@font-face` declarations and `:root` custom properties,
    /// ready to be inlined in a `<style>` tag.
    pub css: String,
    /// Font files that need to be copied to the output directory.
    pub assets: Vec<FontAsset>,
}

/// Generate inline CSS for font `@font-face` declarations and `:root` CSS custom properties.
///
/// Font file URLs are resolved to their final hashed paths so the CSS can be directly
/// inlined into HTML `<style>` tags. Returns the CSS string and a list of font files
/// to copy to the output directory.
pub(crate) fn generate_font_css_inline(
    fonts: &[FontFamily],
    assets_dir: &Path,
    output_assets_dir: &Path,
    hashing_strategy: &AssetHashingStrategy,
) -> Result<FontCssResult, Box<dyn std::error::Error>> {
    let mut css = String::new();
    let mut font_assets = Vec::new();

    for family in fonts {
        for variant in &family.variants {
            let canonical_path = fs::canonicalize(&variant.file).map_err(|e| {
                format!(
                    "Font file not found: {} ({})",
                    variant.file.display(),
                    e
                )
            })?;

            let format = font_format(&variant.file);
            let extension = variant.file.extension().and_then(|e| e.to_str());

            let hash = calculate_hash(
                &canonical_path,
                Some(&HashConfig {
                    asset_type: HashAssetType::Script, // Font files have no special hash options
                    hashing_strategy,
                }),
            )?;

            let filename = make_filename(&canonical_path, &hash, extension);
            let url = make_final_url(assets_dir, &filename);
            let dest = make_final_path(output_assets_dir, &filename);

            css.push_str("@font-face {\n");
            css.push_str(&format!(
                "\tfont-family: \"{}\";\n",
                family.family
            ));
            css.push_str(&format!(
                "\tsrc: url(\"{}\") format(\"{}\");\n",
                url, format
            ));
            css.push_str(&format!("\tfont-weight: {};\n", variant.weight));
            css.push_str(&format!("\tfont-style: {};\n", variant.style));
            css.push_str(&format!("\tfont-display: {};\n", family.display));

            if let Some(ref range) = variant.unicode_range {
                css.push_str(&format!("\tunicode-range: {};\n", range));
            }

            css.push_str("}\n\n");

            font_assets.push(FontAsset {
                source: canonical_path,
                dest,
            });
        }
    }

    // Generate :root with CSS custom properties
    if !fonts.is_empty() {
        css.push_str(":root {\n");
        for family in fonts {
            let mut value_parts: Vec<String> =
                vec![css_quote_family(&family.family)];
            for fallback in &family.fallbacks {
                value_parts.push(css_quote_family(fallback));
            }
            css.push_str(&format!(
                "\t{}: {};\n",
                family.css_variable,
                value_parts.join(", ")
            ));
        }
        css.push_str("}\n");
    }

    Ok(FontCssResult {
        css,
        assets: font_assets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_weight_display() {
        assert_eq!(FontWeight::Single(400).to_string(), "400");
        assert_eq!(FontWeight::Range(200, 900).to_string(), "200 900");
    }

    #[test]
    fn test_font_style_display() {
        assert_eq!(FontStyle::Normal.to_string(), "normal");
        assert_eq!(FontStyle::Italic.to_string(), "italic");
        assert_eq!(FontStyle::Oblique.to_string(), "oblique");
    }

    #[test]
    fn test_font_display_display() {
        assert_eq!(FontDisplay::Swap.to_string(), "swap");
        assert_eq!(FontDisplay::Block.to_string(), "block");
        assert_eq!(FontDisplay::Fallback.to_string(), "fallback");
        assert_eq!(FontDisplay::Optional.to_string(), "optional");
        assert_eq!(FontDisplay::Auto.to_string(), "auto");
    }

    #[test]
    fn test_font_format_detection() {
        assert_eq!(font_format(Path::new("font.woff2")), "woff2");
        assert_eq!(font_format(Path::new("font.woff")), "woff");
        assert_eq!(font_format(Path::new("font.ttf")), "truetype");
        assert_eq!(font_format(Path::new("font.otf")), "opentype");
        assert_eq!(font_format(Path::new("font.unknown")), "woff2");
    }

    #[test]
    fn test_css_quote_family() {
        assert_eq!(css_quote_family("Charter"), "\"Charter\"");
        assert_eq!(css_quote_family("serif"), "serif");
        assert_eq!(css_quote_family("sans-serif"), "sans-serif");
        assert_eq!(css_quote_family("system-ui"), "system-ui");
        assert_eq!(
            css_quote_family("Bitstream Charter"),
            "\"Bitstream Charter\""
        );
    }

    #[test]
    fn test_generate_font_css_inline() {
        let temp_dir = tempfile::tempdir().unwrap();
        let font_file = temp_dir.path().join("test.woff2");
        std::fs::write(&font_file, b"fake font data").unwrap();

        let fonts = vec![FontFamily {
            family: "TestFont".into(),
            css_variable: "--font-test".into(),
            fallbacks: vec!["sans-serif".into()],
            display: FontDisplay::Swap,
            variants: vec![FontVariant {
                file: font_file.clone(),
                weight: FontWeight::Single(400),
                style: FontStyle::Normal,
                unicode_range: None,
            }],
        }];

        let result = generate_font_css_inline(
            &fonts,
            Path::new("_maudit"),
            &temp_dir.path().join("output"),
            &AssetHashingStrategy::Precise,
        )
        .unwrap();

        assert!(result.css.contains("@font-face"));
        assert!(result.css.contains("font-family: \"TestFont\""));
        assert!(result.css.contains("format(\"woff2\")"));
        assert!(result.css.contains("font-weight: 400"));
        assert!(result.css.contains("font-style: normal"));
        assert!(result.css.contains("font-display: swap"));
        assert!(result.css.contains("--font-test: \"TestFont\", sans-serif"));
        // URL should be a hashed path, not the original file path
        assert!(result.css.contains("/_maudit/test."));
        assert!(result.css.contains(".woff2"));
        // Should have one font asset to copy
        assert_eq!(result.assets.len(), 1);
        assert_eq!(result.assets[0].source, fs::canonicalize(&font_file).unwrap());
    }

    #[test]
    fn test_generate_font_css_inline_with_unicode_range() {
        let temp_dir = tempfile::tempdir().unwrap();
        let font_file = temp_dir.path().join("test.woff2");
        std::fs::write(&font_file, b"fake font data").unwrap();

        let fonts = vec![FontFamily {
            family: "TestFont".into(),
            css_variable: "--font-test".into(),
            fallbacks: vec![],
            display: FontDisplay::default(),
            variants: vec![FontVariant {
                file: font_file,
                weight: FontWeight::Range(200, 900),
                style: FontStyle::Italic,
                unicode_range: Some("U+0000-00FF, U+0131".into()),
            }],
        }];

        let result = generate_font_css_inline(
            &fonts,
            Path::new("_maudit"),
            &temp_dir.path().join("output"),
            &AssetHashingStrategy::Precise,
        )
        .unwrap();

        assert!(result.css.contains("font-weight: 200 900"));
        assert!(result.css.contains("font-style: italic"));
        assert!(result.css.contains("unicode-range: U+0000-00FF, U+0131"));
    }

    #[test]
    fn test_generate_font_css_inline_multiple_families() {
        let temp_dir = tempfile::tempdir().unwrap();
        let font_a = temp_dir.path().join("a.woff2");
        let font_b = temp_dir.path().join("b.woff2");
        std::fs::write(&font_a, b"font a").unwrap();
        std::fs::write(&font_b, b"font b").unwrap();

        let fonts = vec![
            FontFamily {
                family: "FontA".into(),
                css_variable: "--font-a".into(),
                fallbacks: vec!["serif".into()],
                display: FontDisplay::Swap,
                variants: vec![FontVariant {
                    file: font_a,
                    weight: FontWeight::Single(400),
                    style: FontStyle::Normal,
                    unicode_range: None,
                }],
            },
            FontFamily {
                family: "FontB".into(),
                css_variable: "--font-b".into(),
                fallbacks: vec!["monospace".into()],
                display: FontDisplay::Block,
                variants: vec![FontVariant {
                    file: font_b,
                    weight: FontWeight::Single(700),
                    style: FontStyle::Normal,
                    unicode_range: None,
                }],
            },
        ];

        let result = generate_font_css_inline(
            &fonts,
            Path::new("_maudit"),
            &temp_dir.path().join("output"),
            &AssetHashingStrategy::Precise,
        )
        .unwrap();

        assert!(result.css.contains("--font-a: \"FontA\", serif"));
        assert!(result.css.contains("--font-b: \"FontB\", monospace"));
        assert!(result.css.contains("font-display: swap"));
        assert!(result.css.contains("font-display: block"));
        assert_eq!(result.assets.len(), 2);
    }

    #[test]
    fn test_font_file_not_found() {
        let fonts = vec![FontFamily {
            family: "Missing".into(),
            css_variable: "--font-missing".into(),
            fallbacks: vec![],
            display: FontDisplay::default(),
            variants: vec![FontVariant {
                file: "/nonexistent/font.woff2".into(),
                weight: FontWeight::Single(400),
                style: FontStyle::Normal,
                unicode_range: None,
            }],
        }];

        let result = generate_font_css_inline(
            &fonts,
            Path::new("_maudit"),
            Path::new("/tmp/output"),
            &AssetHashingStrategy::Precise,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Font file not found"));
    }
}
