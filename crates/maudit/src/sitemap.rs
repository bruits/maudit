use std::fs;
use std::io::Write;
use std::path::Path;

// THOUGHTS: I don't like that we maintain an implementation of sitemap generation here. I'd like to either move this into a
// separate crate or use an existing crate for this. But, the existing crates I found didn't really satisfy my needs, which is annoying.

/// Metadata for sitemap configuration on a specific route.
#[derive(Debug, Clone, Default)]
pub struct RouteSitemapMetadata {
    /// Whether to exclude this route from the sitemap
    pub exclude: Option<bool>,
    /// Change frequency for this route
    pub changefreq: Option<ChangeFreq>,
    /// Priority for this route (0.0 to 1.0)
    pub priority: Option<f32>,
}

/// Options for sitemap generation.
#[derive(Debug, Clone)]
pub struct SitemapOptions {
    /// Whether to generate a sitemap. Default: `false`
    pub enabled: bool,
    /// The filename for the sitemap index. Default: `"sitemap.xml"`
    ///
    /// If multiple sitemaps are needed, individual sitemap files will be named `sitemap-1.xml`, `sitemap-2.xml`, etc.
    pub filename: String,
    /// Maximum number of URLs per sitemap file. Default: `10000`
    ///
    /// Note that search engines will often ignore sitemaps with more than 50,000 URLs,
    /// so it's recommended to keep this value at or below that limit.
    pub max_urls_per_sitemap: usize,
    /// Default change frequency for pages. Default: `None`
    ///
    /// Note that changefreq is often ignored by search engines nowadays.
    pub default_changefreq: Option<ChangeFreq>,
    /// Default priority for pages. Default: `None`
    ///
    /// Note that priority is often ignored by search engines nowadays.
    pub default_priority: Option<f32>,
    /// Optional XSL stylesheet URL for styling the sitemap. Default: `None`
    ///
    /// If the value starts with `http(s)://`it will be used as-is (ex: your stylesheet might be coming from a CDN).
    ///
    /// Otherwise, the path is appended to the base URL. For example, `sitemap.xsl` with base URL
    /// `https://example.com` becomes `https://example.com/sitemap.xsl`.
    pub stylesheet: Option<String>,
}

impl Default for SitemapOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            filename: "sitemap.xml".to_string(),
            max_urls_per_sitemap: 10000,
            default_changefreq: None,
            default_priority: None,
            stylesheet: None,
        }
    }
}

/// Change frequency values for sitemap entries.
///
/// See: https://www.sitemaps.org/protocol.html#changefreqdef for more details.
/// This property is often ignored by search engines nowadays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChangeFreq {
    Always,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Never,
}

impl ChangeFreq {
    fn as_str(&self) -> &str {
        match self {
            ChangeFreq::Always => "always",
            ChangeFreq::Hourly => "hourly",
            ChangeFreq::Daily => "daily",
            ChangeFreq::Weekly => "weekly",
            ChangeFreq::Monthly => "monthly",
            ChangeFreq::Yearly => "yearly",
            ChangeFreq::Never => "never",
        }
    }
}

/// Represents a single URL entry in the sitemap.
#[derive(Debug)]
pub struct SitemapEntry {
    pub loc: String,
    pub lastmod: Option<String>,
    pub changefreq: Option<ChangeFreq>,
    pub priority: Option<f32>,
}

impl SitemapEntry {
    fn to_xml(&self) -> String {
        let mut xml = String::from("<url>");
        xml.push_str(&format!("<loc>{}</loc>", escape_xml(&self.loc)));

        if let Some(ref lastmod) = self.lastmod {
            xml.push_str(&format!("<lastmod>{}</lastmod>", lastmod));
        }

        if let Some(changefreq) = self.changefreq {
            xml.push_str(&format!("<changefreq>{}</changefreq>", changefreq.as_str()));
        }

        if let Some(priority) = self.priority {
            xml.push_str(&format!("<priority>{:.1}</priority>", priority));
        }

        xml.push_str("</url>");
        xml
    }
}

