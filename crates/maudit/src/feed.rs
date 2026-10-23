//! RSS 2.0 and Atom 1.0 feed generation.
//!
//! Feeds are implemented as regular routes that return a [`RssFeed`] or [`AtomFeed`] value.
//! This gives you full control over feed content and lets you have multiple feeds
//! (per-category, per-author, etc.) without any configuration overhead.
//!
//! ## Example
//!
//! ```rust
//! use maudit::route::prelude::*;
//! use maudit::feed::{RssFeed, RssItem};
//! # use maudit::content::markdown_entry;
//! #
//! # #[markdown_entry]
//! # pub struct ArticleContent {
//! #     pub title: String,
//! #     pub description: String,
//! # }
//!
//! #[route("/feed.xml")]
//! pub struct Feed;
//!
//! impl Route for Feed {
//!     fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
//!         let articles = ctx.content.get_source::<ArticleContent>("articles");
//!         let base = ctx.base_url.as_deref().unwrap_or("");
//!
//!         RssFeed::new(
//!             "My Blog",
//!             ctx.canonical_url().unwrap_or_default(),
//!             "Latest articles from my blog",
//!         )
//!         .items(articles.entries.iter().map(|entry| {
//!             let data = entry.data(ctx);
//!             RssItem::new(
//!                 data.title.clone(),
//!                 format!("{}/articles/{}", base, entry.id),
//!             )
//!             .description(&data.description)
//!         }))
//!     }
//! }
//! ```

use crate::route::RenderResult;

// ---------------------------------------------------------------------------
// XML helpers
// ---------------------------------------------------------------------------

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Wraps content in a CDATA section, escaping any `]]>` sequences within.
fn cdata(s: &str) -> String {
    format!("<![CDATA[{}]]>", s.replace("]]>", "]]]]><![CDATA[>"))
}

// ---------------------------------------------------------------------------
// RssItem
// ---------------------------------------------------------------------------

/// A single item (entry) in an RSS 2.0 feed.
///
/// ## Example
/// ```rust
/// use maudit::feed::RssItem;
///
/// let item = RssItem::new("My Article", "https://example.com/articles/my-article")
///     .description("A short summary of the article.")
///     .pub_date(Some("2026-03-04T00:00:00Z"))
///     .content("<p>Full HTML content here.</p>");
/// ```
#[derive(Debug, Clone, Default)]
pub struct RssItem {
    title: String,
    /// Full URL to the page.
    link: String,
    /// Short excerpt or summary. Shown in feed readers that don't display full content.
    description: Option<String>,
    /// Publication date. Accepts ISO 8601 (`2026-03-04T00:00:00Z`) or RFC 2822 formats.
    pub_date: Option<String>,
    /// Author email address (RSS 2.0 convention: `name@example.com (Display Name)`).
    author: Option<String>,
    /// Full HTML content, emitted as `<content:encoded>`.
    content: Option<String>,
    /// Unique identifier for the item. Defaults to [`link`](Self::link) if not set.
    guid: Option<String>,
    categories: Vec<String>,
}

impl RssItem {
    pub fn new(title: impl Into<String>, link: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            link: link.into(),
            ..Default::default()
        }
    }

    /// Short excerpt or summary displayed in feed readers.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Publication date. Accepts ISO 8601 (`2026-03-04T00:00:00Z`) or RFC 2822 formats.
    pub fn pub_date(mut self, date: Option<&str>) -> Self {
        self.pub_date = date.map(str::to_owned);
        self
    }

    /// Author of the item (RSS 2.0: `email@example.com (Name)`).
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Full HTML content emitted as `<content:encoded>`.
    pub fn content(mut self, html: impl Into<String>) -> Self {
        self.content = Some(html.into());
        self
    }

    /// Override the item GUID. Defaults to the item link.
    pub fn guid(mut self, guid: impl Into<String>) -> Self {
        self.guid = Some(guid.into());
        self
    }

    /// Add a category tag to the item.
    pub fn category(mut self, cat: impl Into<String>) -> Self {
        self.categories.push(cat.into());
        self
    }

    fn render_xml(&self) -> String {
        let mut out = String::from("    <item>\n");

        out.push_str(&format!("      <title>{}</title>\n", xml_escape(&self.title)));
        out.push_str(&format!("      <link>{}</link>\n", xml_escape(&self.link)));

        let guid = self.guid.as_deref().unwrap_or(&self.link);
        out.push_str(&format!(
            "      <guid isPermaLink=\"true\">{}</guid>\n",
            xml_escape(guid)
        ));

        if let Some(desc) = &self.description {
            out.push_str(&format!(
                "      <description>{}</description>\n",
                xml_escape(desc)
            ));
        }

        if let Some(date) = &self.pub_date {
            out.push_str(&format!("      <pubDate>{}</pubDate>\n", xml_escape(date)));
        }

        if let Some(author) = &self.author {
            out.push_str(&format!(
                "      <author>{}</author>\n",
                xml_escape(author)
            ));
        }

        for cat in &self.categories {
            out.push_str(&format!(
                "      <category>{}</category>\n",
                xml_escape(cat)
            ));
        }

        if let Some(html) = &self.content {
            out.push_str(&format!(
                "      <content:encoded>{}</content:encoded>\n",
                cdata(html)
            ));
        }

        out.push_str("    </item>\n");
        out
    }
}

