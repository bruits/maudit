use std::fs;
use std::io::Write;
use std::path::Path;

use rapidhash::fast::RapidHasher;
use std::hash::{Hash, Hasher};

use crate::build::metadata::BuildOutput;

/// Options for PWA (Progressive Web App) support.
///
/// When enabled, Maudit will generate a web app manifest (`manifest.json`),
/// a service worker (`sw.js`), and inject the necessary registration script
/// and manifest link into all pages.
pub struct PwaOptions {
    /// Whether PWA support is enabled. Default: `false`
    pub enabled: bool,
    /// The name of the web application.
    pub name: String,
    /// A short name for the web application, used where space is limited.
    pub short_name: Option<String>,
    /// A description of the web application.
    pub description: Option<String>,
    /// The URL that loads when a user launches the application. Default: `"/"`
    pub start_url: String,
    /// The navigation scope of this web application's context. Default: `"/"`
    pub scope: String,
    /// The preferred display mode for the web application. Default: `Standalone`
    pub display: PwaDisplayMode,
    /// The default theme color for the application.
    pub theme_color: Option<String>,
    /// The expected background color for the web application.
    pub background_color: Option<String>,
    /// Icons for the web application.
    pub icons: Vec<PwaIcon>,
    /// Whether to precache all page URLs in the service worker. Default: `false`
    ///
    /// When enabled, the service worker will cache all pages during installation,
    /// making the entire site available offline.
    pub precache: bool,
}

impl Default for PwaOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            name: String::new(),
            short_name: None,
            description: None,
            start_url: "/".into(),
            scope: "/".into(),
            display: PwaDisplayMode::Standalone,
            theme_color: None,
            background_color: None,
            icons: Vec::new(),
            precache: false,
        }
    }
}

/// Display mode for the web application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PwaDisplayMode {
    Fullscreen,
    Standalone,
    MinimalUi,
    Browser,
}

impl PwaDisplayMode {
    fn as_str(&self) -> &str {
        match self {
            PwaDisplayMode::Fullscreen => "fullscreen",
            PwaDisplayMode::Standalone => "standalone",
            PwaDisplayMode::MinimalUi => "minimal-ui",
            PwaDisplayMode::Browser => "browser",
        }
    }
}

/// An icon for the web application manifest.
pub struct PwaIcon {
    /// Path to the icon file, relative to the site root.
    pub src: String,
    /// Icon dimensions, e.g. `"192x192"` or `"512x512"`.
    pub sizes: String,
    /// MIME type of the icon, e.g. `"image/png"`.
    pub icon_type: Option<String>,
}

fn escape_json_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Generates a `manifest.json` file in the output directory.
pub fn generate_manifest(
    options: &PwaOptions,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output_dir)?;

    let mut json = String::from("{\n");

    json.push_str(&format!(
        "  \"name\": \"{}\",\n",
        escape_json_string(&options.name)
    ));

    if let Some(ref short_name) = options.short_name {
        json.push_str(&format!(
            "  \"short_name\": \"{}\",\n",
            escape_json_string(short_name)
        ));
    }

    if let Some(ref description) = options.description {
        json.push_str(&format!(
            "  \"description\": \"{}\",\n",
            escape_json_string(description)
        ));
    }

    json.push_str(&format!(
        "  \"start_url\": \"{}\",\n",
        escape_json_string(&options.start_url)
    ));
    json.push_str(&format!(
        "  \"scope\": \"{}\",\n",
        escape_json_string(&options.scope)
    ));
    json.push_str(&format!(
        "  \"display\": \"{}\",\n",
        options.display.as_str()
    ));

    if let Some(ref theme_color) = options.theme_color {
        json.push_str(&format!(
            "  \"theme_color\": \"{}\",\n",
            escape_json_string(theme_color)
        ));
    }

    if let Some(ref background_color) = options.background_color {
        json.push_str(&format!(
            "  \"background_color\": \"{}\",\n",
            escape_json_string(background_color)
        ));
    }

    if !options.icons.is_empty() {
        json.push_str("  \"icons\": [\n");
        for (i, icon) in options.icons.iter().enumerate() {
            json.push_str("    {\n");
            json.push_str(&format!(
                "      \"src\": \"{}\",\n",
                escape_json_string(&icon.src)
            ));
            json.push_str(&format!(
                "      \"sizes\": \"{}\"",
                escape_json_string(&icon.sizes)
            ));
            if let Some(ref icon_type) = icon.icon_type {
                json.push_str(&format!(
                    ",\n      \"type\": \"{}\"",
                    escape_json_string(icon_type)
                ));
            }
            json.push('\n');
            if i < options.icons.len() - 1 {
                json.push_str("    },\n");
            } else {
                json.push_str("    }\n");
            }
        }
        json.push_str("  ]\n");
    }

    // Remove trailing comma+newline and replace with just newline if needed
    if json.ends_with(",\n") {
        json.truncate(json.len() - 2);
        json.push('\n');
    }

    json.push('}');

    let manifest_path = output_dir.join("manifest.json");
    let mut file = fs::File::create(&manifest_path)?;
    file.write_all(json.as_bytes())?;

    Ok(())
}

