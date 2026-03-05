use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use maudit::content::markdown_entry;
use maudit::content::{ContentSource, ContentSources, glob_markdown};
use maudit::route::prelude::*;
use maudit::{BuildOptions, coronate};

#[markdown_entry]
#[derive(Debug, Clone)]
pub struct ArticleContent {
    pub title: String,
    pub description: String,
}

#[route("/")]
pub struct IndexPage;

impl Route for IndexPage {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let articles = ctx.content::<ArticleContent>("articles");

        let mut html = String::from("<html><body><h1>Index</h1><ul>");
        for entry in articles.entries() {
            let data = entry.data(ctx);
            html.push_str(&format!("<li>{}</li>", data.title));
        }
        html.push_str("</ul></body></html>");
        html
    }
}

#[route("/about")]
pub struct AboutPage;

impl Route for AboutPage {
    fn render(&self, _ctx: &mut PageContext) -> impl Into<RenderResult> {
        "<html><body><h1>About</h1></body></html>"
    }
}

#[route("/articles/[article]")]
pub struct ArticlePage;

#[derive(Params, Clone)]
pub struct ArticleParams {
    pub article: String,
}

impl Route<ArticleParams> for ArticlePage {
    fn pages(&self, ctx: &mut DynamicRouteContext) -> Pages<ArticleParams> {
        let articles = ctx.content::<ArticleContent>("articles");
        articles.into_pages(|entry| {
            Page::from_params(ArticleParams {
                article: entry.id.clone(),
            })
        })
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let params = ctx.params::<ArticleParams>();
        let articles = ctx.content::<ArticleContent>("articles");
        let article = articles.get_entry(&params.article);
        let data = article.data(ctx);
        format!(
            "<html><body><h1>{}</h1><p>{}</p></body></html>",
            data.title, data.description
        )
    }
}

#[markdown_entry]
#[derive(Debug, Clone)]
pub struct ProjectContent {
    pub title: String,
    pub description: String,
}

static IMAGE_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

#[route("/with-image")]
pub struct ImagePage;

impl Route for ImagePage {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let image_path = IMAGE_PATH.lock().unwrap().clone().unwrap();
        let image = ctx.assets.add_image_unchecked(&image_path);
        let placeholder = image.placeholder().unwrap();
        format!(
            "<html><body><img src=\"{}\" data-placeholder=\"{}\" /></body></html>",
            image.url(),
            placeholder.thumbhash_base64
        )
    }
}

static STYLE_PATH_1: Mutex<Option<PathBuf>> = Mutex::new(None);
static STYLE_PATH_2: Mutex<Option<PathBuf>> = Mutex::new(None);

#[route("/styled")]
pub struct StyledPage;

impl Route for StyledPage {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let style_path = STYLE_PATH_1.lock().unwrap().clone().unwrap();
        ctx.assets
            .include_style(&style_path)
            .expect("Failed to include style");
        "<html><head></head><body><h1>Styled</h1></body></html>"
    }
}

#[route("/styled2")]
pub struct StyledPage2;

impl Route for StyledPage2 {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let style_path = STYLE_PATH_2.lock().unwrap().clone().unwrap();
        ctx.assets
            .include_style(&style_path)
            .expect("Failed to include style");
        "<html><head></head><body><h1>Styled 2</h1></body></html>"
    }
}

#[route("/featured")]
pub struct FeaturedArticlePage;

impl Route for FeaturedArticlePage {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let articles = ctx.content::<ArticleContent>("articles");
        match articles.get_entry_safe("first") {
            Some(entry) => {
                let data = entry.data(ctx);
                format!(
                    "<html><body><h1>Featured: {}</h1></body></html>",
                    data.title
                )
            }
            None => "<html><body><h1>No featured article</h1></body></html>".to_string(),
        }
    }
}

static SAFE_ENTRY_ID: Mutex<Option<String>> = Mutex::new(None);

#[route("/safe-lookup")]
pub struct SafeLookupPage;

impl Route for SafeLookupPage {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let articles = ctx.content::<ArticleContent>("articles");
        let entry_id = SAFE_ENTRY_ID.lock().unwrap().clone().unwrap();
        match articles.get_entry_safe(&entry_id) {
            Some(entry) => {
                let data = entry.data(ctx);
                format!(
                    "<html><body><h1>Found: {}</h1></body></html>",
                    data.title
                )
            }
            None => "<html><body><h1>Not found</h1></body></html>".to_string(),
        }
    }
}