// ---------------------------------------------------------------------------
// RssFeed
// ---------------------------------------------------------------------------

/// An RSS 2.0 feed.
///
/// Implements [`Into<RenderResult>`] so it can be returned directly from a route's `render` method.
///
/// ## Example
/// ```rust
/// use maudit::route::prelude::*;
/// use maudit::feed::{RssFeed, RssItem};
///
/// #[route("/feed.xml")]
/// pub struct Feed;
///
/// impl Route for Feed {
///     fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
///         RssFeed::new("My Blog", "https://example.com", "Latest posts")
///             .language("en")
///     }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct RssFeed {
    title: String,
    /// Canonical URL of the website or section this feed covers.
    link: String,
    description: String,
    language: Option<String>,
    /// Time-to-live in minutes — how long clients may cache the feed.
    ttl: Option<u32>,
    items: Vec<RssItem>,
}

impl RssFeed {
    pub fn new(
        title: impl Into<String>,
        link: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            link: link.into(),
            description: description.into(),
            ..Default::default()
        }
    }

    /// BCP 47 language tag for the feed (e.g. `"en"`, `"fr"`, `"pt-BR"`).
    pub fn language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    /// How many minutes clients may cache the feed before re-fetching.
    pub fn ttl(mut self, minutes: u32) -> Self {
        self.ttl = Some(minutes);
        self
    }

    /// Add items to the feed from any iterator of [`RssItem`].
    pub fn items(mut self, items: impl IntoIterator<Item = RssItem>) -> Self {
        self.items.extend(items);
        self
    }

    fn render_xml(&self) -> String {
        let mut out = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <rss version=\"2.0\" xmlns:content=\"http://purl.org/rss/1.0/modules/content/\">\n\
               <channel>\n",
        );

        out.push_str(&format!(
            "    <title>{}</title>\n",
            xml_escape(&self.title)
        ));
        out.push_str(&format!(
            "    <link>{}</link>\n",
            xml_escape(&self.link)
        ));
        out.push_str(&format!(
            "    <description>{}</description>\n",
            xml_escape(&self.description)
        ));

        if let Some(lang) = &self.language {
            out.push_str(&format!("    <language>{}</language>\n", xml_escape(lang)));
        }

        if let Some(ttl) = self.ttl {
            out.push_str(&format!("    <ttl>{ttl}</ttl>\n"));
        }

        for item in &self.items {
            out.push_str(&item.render_xml());
        }

        out.push_str("  </channel>\n</rss>\n");
        out
    }
}

impl From<RssFeed> for RenderResult {
    fn from(feed: RssFeed) -> RenderResult {
        RenderResult::Raw(feed.render_xml().into_bytes())
    }
}

// ---------------------------------------------------------------------------
// AtomEntry
// ---------------------------------------------------------------------------