/// Generates a `sw.js` service worker file in the output directory.
///
/// The service worker uses:
/// - Cache-first strategy for hashed assets (JS, CSS, images, fonts)
/// - Network-first strategy for HTML pages
/// - A cache name based on the build timestamp for busting on deploys
///
/// If `precache` is enabled, all page URLs from the build output are cached during installation.
pub fn generate_service_worker(
    options: &PwaOptions,
    build_output: &BuildOutput,
    output_dir: &Path,
    assets_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output_dir)?;

    let mut hasher = RapidHasher::default();
    for asset in &build_output.assets {
        asset.hash(&mut hasher);
    }
    for page in &build_output.pages {
        page.url.hash(&mut hasher);
    }
    let cache_name = format!("maudit-{:x}", hasher.finish());

    let assets_dir_str = assets_dir.to_string_lossy();

    let mut sw = String::new();

    sw.push_str(&format!(
        "const CACHE_NAME = \"{}\";\n",
        escape_json_string(&cache_name)
    ));

    // Match assets in the hashed assets directory (cache-first strategy)
    sw.push_str(&format!(
        "const HASHED_ASSET_RE = /^\\/{}\\//;\n\n",
        escape_json_string(&assets_dir_str)
    ));

    // Precache URLs
    if options.precache {
        sw.push_str("const PRECACHE_URLS = [\n");
        for page in &build_output.pages {
            sw.push_str(&format!(
                "  \"{}\",\n",
                escape_json_string(&page.url)
            ));
        }
        sw.push_str("];\n\n");
    } else {
        sw.push_str("const PRECACHE_URLS = [];\n\n");
    }

    // Install event
    sw.push_str(
        r#"self.addEventListener("install", (event) => {
  if (PRECACHE_URLS.length > 0) {
    event.waitUntil(
      caches.open(CACHE_NAME).then((cache) => cache.addAll(PRECACHE_URLS))
    );
  }
  self.skipWaiting();
});

"#,
    );

    // Activate event — clean old caches
    sw.push_str(
        r#"self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(
        keys
          .filter((key) => key !== CACHE_NAME)
          .map((key) => caches.delete(key))
      )
    )
  );
  self.clients.claim();
});