#[route("/projects")]
pub struct ProjectsIndexPage;

impl Route for ProjectsIndexPage {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let projects = ctx.content::<ProjectContent>("projects");

        let mut html = String::from("<html><body><h1>Projects</h1><ul>");
        for entry in projects.entries() {
            let data = entry.data(ctx);
            html.push_str(&format!("<li>{}</li>", data.title));
        }
        html.push_str("</ul></body></html>");
        html
    }
}

#[route("/projects/[project]")]
pub struct ProjectPage;

#[derive(Params, Clone)]
pub struct ProjectParams {
    pub project: String,
}

impl Route<ProjectParams> for ProjectPage {
    fn pages(&self, ctx: &mut DynamicRouteContext) -> Pages<ProjectParams> {
        let projects = ctx.content::<ProjectContent>("projects");
        projects.into_pages(|entry| {
            Page::from_params(ProjectParams {
                project: entry.id.clone(),
            })
        })
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let params = ctx.params::<ProjectParams>();
        let projects = ctx.content::<ProjectContent>("projects");
        let project = projects.get_entry(&params.project);
        let data = project.data(ctx);
        format!(
            "<html><body><h1>{}</h1><p>{}</p></body></html>",
            data.title, data.description
        )
    }
}


fn write_markdown(dir: &Path, filename: &str, title: &str, description: &str, body: &str) {
    let content = format!(
        "---\ntitle: \"{}\"\ndescription: \"{}\"\n---\n\n{}",
        title, description, body
    );
    fs::write(dir.join(filename), content).unwrap();
}

/// Create a minimal valid 1x1 red PNG file.
fn write_minimal_png(path: &Path) {
    #[rustfmt::skip]
    let png: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
        0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,
        0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
        0x00, 0x03, 0x01, 0x01, 0x00, 0xC9, 0xFE, 0x92,
        0xEF, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
        0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    fs::write(path, png).unwrap();
}

fn routes_with_image() -> &'static [&'static dyn FullRoute] {
    &[&IndexPage, &AboutPage, &ArticlePage, &ImagePage]
}

fn build_options(tmp: &Path) -> BuildOptions {
    BuildOptions {
        output_dir: tmp.join("dist"),
        static_dir: tmp.join("static"),
        clean_output_dir: false,
        incremental: true,
        cache_dir: tmp.join("cache"),
        ..Default::default()
    }
}

fn make_content_sources(content_dir: &Path) -> ContentSources {
    let pattern = content_dir
        .join("articles/*.md")
        .to_string_lossy()
        .to_string();
    ContentSources::new(vec![Box::new(ContentSource::new(
        "articles",
        Box::new(move || glob_markdown::<ArticleContent>(&pattern)),
    ))])
}

fn rendered_routes(output: &maudit::BuildOutput) -> Vec<String> {
    output
        .pages
        .iter()
        .filter(|p| !p.cached)
        .map(|p| p.route.clone())
        .collect()
}

fn cached_routes(output: &maudit::BuildOutput) -> Vec<String> {
    output
        .pages
        .iter()
        .filter(|p| p.cached)
        .map(|p| p.route.clone())
        .collect()
}

#[route("/feed.xml", always_revalidate)]
pub struct FeedPage;

impl Route for FeedPage {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let articles = ctx.content::<ArticleContent>("articles");
        let mut xml = String::from("<rss><channel>");
        for entry in articles.entries() {
            let data = entry.data(ctx);
            xml.push_str(&format!("<item><title>{}</title></item>", data.title));
        }
        xml.push_str("</channel></rss>");
        xml
    }
}

fn routes() -> &'static [&'static dyn FullRoute] {
    &[&IndexPage, &AboutPage, &ArticlePage]
}

fn routes_with_feed() -> &'static [&'static dyn FullRoute] {
    &[&IndexPage, &AboutPage, &ArticlePage, &FeedPage]
}

fn routes_with_featured() -> &'static [&'static dyn FullRoute] {
    &[&IndexPage, &AboutPage, &ArticlePage, &FeaturedArticlePage]
}

fn routes_with_safe_lookup() -> &'static [&'static dyn FullRoute] {
    &[&IndexPage, &AboutPage, &ArticlePage, &SafeLookupPage]
}

fn routes_with_styled1() -> &'static [&'static dyn FullRoute] {
    &[&IndexPage, &AboutPage, &ArticlePage, &StyledPage]
}

fn routes_with_styled2() -> &'static [&'static dyn FullRoute] {
    &[&IndexPage, &AboutPage, &ArticlePage, &StyledPage2]
}