/// A single entry in an Atom 1.0 feed.
///
/// ## Example
/// ```rust
/// use maudit::feed::AtomEntry;
///
/// let entry = AtomEntry::new("My Article", "https://example.com/articles/my-article")
///     .summary("A short summary.")
///     .updated("2026-03-04T00:00:00Z")
///     .content("<p>Full HTML content.</p>");
/// ```
#[derive(Debug, Clone, Default)]
pub struct AtomEntry {
    title: String,
    /// Full URL to the page (used as both link and id).
    link: String,
    summary: Option<String>,
    /// ISO 8601 timestamp. Required by the Atom spec — defaults to an empty string if not provided.
    updated: Option<String>,
    /// ISO 8601 timestamp for initial publication.
    published: Option<String>,
    author_name: Option<String>,
    author_email: Option<String>,
    /// Full HTML content.
    content: Option<String>,
    /// Override the entry id. Defaults to [`link`](Self::link).
    id: Option<String>,
    categories: Vec<String>,
}

impl AtomEntry {
    pub fn new(title: impl Into<String>, link: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            link: link.into(),
            ..Default::default()
        }
    }

    /// Short summary displayed in feed readers that don't show full content.
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// ISO 8601 last-modified timestamp (e.g. `"2026-03-04T00:00:00Z"`). Required by Atom spec.
    pub fn updated(mut self, date: impl Into<String>) -> Self {
        self.updated = Some(date.into());
        self
    }

    /// ISO 8601 initial publication timestamp.
    pub fn published(mut self, date: impl Into<String>) -> Self {
        self.published = Some(date.into());
        self
    }

    /// Display name of the entry author.
    pub fn author_name(mut self, name: impl Into<String>) -> Self {
        self.author_name = Some(name.into());
        self
    }

    /// Email address of the entry author.
    pub fn author_email(mut self, email: impl Into<String>) -> Self {
        self.author_email = Some(email.into());
        self
    }

    /// Full HTML body of the entry.
    pub fn content(mut self, html: impl Into<String>) -> Self {
        self.content = Some(html.into());
        self
    }

    /// Override the entry `<id>`. Defaults to the entry link.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add a category term to the entry.
    pub fn category(mut self, term: impl Into<String>) -> Self {
        self.categories.push(term.into());
        self
    }

    fn render_xml(&self) -> String {
        let mut out = String::from("    <entry>\n");

        out.push_str(&format!(
            "      <title>{}</title>\n",
            xml_escape(&self.title)
        ));
        out.push_str(&format!(
            "      <link href=\"{}\"/>\n",
            xml_escape(&self.link)
        ));

        let id = self.id.as_deref().unwrap_or(&self.link);
        out.push_str(&format!("      <id>{}</id>\n", xml_escape(id)));

        let updated = self.updated.as_deref().unwrap_or("");
        out.push_str(&format!(
            "      <updated>{}</updated>\n",
            xml_escape(updated)
        ));

        if let Some(published) = &self.published {
            out.push_str(&format!(
                "      <published>{}</published>\n",
                xml_escape(published)
            ));
        }

        if self.author_name.is_some() || self.author_email.is_some() {
            out.push_str("      <author>\n");
            if let Some(name) = &self.author_name {
                out.push_str(&format!("        <name>{}</name>\n", xml_escape(name)));
            }
            if let Some(email) = &self.author_email {
                out.push_str(&format!("        <email>{}</email>\n", xml_escape(email)));
            }
            out.push_str("      </author>\n");
        }

        if let Some(summary) = &self.summary {
            out.push_str(&format!(
                "      <summary>{}</summary>\n",
                xml_escape(summary)
            ));
        }

        for cat in &self.categories {
            out.push_str(&format!(
                "      <category term=\"{}\"/>\n",
                xml_escape(cat)
            ));
        }

        if let Some(html) = &self.content {
            out.push_str(&format!(
                "      <content type=\"html\">{}</content>\n",
                cdata(html)
            ));
        }

        out.push_str("    </entry>\n");
        out
    }
}

// ---------------------------------------------------------------------------
// AtomFeed
// ---------------------------------------------------------------------------

/// An Atom 1.0 feed.
///
/// Implements [`Into<RenderResult>`] so it can be returned directly from a route's `render` method.
///
/// ## Example
/// ```rust
/// use maudit::route::prelude::*;
/// use maudit::feed::{AtomFeed, AtomEntry};
///
/// #[route("/feed.atom")]
/// pub struct Feed;
///
/// impl Route for Feed {
///     fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
///         AtomFeed::new("My Blog", "https://example.com")
///             .self_link("https://example.com/feed.atom")
///             .updated("2026-03-04T00:00:00Z")
///     }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct AtomFeed {
    title: String,
    /// Canonical URL of the website or section.
    link: String,
    /// URL of this feed document itself (the `rel="self"` link).
    self_link: Option<String>,
    /// ISO 8601 timestamp of the most recent update across all entries.
    updated: Option<String>,
    author_name: Option<String>,
    author_email: Option<String>,
    subtitle: Option<String>,
    /// Override the feed `<id>`. Defaults to [`link`](Self::link).
    id: Option<String>,
    entries: Vec<AtomEntry>,
}