/// Represents a sitemap file reference in a sitemap index.
#[derive(Debug)]
struct SitemapReference {
    loc: String,
    lastmod: Option<String>,
}

impl SitemapReference {
    fn to_xml(&self) -> String {
        let mut xml = String::from("<sitemap>");
        xml.push_str(&format!("<loc>{}</loc>", escape_xml(&self.loc)));

        if let Some(ref lastmod) = self.lastmod {
            xml.push_str(&format!("<lastmod>{}</lastmod>", lastmod));
        }

        xml.push_str("</sitemap>");
        xml
    }
}

/// Escapes XML special characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Resolves a stylesheet path to a full URL.
/// If the path starts with http:// or https://, it's used as-is.
/// Otherwise, it's appended to the base URL.
fn resolve_stylesheet_url(base_url: &str, stylesheet_path: &str) -> String {
    if stylesheet_path.starts_with("http://") || stylesheet_path.starts_with("https://") {
        stylesheet_path.to_string()
    } else {
        format!("{}{}", base_url.trim_end_matches('/'), stylesheet_path)
    }
}

/// Generates a sitemap index with multiple sitemap files from pre-built entries.
pub fn generate_sitemap(
    entries: Vec<SitemapEntry>,
    base_url: &str,
    output_dir: &Path,
    options: &SitemapOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    if !options.enabled {
        return Ok(());
    }

    if entries.is_empty() {
        return Ok(());
    }

    // Sort entries by URL for consistency
    let mut sorted_entries = entries;
    sorted_entries.sort_by(|a, b| a.loc.cmp(&b.loc));

    let total_entries = sorted_entries.len();

    // If we have very few entries, just create a single sitemap
    if total_entries <= options.max_urls_per_sitemap {
        generate_single_sitemap(
            &sorted_entries,
            output_dir,
            &options.filename,
            base_url,
            options.stylesheet.as_deref(),
        )?;

        log::info!(
            target: "sitemap",
            "Generated sitemap with {} URLs at {}",
            total_entries,
            output_dir.join(&options.filename).display()
        );

        return Ok(());
    }

    // Split into chunks and create multiple sitemap files
    let chunks: Vec<&[SitemapEntry]> = sorted_entries
        .chunks(options.max_urls_per_sitemap)
        .collect();

    let num_sitemaps = chunks.len();
    let mut sitemap_refs = Vec::new();

    // Generate individual sitemap files
    for (i, chunk) in chunks.iter().enumerate() {
        let sitemap_num = i + 1;
        let sitemap_filename = format!("sitemap-{}.xml", sitemap_num);

        generate_single_sitemap(
            chunk,
            output_dir,
            &sitemap_filename,
            base_url,
            options.stylesheet.as_deref(),
        )?;

        let sitemap_url = format!("{}/{}", base_url.trim_end_matches('/'), sitemap_filename);
        sitemap_refs.push(SitemapReference {
            loc: sitemap_url,
            lastmod: None, // TODO: Somehow the user should be able to specify lastmod per chunk or we should somehow calculate it? Probably can't and probably doesn't matter anyway.
        });
    }

    generate_sitemap_index(
        &sitemap_refs,
        output_dir,
        &options.filename,
        base_url,
        options.stylesheet.as_deref(),
    )?;

    log::info!(
        target: "sitemap",
        "Generated sitemap index with {} sitemaps ({} total URLs) at {}",
        num_sitemaps,
        total_entries,
        output_dir.join(&options.filename).display()
    );

    Ok(())
}

/// Generates a single sitemap file.
fn generate_single_sitemap(
    entries: &[SitemapEntry],
    output_dir: &Path,
    filename: &str,
    base_url: &str,
    stylesheet: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");

    if let Some(stylesheet_path) = stylesheet {
        let stylesheet_url = resolve_stylesheet_url(base_url, stylesheet_path);
        xml.push_str(&format!(
            "<?xml-stylesheet type=\"text/xsl\" href=\"{}\"?>\n",
            escape_xml(&stylesheet_url)
        ));
    }

    xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">");

    for entry in entries {
        xml.push_str(&entry.to_xml());
    }

    xml.push_str("</urlset>");

    let sitemap_path = output_dir.join(filename);
    let mut file = fs::File::create(&sitemap_path)?;
    file.write_all(xml.as_bytes())?;

    Ok(())
}