"#,
    );

    // Fetch event — strategy-based routing
    sw.push_str(
        r#"self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);

  if (event.request.method !== "GET") return;
  if (url.origin !== self.location.origin) return;

  if (HASHED_ASSET_RE.test(url.pathname)) {
    // Cache-first for hashed assets
    event.respondWith(
      caches.match(event.request).then((cached) => {
        if (cached) return cached;
        return fetch(event.request).then((response) => {
          if (response.ok) {
            const clone = response.clone();
            caches.open(CACHE_NAME).then((cache) => cache.put(event.request, clone));
          }
          return response;
        });
      })
    );
  } else {
    // Network-first for HTML pages and other resources
    event.respondWith(
      fetch(event.request)
        .then((response) => {
          if (response.ok) {
            const clone = response.clone();
            caches.open(CACHE_NAME).then((cache) => cache.put(event.request, clone));
          }
          return response;
        })
        .catch(() => caches.match(event.request))
    );
  }
});
"#,
    );

    let sw_path = output_dir.join("sw.js");
    let mut file = fs::File::create(&sw_path)?;
    file.write_all(sw.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_escape_json_string() {
        assert_eq!(escape_json_string("hello"), "hello");
        assert_eq!(escape_json_string("he\"llo"), "he\\\"llo");
        assert_eq!(escape_json_string("he\\llo"), "he\\\\llo");
        assert_eq!(escape_json_string("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_display_mode_as_str() {
        assert_eq!(PwaDisplayMode::Fullscreen.as_str(), "fullscreen");
        assert_eq!(PwaDisplayMode::Standalone.as_str(), "standalone");
        assert_eq!(PwaDisplayMode::MinimalUi.as_str(), "minimal-ui");
        assert_eq!(PwaDisplayMode::Browser.as_str(), "browser");
    }

    #[test]
    fn test_generate_manifest_minimal() {
        let dir = tempdir().unwrap();
        let options = PwaOptions {
            enabled: true,
            name: "Test App".into(),
            ..Default::default()
        };

        generate_manifest(&options, dir.path()).unwrap();

        let content = fs::read_to_string(dir.path().join("manifest.json")).unwrap();
        assert!(content.contains("\"name\": \"Test App\""));
        assert!(content.contains("\"start_url\": \"/\""));
        assert!(content.contains("\"display\": \"standalone\""));
    }

    #[test]
    fn test_generate_manifest_full() {
        let dir = tempdir().unwrap();
        let options = PwaOptions {
            enabled: true,
            name: "My App".into(),
            short_name: Some("App".into()),
            description: Some("A test application".into()),
            start_url: "/".into(),
            scope: "/".into(),
            display: PwaDisplayMode::Fullscreen,
            theme_color: Some("#ffffff".into()),
            background_color: Some("#000000".into()),
            icons: vec![PwaIcon {
                src: "/icon-192.png".into(),
                sizes: "192x192".into(),
                icon_type: Some("image/png".into()),
            }],
            precache: false,
        };

        generate_manifest(&options, dir.path()).unwrap();

        let content = fs::read_to_string(dir.path().join("manifest.json")).unwrap();
        assert!(content.contains("\"short_name\": \"App\""));
        assert!(content.contains("\"description\": \"A test application\""));
        assert!(content.contains("\"display\": \"fullscreen\""));
        assert!(content.contains("\"theme_color\": \"#ffffff\""));
        assert!(content.contains("\"background_color\": \"#000000\""));
        assert!(content.contains("\"src\": \"/icon-192.png\""));
        assert!(content.contains("\"sizes\": \"192x192\""));
        assert!(content.contains("\"type\": \"image/png\""));
    }

    #[test]
    fn test_generate_service_worker() {
        let dir = tempdir().unwrap();
        let options = PwaOptions {
            enabled: true,
            name: "Test".into(),
            precache: false,
            ..Default::default()
        };
        let build_output = BuildOutput::default();
        let assets_dir = Path::new("_maudit");

        generate_service_worker(&options, &build_output, dir.path(), assets_dir).unwrap();

        let content = fs::read_to_string(dir.path().join("sw.js")).unwrap();
        assert!(content.contains("CACHE_NAME"));
        assert!(content.contains("PRECACHE_URLS = []"));
        assert!(content.contains("install"));
        assert!(content.contains("activate"));
        assert!(content.contains("fetch"));
        assert!(content.contains("HASHED_ASSET_RE = /^\\/_maudit\\//"));
    }

    #[test]
    fn test_generate_service_worker_with_precache() {
        let dir = tempdir().unwrap();
        let options = PwaOptions {
            enabled: true,
            name: "Test".into(),
            precache: true,
            ..Default::default()
        };
        let mut build_output = BuildOutput::default();
        build_output.add_page("/".into(), "/".into(), "dist/index.html".into(), None);
        build_output.add_page("/about".into(), "/about".into(), "dist/about/index.html".into(), None);
        let assets_dir = Path::new("_maudit");

        generate_service_worker(&options, &build_output, dir.path(), assets_dir).unwrap();

        let content = fs::read_to_string(dir.path().join("sw.js")).unwrap();
        assert!(content.contains("\"/\""));
        assert!(content.contains("\"/about\""));
    }

    #[test]
    fn test_generate_service_worker_deterministic_cache_name() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();
        let options = PwaOptions {
            enabled: true,
            name: "Test".into(),
            precache: false,
            ..Default::default()
        };
        let mut build_output = BuildOutput::default();
        build_output.add_page("/".into(), "/".into(), "dist/index.html".into(), None);
        build_output.add_asset("main-abc123.js".into());
        let assets_dir = Path::new("_maudit");

        generate_service_worker(&options, &build_output, dir1.path(), assets_dir).unwrap();
        generate_service_worker(&options, &build_output, dir2.path(), assets_dir).unwrap();

        let content1 = fs::read_to_string(dir1.path().join("sw.js")).unwrap();
        let content2 = fs::read_to_string(dir2.path().join("sw.js")).unwrap();
        assert_eq!(content1, content2);
    }
}
