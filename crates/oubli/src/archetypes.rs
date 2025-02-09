use maudit::{
    content::{glob_markdown, ContentSource, UntypedMarkdownContent},
    page::FullPage,
};
// Import the blog archetype pages.
mod blog;
use crate::archetypes::blog::{BlogEntry, BlogEntryContent, BlogIndex};

/// Oubli provides Archetypes to help you quickly scaffold common types of content, like blogs or documentation.
#[derive(Debug, Clone)]
pub enum Archetype {
    /// Represents a markdown blog archetype.
    Blog(BlogEntryContent),
    /// Represents a markdown documentation archetype.
    MarkdownDoc(UntypedMarkdownContent),
}

/// Builds an archetype based on the provided name, type, and glob pattern. Returns the routes and content source.
///
/// ## Example
/// ```rust
/// use oubli::{Archetype, build_archetype, content_sources, routes, BuildOptions, BuildOutput};
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///   let archetypes = &[ (Archetype::Blog, "content/blog/**/*.md") ];
///   forget(
///     archetypes,
///     routes![],
///     content_sources![],
///     BuildOptions::default(),
///   )
/// }
/// ```
pub fn build_archetype(
    name: &str,
    archetype: Archetype,
    glob: &str,
) -> (Vec<&'static dyn FullPage>, ContentSource<Archetype>) {
    // Generate the content source
    let content_source = ContentSource::new(
        name,
        Box::new(move || glob_markdown::<BlogEntryContent>(glob)),
    );

    // Generate the routes based on the archetype
    let routes: Vec<&'static dyn FullPage> = match archetype {
        Archetype::Blog(_) => {
            static BLOG_INDEX: BlogIndex = BlogIndex;
            static BLOG_ENTRY: BlogEntry = BlogEntry;
            vec![&BLOG_INDEX, &BLOG_ENTRY]
        }
        Archetype::MarkdownDoc(_) => {
            // To be implemented: return routes for a markdown documentation archetype.
            vec![]
        }
    };

    // Return the final tuple
    (routes, content_source)
}

pub trait NimporteQuoi {}