impl AtomFeed {
    pub fn new(title: impl Into<String>, link: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            link: link.into(),
            ..Default::default()
        }
    }

    /// URL of this feed document (used as `<link rel="self">`).
    pub fn self_link(mut self, url: impl Into<String>) -> Self {
        self.self_link = Some(url.into());
        self
    }

    /// ISO 8601 timestamp of the most recent update in the feed.
    pub fn updated(mut self, date: impl Into<String>) -> Self {
        self.updated = Some(date.into());
        self
    }

    /// Display name of the feed-level author.
    pub fn author_name(mut self, name: impl Into<String>) -> Self {
        self.author_name = Some(name.into());
        self
    }

    /// Email of the feed-level author.
    pub fn author_email(mut self, email: impl Into<String>) -> Self {
        self.author_email = Some(email.into());
        self
    }

    /// Short tagline or subtitle for the feed.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Override the feed `<id>`. Defaults to the feed link.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add entries to the feed from any iterator of [`AtomEntry`].
    pub fn entries(mut self, entries: impl IntoIterator<Item = AtomEntry>) -> Self {
        self.entries.extend(entries);
        self
    }

    fn render_xml(&self) -> String {
        let mut out = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <feed xmlns=\"http://www.w3.org/2005/Atom\">\n",
        );

        out.push_str(&format!(
            "  <title>{}</title>\n",
            xml_escape(&self.title)
        ));
        out.push_str(&format!(
            "  <link href=\"{}\"/>\n",
            xml_escape(&self.link)
        ));

        if let Some(self_link) = &self.self_link {
            out.push_str(&format!(
                "  <link rel=\"self\" href=\"{}\"/>\n",
                xml_escape(self_link)
            ));
        }

        let id = self.id.as_deref().unwrap_or(&self.link);
        out.push_str(&format!("  <id>{}</id>\n", xml_escape(id)));

        let updated = self.updated.as_deref().unwrap_or("");
        out.push_str(&format!("  <updated>{}</updated>\n", xml_escape(updated)));

        if let Some(subtitle) = &self.subtitle {
            out.push_str(&format!(
                "  <subtitle>{}</subtitle>\n",
                xml_escape(subtitle)
            ));
        }

        if self.author_name.is_some() || self.author_email.is_some() {
            out.push_str("  <author>\n");
            if let Some(name) = &self.author_name {
                out.push_str(&format!("    <name>{}</name>\n", xml_escape(name)));
            }
            if let Some(email) = &self.author_email {
                out.push_str(&format!("    <email>{}</email>\n", xml_escape(email)));
            }
            out.push_str("  </author>\n");
        }

        for entry in &self.entries {
            out.push_str(&entry.render_xml());
        }

        out.push_str("</feed>\n");
        out
    }
}