/// Generates a sitemap index file.
fn generate_sitemap_index(
    sitemaps: &[SitemapReference],
    output_dir: &Path,
    filename: &str,
    base_url: &str,
    stylesheet: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");

    if let Some(stylesheet_path) = stylesheet {
        let stylesheet_url = resolve_stylesheet_url(base_url, stylesheet_path);
        xml.push_str(&format!(
            "<?xml-stylesheet type=\"text/xsl\" href=\"{}\"?>\n",
            escape_xml(&stylesheet_url)
        ));
    }

    xml.push_str("<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">");

    for sitemap in sitemaps {
        xml.push_str(&sitemap.to_xml());
    }

    xml.push_str("</sitemapindex>");

    let index_path = output_dir.join(filename);
    let mut file = fs::File::create(&index_path)?;
    file.write_all(xml.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("hello"), "hello");
        assert_eq!(escape_xml("a&b"), "a&amp;b");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(
            escape_xml("it's \"quoted\""),
            "it&apos;s &quot;quoted&quot;"
        );
    }

    #[test]
    fn test_changefreq_as_str() {
        assert_eq!(ChangeFreq::Always.as_str(), "always");
        assert_eq!(ChangeFreq::Daily.as_str(), "daily");
        assert_eq!(ChangeFreq::Never.as_str(), "never");
    }

    #[test]
    fn test_sitemap_entry_to_xml() {
        let entry = SitemapEntry {
            loc: "https://example.com/page".to_string(),
            lastmod: Some("2024-01-01".to_string()),
            changefreq: Some(ChangeFreq::Weekly),
            priority: Some(0.8),
        };

        let xml = entry.to_xml();
        assert!(xml.contains("<loc>https://example.com/page</loc>"));
        assert!(xml.contains("<lastmod>2024-01-01</lastmod>"));
        assert!(xml.contains("<changefreq>weekly</changefreq>"));
        assert!(xml.contains("<priority>0.8</priority>"));
    }

    #[test]
    fn test_sitemap_entry_minimal() {
        let entry = SitemapEntry {
            loc: "https://example.com/".to_string(),
            lastmod: None,
            changefreq: None,
            priority: None,
        };

        let xml = entry.to_xml();
        assert!(xml.contains("<loc>https://example.com/</loc>"));
        assert!(!xml.contains("<lastmod>"));
        assert!(!xml.contains("<changefreq>"));
        assert!(!xml.contains("<priority>"));
    }

    #[test]
    fn test_sitemap_reference_to_xml() {
        let reference = SitemapReference {
            loc: "https://example.com/sitemap-1.xml".to_string(),
            lastmod: Some("2024-01-01".to_string()),
        };

        let xml = reference.to_xml();
        assert!(xml.contains("<sitemap>"));
        assert!(xml.contains("<loc>https://example.com/sitemap-1.xml</loc>"));
        assert!(xml.contains("<lastmod>2024-01-01</lastmod>"));
        assert!(xml.contains("</sitemap>"));
    }

    #[test]
    fn test_generate_single_sitemap_with_stylesheet() {
        use std::io::Read;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let entries = vec![
            SitemapEntry {
                loc: "https://example.com/page1".to_string(),
                lastmod: None,
                changefreq: None,
                priority: None,
            },
            SitemapEntry {
                loc: "https://example.com/page2".to_string(),
                lastmod: None,
                changefreq: None,
                priority: None,
            },
        ];

        generate_single_sitemap(
            &entries,
            dir.path(),
            "sitemap.xml",
            "https://example.com",
            Some("/sitemap.xsl"),
        )
        .unwrap();

        let mut file = std::fs::File::open(dir.path().join("sitemap.xml")).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();

        assert!(content.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(content.contains(
            "<?xml-stylesheet type=\"text/xsl\" href=\"https://example.com/sitemap.xsl\"?>"
        ));
        assert!(content.contains("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">"));
        assert!(content.contains("<loc>https://example.com/page1</loc>"));
        assert!(content.contains("<loc>https://example.com/page2</loc>"));
    }

    #[test]
    fn test_generate_single_sitemap_without_stylesheet() {
        use std::io::Read;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let entries = vec![SitemapEntry {
            loc: "https://example.com/page1".to_string(),
            lastmod: None,
            changefreq: None,
            priority: None,
        }];

        generate_single_sitemap(
            &entries,
            dir.path(),
            "sitemap.xml",
            "https://example.com",
            None,
        )
        .unwrap();

        let mut file = std::fs::File::open(dir.path().join("sitemap.xml")).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();

        assert!(content.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(!content.contains("<?xml-stylesheet"));
        assert!(content.contains("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">"));
    }

    #[test]
    fn test_generate_sitemap_index_with_stylesheet() {
        use std::io::Read;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let sitemaps = vec![
            SitemapReference {
                loc: "https://example.com/sitemap-1.xml".to_string(),
                lastmod: None,
            },
            SitemapReference {
                loc: "https://example.com/sitemap-2.xml".to_string(),
                lastmod: None,
            },
        ];

        generate_sitemap_index(
            &sitemaps,
            dir.path(),
            "sitemap.xml",
            "https://example.com",
            Some("/sitemap.xsl"),
        )
        .unwrap();

        let mut file = std::fs::File::open(dir.path().join("sitemap.xml")).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();

        assert!(content.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(content.contains(
            "<?xml-stylesheet type=\"text/xsl\" href=\"https://example.com/sitemap.xsl\"?>"
        ));
        assert!(
            content
                .contains("<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">")
        );
        assert!(content.contains("<loc>https://example.com/sitemap-1.xml</loc>"));
        assert!(content.contains("<loc>https://example.com/sitemap-2.xml</loc>"));
    }

    #[test]
    fn test_stylesheet_xml_escaping() {
        use std::io::Read;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let entries = vec![SitemapEntry {
            loc: "https://example.com/".to_string(),
            lastmod: None,
            changefreq: None,
            priority: None,
        }];

        generate_single_sitemap(
            &entries,
            dir.path(),
            "sitemap.xml",
            "https://example.com",
            Some("/sitemap.xsl?param=value&other=123"),
        )
        .unwrap();

        let mut file = std::fs::File::open(dir.path().join("sitemap.xml")).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();

        // Verify that & is properly escaped in the stylesheet URL
        assert!(
            content.contains("href=\"https://example.com/sitemap.xsl?param=value&amp;other=123\"")
        );
    }

    #[test]
    fn test_stylesheet_absolute_url() {
        use std::io::Read;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let entries = vec![SitemapEntry {
            loc: "https://example.com/".to_string(),
            lastmod: None,
            changefreq: None,
            priority: None,
        }];

        generate_single_sitemap(
            &entries,
            dir.path(),
            "sitemap.xml",
            "https://example.com",
            Some("https://cdn.example.com/sitemap.xsl"),
        )
        .unwrap();

        let mut file = std::fs::File::open(dir.path().join("sitemap.xml")).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();

        // Verify that absolute URLs are used as-is
        assert!(content.contains("href=\"https://cdn.example.com/sitemap.xsl\""));
    }

    #[test]
    fn test_stylesheet_absolute_url_http() {
        use std::io::Read;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let entries = vec![SitemapEntry {
            loc: "https://example.com/".to_string(),
            lastmod: None,
            changefreq: None,
            priority: None,
        }];

        generate_single_sitemap(
            &entries,
            dir.path(),
            "sitemap.xml",
            "https://example.com",
            Some("http://cdn.example.com/sitemap.xsl"),
        )
        .unwrap();

        let mut file = std::fs::File::open(dir.path().join("sitemap.xml")).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();

        // Verify that http:// URLs are also used as-is
        assert!(content.contains("href=\"http://cdn.example.com/sitemap.xsl\""));
    }
}