fn multi_routes() -> &'static [&'static dyn FullRoute] {
    &[
        &IndexPage,
        &AboutPage,
        &ArticlePage,
        &ProjectsIndexPage,
        &ProjectPage,
    ]
}

fn make_multi_content_sources(content_dir: &Path) -> ContentSources {
    let articles_pattern = content_dir
        .join("articles/*.md")
        .to_string_lossy()
        .to_string();
    let projects_pattern = content_dir
        .join("projects/*.md")
        .to_string_lossy()
        .to_string();
    ContentSources::new(vec![
        Box::new(ContentSource::new(
            "articles",
            Box::new(move || glob_markdown::<ArticleContent>(&articles_pattern)),
        )),
        Box::new(ContentSource::new(
            "projects",
            Box::new(move || glob_markdown::<ProjectContent>(&projects_pattern)),
        )),
    ])
}

#[test]
fn test_full_build_renders_all_pages() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );
    write_markdown(
        &content_dir.join("articles"),
        "second.md",
        "Second Post",
        "The second post",
        "Goodbye world",
    );

    let output = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // First build: all pages should be rendered (not cached)
    assert_eq!(output.pages.len(), 4); // index + about + 2 articles
    assert!(output.pages.iter().all(|p| !p.cached));

    // Verify output files exist
    assert!(tmp.path().join("dist/index.html").exists());
    assert!(tmp.path().join("dist/about/index.html").exists());
    assert!(tmp.path().join("dist/articles/first/index.html").exists());
    assert!(tmp.path().join("dist/articles/second/index.html").exists());
}

#[test]
fn test_no_changes_all_cached() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // First build
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Second build with no changes
    let output = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // All pages should be cached
    assert_eq!(output.pages.len(), 3); // index + about + 1 article
    let rendered = rendered_routes(&output);
    let cached = cached_routes(&output);
    assert!(
        rendered.is_empty(),
        "expected no rendered pages, got: {:?}",
        rendered
    );
    assert_eq!(cached.len(), 3);
}

#[test]
fn test_change_markdown_only_affected_pages_rebuild() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );
    write_markdown(
        &content_dir.join("articles"),
        "second.md",
        "Second Post",
        "The second post",
        "Goodbye world",
    );

    // First build
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Modify only the first article
    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post Updated",
        "The first post updated",
        "Hello updated world",
    );

    // Second build
    let output = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let rendered = rendered_routes(&output);
    let cached = cached_routes(&output);

    // Index should be re-rendered (iterates all articles via collection().entries())
    assert!(
        rendered.contains(&"/".to_string()),
        "index should be rendered, rendered={:?}",
        rendered
    );
    // About should be cached (no dependency on articles)
    assert!(
        cached.contains(&"/about".to_string()),
        "about should be cached, cached={:?}",
        cached
    );
}

#[test]
fn test_add_new_article() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // First build
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Add a new article
    write_markdown(
        &content_dir.join("articles"),
        "third.md",
        "Third Post",
        "The third post",
        "A new post",
    );

    // Second build
    let output = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // New article page should exist
    assert!(tmp.path().join("dist/articles/third/index.html").exists());

    // Total pages should now be 4 (index + about + first + third)
    assert_eq!(output.pages.len(), 4);

    let rendered = rendered_routes(&output);

    // Index should be re-rendered (source structurally changed)
    assert!(
        rendered.contains(&"/".to_string()),
        "index should be rendered"
    );
    // About should be cached (no dependency on articles)
    let cached = cached_routes(&output);
    assert!(
        cached.contains(&"/about".to_string()),
        "about should be cached"
    );
}

#[test]
fn test_delete_article_removes_output() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );
    write_markdown(
        &content_dir.join("articles"),
        "second.md",
        "Second Post",
        "The second post",
        "Goodbye world",
    );

    // First build
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    assert!(tmp.path().join("dist/articles/second/index.html").exists());

    // Delete the second article
    fs::remove_file(content_dir.join("articles/second.md")).unwrap();

    // Second build
    let output = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // The deleted article's output should be removed
    assert!(
        !tmp.path().join("dist/articles/second/index.html").exists(),
        "deleted article output should be removed"
    );

    // Total pages should be 3 (index + about + first)
    assert_eq!(output.pages.len(), 3);
}

