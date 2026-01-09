use std::fs;
use std::io::Write;
use std::path::Path;

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

impl RouteSitemapMetadata {
    /// Check if this route should be excluded from the sitemap
    pub fn should_exclude(&self) -> bool {
        self.exclude.unwrap_or(false)
    }

    /// Get the change frequency, falling back to a default if not set
    pub fn get_changefreq(&self, default: Option<ChangeFreq>) -> Option<ChangeFreq> {
        self.changefreq.or(default)
    }

    /// Get the priority, falling back to a default if not set
    pub fn get_priority(&self, default: Option<f32>) -> Option<f32> {
        self.priority.or(default)
    }
}

/// Options for sitemap generation.
#[derive(Debug, Clone)]
pub struct SitemapOptions {
    /// Whether to generate a sitemap. Default: `true`
    pub enabled: bool,
    /// The filename for the sitemap. Default: `"sitemap.xml"`
    pub filename: String,
    /// Default change frequency for pages. Default: `None`
    pub default_changefreq: Option<ChangeFreq>,
    /// Default priority for pages. Default: `None`
    pub default_priority: Option<f32>,
}

impl Default for SitemapOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            filename: "sitemap.xml".to_string(),
            default_changefreq: None,
            default_priority: None,
        }
    }
}

/// Change frequency values for sitemap entries.
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
        let mut xml = String::from("  <url>\n");
        xml.push_str(&format!("    <loc>{}</loc>\n", escape_xml(&self.loc)));

        if let Some(ref lastmod) = self.lastmod {
            xml.push_str(&format!("    <lastmod>{}</lastmod>\n", lastmod));
        }

        if let Some(changefreq) = self.changefreq {
            xml.push_str(&format!(
                "    <changefreq>{}</changefreq>\n",
                changefreq.as_str()
            ));
        }

        if let Some(priority) = self.priority {
            xml.push_str(&format!("    <priority>{:.1}</priority>\n", priority));
        }

        xml.push_str("  </url>\n");
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

/// Generates a sitemap from pre-built entries.
pub fn generate_sitemap(
    entries: Vec<SitemapEntry>,
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

    // Generate XML
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

    for entry in &sorted_entries {
        xml.push_str(&entry.to_xml());
    }

    xml.push_str("</urlset>\n");

    // Write to file
    let sitemap_path = output_dir.join(&options.filename);
    let mut file = fs::File::create(&sitemap_path)?;
    file.write_all(xml.as_bytes())?;

    log::info!(
        target: "sitemap",
        "Generated sitemap with {} URLs at {}",
        sorted_entries.len(),
        sitemap_path.display()
    );

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
}
