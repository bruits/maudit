use std::fs;
use std::path::Path;

use maudit::content::markdown_entry;
use maudit::content::{ContentSource, ContentSources, glob_markdown};
use maudit::route::prelude::*;
use maudit::{BuildOptions, coronate};

// -- Content types --

#[markdown_entry]
#[derive(Debug, Clone)]
pub struct ArticleContent {
    pub title: String,
    pub description: String,
}

// -- Routes --

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

// -- Helpers --

fn write_markdown(dir: &Path, filename: &str, title: &str, description: &str, body: &str) {
    let content = format!(
        "---\ntitle: \"{}\"\ndescription: \"{}\"\n---\n\n{}",
        title, description, body
    );
    fs::write(dir.join(filename), content).unwrap();
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

fn routes() -> &'static [&'static dyn FullRoute] {
    &[&IndexPage, &AboutPage, &ArticlePage]
}

// -- Tests --

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