#[test]
fn test_static_page_unchanged_when_content_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // First build
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Modify article
    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post Updated",
        "The first post updated",
        "Hello updated world",
    );

    // Second build
    let output = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let cached = cached_routes(&output);

    // About page has no content dependencies, should always be cached
    assert!(
        cached.contains(&"/about".to_string()),
        "about page should be cached when only content changes, cached={:?}",
        cached
    );
}

#[test]
fn test_output_content_is_correct_after_incremental_rebuild() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // First build
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Verify initial content
    let article_html =
        fs::read_to_string(tmp.path().join("dist/articles/first/index.html")).unwrap();
    assert!(article_html.contains("First Post"));

    // Update the article
    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "Updated Title",
        "Updated description",
        "Updated body",
    );

    // Second build
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Verify the output file was actually updated
    let article_html =
        fs::read_to_string(tmp.path().join("dist/articles/first/index.html")).unwrap();
    assert!(
        article_html.contains("Updated Title"),
        "article output should contain updated content, got: {}",
        article_html
    );

    // About page should still exist and be unchanged
    let about_html = fs::read_to_string(tmp.path().join("dist/about/index.html")).unwrap();
    assert!(about_html.contains("About"));
}

#[test]
fn test_corrupt_cache_falls_back_to_full_build() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // First build
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Corrupt the cache file
    let cache_file = tmp.path().join("cache/build_cache.bin");
    fs::write(&cache_file, b"corrupted data").unwrap();

    // Second build should still succeed (full rebuild)
    let output = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // All pages should be rendered (full build fallback)
    assert_eq!(output.pages.len(), 3);
    assert!(
        output.pages.iter().all(|p| !p.cached),
        "all pages should be rendered after corrupt cache"
    );
}

#[test]
fn test_incremental_disabled_always_renders_all() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // First build
    let mut options = build_options(tmp.path());
    options.incremental = false;
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        options,
    )
    .unwrap();

    // Second build with no changes, incremental disabled
    let mut options = build_options(tmp.path());
    options.incremental = false;
    let output = coronate(
        routes(),
        make_content_sources(&content_dir),
        options,
    )
    .unwrap();

    // All pages should be rendered (not cached) since incremental is off
    assert!(
        output.pages.iter().all(|p| !p.cached),
        "all pages should be rendered when incremental is disabled"
    );
}

#[test]
fn test_second_article_cached_when_first_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );
    write_markdown(
        &content_dir.join("articles"),
        "second.md",
        "Second Post",
        "The second post",
        "Goodbye world",
    );

    // First build
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Modify only the first article
    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post Updated",
        "The first post updated",
        "Hello updated world",
    );

    // Second build
    let output = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Find the second article in the output
    let second_article = output
        .pages
        .iter()
        .find(|p| {
            p.params
                .as_ref()
                .and_then(|params| params.get("article"))
                .and_then(|v| v.as_deref())
                == Some("second")
        });

    assert!(
        second_article.is_some(),
        "second article should be in output"
    );
    assert!(
        second_article.unwrap().cached,
        "second article should be cached when only first article changed"
    );

    // The first article should have been re-rendered
    let first_article = output
        .pages
        .iter()
        .find(|p| {
            p.params
                .as_ref()
                .and_then(|params| params.get("article"))
                .and_then(|v| v.as_deref())
                == Some("first")
        });

    assert!(first_article.is_some(), "first article should be in output");
    assert!(
        !first_article.unwrap().cached,
        "first article should be re-rendered when its content changed"
    );
}

#[test]
fn test_three_builds_progressive_caching() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // Build 1: full build
    let output1 = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();
    assert!(output1.pages.iter().all(|p| !p.cached), "build 1: all rendered");

    // Build 2: no changes -> all cached
    let output2 = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();
    assert!(output2.pages.iter().all(|p| p.cached), "build 2: all cached");

    // Modify the article
    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "Updated",
        "Updated desc",
        "Updated body",
    );

    // Build 3: only affected pages re-rendered
    let output3 = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let rendered3 = rendered_routes(&output3);
    let cached3 = cached_routes(&output3);

    // About should still be cached even after 3 builds
    assert!(
        cached3.contains(&"/about".to_string()),
        "build 3: about should be cached, cached={:?}",
        cached3
    );
    // Index and article should be re-rendered
    assert!(
        rendered3.contains(&"/".to_string()),
        "build 3: index should be rendered"
    );
}