impl From<AtomFeed> for RenderResult {
    fn from(feed: AtomFeed) -> RenderResult {
        RenderResult::Raw(feed.render_xml().into_bytes())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("a & b < c > d"), "a &amp; b &lt; c &gt; d");
        assert_eq!(xml_escape("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(xml_escape("it's"), "it&apos;s");
    }

    #[test]
    fn test_cdata_basic() {
        assert_eq!(cdata("<p>Hello</p>"), "<![CDATA[<p>Hello</p>]]>");
    }

    #[test]
    fn test_cdata_escapes_end_sequence() {
        assert_eq!(
            cdata("a]]>b"),
            "<![CDATA[a]]]]><![CDATA[>b]]>"
        );
    }

    #[test]
    fn test_rss_feed_minimal() {
        let feed = RssFeed::new("My Blog", "https://example.com", "A blog");
        let xml = feed.render_xml();
        assert!(xml.contains("<rss version=\"2.0\""));
        assert!(xml.contains("<title>My Blog</title>"));
        assert!(xml.contains("<link>https://example.com</link>"));
        assert!(xml.contains("<description>A blog</description>"));
        assert!(xml.contains("</channel>"));
        assert!(xml.contains("</rss>"));
    }

    #[test]
    fn test_rss_feed_with_item() {
        let feed = RssFeed::new("My Blog", "https://example.com", "A blog")
            .language("en")
            .ttl(60)
            .items([
                RssItem::new("First Post", "https://example.com/posts/first")
                    .description("Summary here")
                    .pub_date(Some("2026-03-04T00:00:00Z"))
                    .author("author@example.com (Author)")
                    .content("<p>Full content</p>")
                    .category("rust"),
            ]);

        let xml = feed.render_xml();
        assert!(xml.contains("<language>en</language>"));
        assert!(xml.contains("<ttl>60</ttl>"));
        assert!(xml.contains("<title>First Post</title>"));
        assert!(xml.contains("<link>https://example.com/posts/first</link>"));
        assert!(xml.contains("<description>Summary here</description>"));
        assert!(xml.contains("<pubDate>2026-03-04T00:00:00Z</pubDate>"));
        assert!(xml.contains("<content:encoded>"));
        assert!(xml.contains("<category>rust</category>"));
    }

    #[test]
    fn test_rss_item_guid_defaults_to_link() {
        let item = RssItem::new("Post", "https://example.com/post");
        let xml = item.render_xml();
        assert!(xml.contains("<guid isPermaLink=\"true\">https://example.com/post</guid>"));
    }

    #[test]
    fn test_rss_into_render_result() {
        let feed = RssFeed::new("Blog", "https://example.com", "desc");
        let result: RenderResult = feed.into();
        match result {
            RenderResult::Raw(bytes) => {
                let s = String::from_utf8(bytes).unwrap();
                assert!(s.starts_with("<?xml"));
            }
            _ => panic!("expected RenderResult::Raw"),
        }
    }

    #[test]
    fn test_atom_feed_minimal() {
        let feed = AtomFeed::new("My Blog", "https://example.com")
            .updated("2026-03-04T00:00:00Z");
        let xml = feed.render_xml();
        assert!(xml.contains("xmlns=\"http://www.w3.org/2005/Atom\""));
        assert!(xml.contains("<title>My Blog</title>"));
        assert!(xml.contains("<updated>2026-03-04T00:00:00Z</updated>"));
        assert!(xml.contains("</feed>"));
    }

    #[test]
    fn test_atom_feed_with_entry() {
        let feed = AtomFeed::new("My Blog", "https://example.com")
            .self_link("https://example.com/feed.atom")
            .updated("2026-03-04T00:00:00Z")
            .author_name("Jane")
            .author_email("jane@example.com")
            .subtitle("A Rust blog")
            .entries([
                AtomEntry::new("First Post", "https://example.com/posts/first")
                    .summary("Summary here")
                    .updated("2026-03-04T00:00:00Z")
                    .published("2026-03-01T00:00:00Z")
                    .author_name("Jane")
                    .content("<p>Full content</p>")
                    .category("rust"),
            ]);

        let xml = feed.render_xml();
        assert!(xml.contains("<link rel=\"self\" href=\"https://example.com/feed.atom\"/>"));
        assert!(xml.contains("<subtitle>A Rust blog</subtitle>"));
        assert!(xml.contains("<name>Jane</name>"));
        assert!(xml.contains("<entry>"));
        assert!(xml.contains("<published>2026-03-01T00:00:00Z</published>"));
        assert!(xml.contains("<content type=\"html\">"));
        assert!(xml.contains("<category term=\"rust\"/>"));
    }

    #[test]
    fn test_atom_entry_id_defaults_to_link() {
        let entry = AtomEntry::new("Post", "https://example.com/post").updated("2026-01-01");
        let xml = entry.render_xml();
        assert!(xml.contains("<id>https://example.com/post</id>"));
    }

    #[test]
    fn test_atom_into_render_result() {
        let feed = AtomFeed::new("Blog", "https://example.com");
        let result: RenderResult = feed.into();
        match result {
            RenderResult::Raw(bytes) => {
                let s = String::from_utf8(bytes).unwrap();
                assert!(s.contains("Atom"));
            }
            _ => panic!("expected RenderResult::Raw"),
        }
    }
}