#[test]
fn test_asset_style_change_triggers_rebuild() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    let style_file = tmp.path().join("test1.css");
    fs::write(&style_file, "body { color: red; }").unwrap();
    *STYLE_PATH_1.lock().unwrap() = Some(style_file.clone());

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // Build 1: full build
    let output1 = coronate(
        routes_with_styled1(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();
    assert!(
        output1.pages.iter().all(|p| !p.cached),
        "build 1: all rendered"
    );

    // Build 2: no changes -> all cached
    let output2 = coronate(
        routes_with_styled1(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();
    assert!(
        output2.pages.iter().all(|p| p.cached),
        "build 2: all cached"
    );

    // Modify the CSS file
    fs::write(&style_file, "body { color: blue; }").unwrap();

    // Build 3: style changed
    let output3 = coronate(
        routes_with_styled1(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let rendered3 = rendered_routes(&output3);
    let cached3 = cached_routes(&output3);

    // /styled should be re-rendered (its asset changed)
    assert!(
        rendered3.contains(&"/styled".to_string()),
        "styled page should be re-rendered when CSS changes, rendered={:?}",
        rendered3
    );
    // /about should be cached (no asset dependency)
    assert!(
        cached3.contains(&"/about".to_string()),
        "about should be cached, cached={:?}",
        cached3
    );
}

#[test]
fn test_asset_change_does_not_affect_unrelated_pages() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    let style_file = tmp.path().join("test2.css");
    fs::write(&style_file, "h1 { font-size: 2em; }").unwrap();
    *STYLE_PATH_2.lock().unwrap() = Some(style_file.clone());

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // Build 1
    let _ = coronate(
        routes_with_styled2(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Modify CSS
    fs::write(&style_file, "h1 { font-size: 3em; }").unwrap();

    // Build 2
    let output = coronate(
        routes_with_styled2(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let rendered = rendered_routes(&output);
    let cached = cached_routes(&output);

    // Only /styled2 should rebuild
    assert!(
        rendered.contains(&"/styled2".to_string()),
        "styled2 should be rendered"
    );
    // All other pages should be cached
    assert!(
        cached.contains(&"/about".to_string()),
        "about should be cached"
    );
    assert!(
        cached.contains(&"/".to_string()),
        "index should be cached, cached={:?}",
        cached
    );
}

#[test]
fn test_multiple_sources_independent_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();
    fs::create_dir_all(content_dir.join("projects")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );
    write_markdown(
        &content_dir.join("projects"),
        "alpha.md",
        "Project Alpha",
        "Alpha project",
        "Alpha body",
    );

    // Build 1: full build
    let _ = coronate(
        multi_routes(),
        make_multi_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Modify only the article
    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post Updated",
        "Updated description",
        "Updated body",
    );

    // Build 2
    let output = coronate(
        multi_routes(),
        make_multi_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let rendered = rendered_routes(&output);
    let cached = cached_routes(&output);

    // Article-dependent pages should re-render
    assert!(
        rendered.contains(&"/".to_string()),
        "index (iterates articles) should be rendered, rendered={:?}",
        rendered
    );

    // Project pages should be cached (different source, untouched)
    assert!(
        cached.contains(&"/projects".to_string()),
        "projects index should be cached, cached={:?}",
        cached
    );

    let project_alpha_cached = output.pages.iter().any(|p| {
        p.cached
            && p.params
                .as_ref()
                .and_then(|params| params.get("project"))
                .and_then(|v| v.as_deref())
                == Some("alpha")
    });
    assert!(
        project_alpha_cached,
        "project alpha should be cached when only articles changed"
    );
}

#[test]
fn test_multiple_sources_structural_change_in_one() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();
    fs::create_dir_all(content_dir.join("projects")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello",
    );
    write_markdown(
        &content_dir.join("projects"),
        "alpha.md",
        "Project Alpha",
        "Alpha project",
        "Alpha body",
    );

    // Build 1
    let _ = coronate(
        multi_routes(),
        make_multi_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Add a new project (structural change in "projects" source)
    write_markdown(
        &content_dir.join("projects"),
        "beta.md",
        "Project Beta",
        "Beta project",
        "Beta body",
    );

    // Build 2
    let output = coronate(
        multi_routes(),
        make_multi_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let rendered = rendered_routes(&output);
    let cached = cached_routes(&output);

    // Projects index should re-render (iterates projects, structural change)
    assert!(
        rendered.contains(&"/projects".to_string()),
        "projects index should be rendered, rendered={:?}",
        rendered
    );

    // New project page should exist
    assert!(
        tmp.path()
            .join("dist/projects/beta/index.html")
            .exists(),
        "new project output should exist"
    );

    // Article pages should be cached (articles source untouched)
    assert!(
        cached.contains(&"/about".to_string()),
        "about should be cached"
    );

    let first_article_cached = output.pages.iter().any(|p| {
        p.cached
            && p.params
                .as_ref()
                .and_then(|params| params.get("article"))
                .and_then(|v| v.as_deref())
                == Some("first")
    });
    assert!(
        first_article_cached,
        "first article should be cached when only projects changed"
    );
}

#[test]
fn test_get_entry_safe_tracks_dependencies() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );
    write_markdown(
        &content_dir.join("articles"),
        "second.md",
        "Second Post",
        "The second post",
        "Goodbye world",
    );

    // Build 1: full build
    let _ = coronate(
        routes_with_featured(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Change only the second article (featured depends on "first")
    write_markdown(
        &content_dir.join("articles"),
        "second.md",
        "Second Post Updated",
        "Updated",
        "Updated body",
    );

    // Build 2
    let output = coronate(
        routes_with_featured(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let cached = cached_routes(&output);

    // /featured only depends on "first" via get_entry_safe, should be cached
    assert!(
        cached.contains(&"/featured".to_string()),
        "featured should be cached when only second article changed, cached={:?}",
        cached
    );

    // Now change the first article
    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post Updated",
        "Updated first",
        "Updated first body",
    );

    // Build 3
    let output = coronate(
        routes_with_featured(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let rendered = rendered_routes(&output);

    // /featured should now re-render (its dependency "first" changed)
    assert!(
        rendered.contains(&"/featured".to_string()),
        "featured should be re-rendered when first article changed, rendered={:?}",
        rendered
    );
}

#[test]
fn test_source_emptied_completely() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );
    write_markdown(
        &content_dir.join("articles"),
        "second.md",
        "Second Post",
        "The second post",
        "Goodbye world",
    );

    // Build 1
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    assert!(tmp.path().join("dist/articles/first/index.html").exists());
    assert!(tmp.path().join("dist/articles/second/index.html").exists());

    // Delete all articles
    fs::remove_file(content_dir.join("articles/first.md")).unwrap();
    fs::remove_file(content_dir.join("articles/second.md")).unwrap();

    // Build 2
    let output = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Article output files should be removed
    assert!(
        !tmp.path().join("dist/articles/first/index.html").exists(),
        "first article output should be removed"
    );
    assert!(
        !tmp.path().join("dist/articles/second/index.html").exists(),
        "second article output should be removed"
    );

    // Only index + about remain (no article pages)
    assert_eq!(output.pages.len(), 2, "should only have index + about");

    let rendered = rendered_routes(&output);
    // Index should re-render (structural change — source emptied)
    assert!(
        rendered.contains(&"/".to_string()),
        "index should be rendered after source emptied, rendered={:?}",
        rendered
    );

    let cached = cached_routes(&output);
    // About should be cached
    assert!(
        cached.contains(&"/about".to_string()),
        "about should be cached"
    );
}

#[test]
fn test_get_entry_safe_missing_entry_then_added() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    // "special" doesn't exist yet
    *SAFE_ENTRY_ID.lock().unwrap() = Some("special".to_string());

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // Build 1: full build, safe-lookup renders "Not found"
    let output1 = coronate(
        routes_with_safe_lookup(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();
    assert!(
        output1.pages.iter().all(|p| !p.cached),
        "build 1: all rendered"
    );

    let lookup_html =
        fs::read_to_string(tmp.path().join("dist/safe-lookup/index.html")).unwrap();
    assert!(
        lookup_html.contains("Not found"),
        "should render 'Not found' when entry doesn't exist"
    );

    // Build 2: no changes -> all cached
    let output2 = coronate(
        routes_with_safe_lookup(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();
    assert!(
        output2.pages.iter().all(|p| p.cached),
        "build 2: all cached"
    );

    // Add "special.md" (structural change in source)
    write_markdown(
        &content_dir.join("articles"),
        "special.md",
        "Special Post",
        "A special post",
        "Special body",
    );

    // Build 3: safe-lookup should re-render
    let output3 = coronate(
        routes_with_safe_lookup(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let rendered3 = rendered_routes(&output3);

    // safe-lookup depends on "special" via get_entry_safe — source structurally changed
    // so it should re-render
    assert!(
        rendered3.contains(&"/safe-lookup".to_string()),
        "safe-lookup should re-render when entry is added, rendered={:?}",
        rendered3
    );

    // Verify the content updated
    let lookup_html =
        fs::read_to_string(tmp.path().join("dist/safe-lookup/index.html")).unwrap();
    assert!(
        lookup_html.contains("Found: Special Post"),
        "should now render the found entry, got: {}",
        lookup_html
    );
}

#[test]
fn test_always_revalidate_rebuilds_even_when_clean() {
    let tmp = tempfile::TempDir::new().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "First description",
        "Hello",
    );

    // Build 1: full build, everything rendered
    let output1 = coronate(
        routes_with_feed(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let rendered1 = rendered_routes(&output1);
    assert!(rendered1.contains(&"/feed.xml".to_string()));
    assert!(rendered1.contains(&"/about".to_string()));

    // Build 2: no changes — about should be cached, but feed.xml should always re-render
    let output2 = coronate(
        routes_with_feed(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let rendered2 = rendered_routes(&output2);
    let cached2 = cached_routes(&output2);

    assert!(
        rendered2.contains(&"/feed.xml".to_string()),
        "feed.xml should always re-render, rendered={:?}",
        rendered2
    );
    assert!(
        cached2.contains(&"/about".to_string()),
        "about should be cached, cached={:?}",
        cached2
    );
}

#[test]
fn test_build_cache_saved_with_incremental() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // Build with incremental enabled
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Build cache should exist
    let cache_path = tmp.path().join("cache/build_cache.bin");
    assert!(cache_path.exists(), "build cache should exist after build");

    // Second build should also succeed and keep the cache
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    assert!(
        cache_path.exists(),
        "build cache should still exist after second build"
    );
}

#[test]
fn test_build_cache_saved_without_incremental() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    let non_incremental_options = BuildOptions {
        output_dir: tmp.path().join("dist"),
        static_dir: tmp.path().join("static"),
        clean_output_dir: false,
        incremental: false,
        cache_dir: tmp.path().join("cache"),
        ..Default::default()
    };

    // Build without incremental
    let _ = coronate(
        routes(),
        make_content_sources(&content_dir),
        non_incremental_options,
    )
    .unwrap();

    // Build cache should still exist (for image cache persistence)
    let cache_path = tmp.path().join("cache/build_cache.bin");
    assert!(
        cache_path.exists(),
        "build cache should exist even without incremental builds (for image cache)"
    );
}

#[test]
fn test_image_placeholder_cached_across_builds() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    // Create a test image
    let image_path = tmp.path().join("test_image.png");
    write_minimal_png(&image_path);
    *IMAGE_PATH.lock().unwrap() = Some(image_path.clone());

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // Build 1: generates placeholder from scratch
    let output1 = coronate(
        routes_with_image(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();
    assert!(
        output1.pages.iter().all(|p| !p.cached),
        "build 1: all rendered"
    );

    // Verify the image page was rendered with placeholder data
    let image_html = fs::read_to_string(tmp.path().join("dist/with-image/index.html")).unwrap();
    assert!(
        image_html.contains("data-placeholder="),
        "should have placeholder data in output"
    );

    // Build 2: placeholder should come from cache (no recomputation)
    let output2 = coronate(
        routes_with_image(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let cached2 = cached_routes(&output2);
    assert!(
        cached2.contains(&"/with-image".to_string()),
        "image page should be cached on second build, cached={:?}",
        cached2
    );

    // Image cache should exist as its own file
    let image_cache_path = tmp.path().join("cache/image_cache.bin");
    assert!(image_cache_path.exists(), "image cache file should exist");
    let cache_size = fs::metadata(&image_cache_path).unwrap().len();
    assert!(
        cache_size > 50,
        "image cache should contain meaningful data (got {} bytes)",
        cache_size
    );
}

#[test]
fn test_image_cache_survives_build_cache_invalidation() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    let image_path = tmp.path().join("test_image.png");
    write_minimal_png(&image_path);
    *IMAGE_PATH.lock().unwrap() = Some(image_path.clone());

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // Build 1: generates placeholder from scratch
    let _ = coronate(
        routes_with_image(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let image_cache_path = tmp.path().join("cache/image_cache.bin");
    assert!(image_cache_path.exists(), "image cache should exist after build 1");
    let image_cache_size_1 = fs::metadata(&image_cache_path).unwrap().len();

    // Corrupt the build cache to simulate a version bump / binary change
    let build_cache_path = tmp.path().join("cache/build_cache.bin");
    fs::write(&build_cache_path, b"corrupted").unwrap();

    // Build 2: build cache is invalid, but image cache should survive
    *IMAGE_PATH.lock().unwrap() = Some(image_path.clone());
    let output2 = coronate(
        routes_with_image(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // All pages re-rendered (full build due to corrupt cache)
    assert!(
        output2.pages.iter().all(|p| !p.cached),
        "build 2: all rendered due to corrupt build cache"
    );

    // Image cache file should still exist with same or similar size
    // (it was loaded from its own file, not from the build cache)
    assert!(image_cache_path.exists(), "image cache should survive build cache corruption");
    let image_cache_size_2 = fs::metadata(&image_cache_path).unwrap().len();
    assert!(
        image_cache_size_2 >= image_cache_size_1,
        "image cache should not have shrunk (before={}, after={})",
        image_cache_size_1,
        image_cache_size_2
    );

    // The image page should still have placeholder data (served from image cache)
    let image_html = fs::read_to_string(tmp.path().join("dist/with-image/index.html")).unwrap();
    assert!(
        image_html.contains("data-placeholder="),
        "should have placeholder data even after build cache invalidation"
    );
}

#[test]
fn test_image_cache_gc_not_triggered_on_incremental_build() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    let image_path = tmp.path().join("test_image.png");
    write_minimal_png(&image_path);
    *IMAGE_PATH.lock().unwrap() = Some(image_path.clone());

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // Build 1: full build, image gets cached
    let _ = coronate(
        routes_with_image(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let image_cache_path = tmp.path().join("cache/image_cache.bin");
    let size_after_build1 = fs::metadata(&image_cache_path).unwrap().len();

    // Build 2: incremental, nothing changed — image cache should be preserved
    *IMAGE_PATH.lock().unwrap() = Some(image_path.clone());
    let output2 = coronate(
        routes_with_image(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let cached2 = cached_routes(&output2);
    assert!(
        cached2.contains(&"/with-image".to_string()),
        "image page should be cached"
    );

    let size_after_build2 = fs::metadata(&image_cache_path).unwrap().len();
    assert_eq!(
        size_after_build1, size_after_build2,
        "image cache size should not change on incremental build with no changes"
    );
}

#[test]
fn test_image_cache_persists_across_incremental_toggle() {
    let tmp = tempfile::tempdir().unwrap();
    let content_dir = tmp.path().join("content");
    fs::create_dir_all(content_dir.join("articles")).unwrap();

    let image_path = tmp.path().join("test_image.png");
    write_minimal_png(&image_path);
    *IMAGE_PATH.lock().unwrap() = Some(image_path.clone());

    write_markdown(
        &content_dir.join("articles"),
        "first.md",
        "First Post",
        "The first post",
        "Hello world",
    );

    // Build 1: incremental=true
    let _ = coronate(
        routes_with_image(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    let image_cache_path = tmp.path().join("cache/image_cache.bin");
    assert!(image_cache_path.exists(), "image cache should exist after incremental build");
    let size_after_incremental = fs::metadata(&image_cache_path).unwrap().len();

    // Build 2: incremental=false — image cache should NOT be wiped
    *IMAGE_PATH.lock().unwrap() = Some(image_path.clone());
    let non_incremental_options = BuildOptions {
        incremental: false,
        ..build_options(tmp.path())
    };
    let _ = coronate(
        routes_with_image(),
        make_content_sources(&content_dir),
        non_incremental_options,
    )
    .unwrap();

    assert!(image_cache_path.exists(), "image cache should survive incremental=false build");
    let size_after_non_incremental = fs::metadata(&image_cache_path).unwrap().len();
    assert_eq!(
        size_after_incremental, size_after_non_incremental,
        "image cache should not change when toggling incremental mode"
    );

    // Build 3: back to incremental=true — image cache should still be intact
    *IMAGE_PATH.lock().unwrap() = Some(image_path.clone());
    let output3 = coronate(
        routes_with_image(),
        make_content_sources(&content_dir),
        build_options(tmp.path()),
    )
    .unwrap();

    // Image page should render correctly with cached placeholder
    let image_html = fs::read_to_string(tmp.path().join("dist/with-image/index.html")).unwrap();
    assert!(
        image_html.contains("data-placeholder="),
        "should have placeholder data after toggling incremental modes"
    );

    let size_after_back_to_incremental = fs::metadata(&image_cache_path).unwrap().len();
    assert_eq!(
        size_after_incremental, size_after_back_to_incremental,
        "image cache should remain stable across incremental mode toggles"
    );
}
