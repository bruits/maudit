use maudit::page::prelude::*;

use maudit::{
    content::{ContentSourceInternal, ContentSources},
    coronate,
    page::{prelude::Params, FullPage},
};

// Re-expose Maudit's public API.
pub use maudit::{content_sources, routes, BuildOptions, BuildOutput};
// Expose the archetypes module.
pub mod archetypes {
    pub mod blog;
}
// Expose the layout module.
pub mod layouts {
    mod layout;
    pub use layout::layout;
}
// Expose public components.
pub mod components {}

/// Help you quickly scaffold common types of content, like blogs or documentation.
#[derive(Debug, Clone)]
pub enum Archetype {
    /// Represents a markdown blog archetype.
    Blog,
    /// Represents a markdown documentation archetype.
    MarkdownDoc,
}

#[macro_export]
/// Helps to define every archetype that should be build by [`forget()`].
///
/// ## Example
/// ```rust
/// use oubli::{Archetype, archetypes, content_sources, forget, routes, BuildOptions, BuildOutput};
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///     forget(
///         // Define archetypes and their glob patterns using the provided macro.
///         archetypes![
///             (news, Archetype::Blog, "content/blog/**/*.md")
///         ],
///         routes![],
///         content_sources![],
///         BuildOptions::default(),
///     )
/// }
/// ```
macro_rules! archetypes {
    ($(($name:ident, $arch:expr, $glob:expr)),* $(,)?) => {{
        let mut vec = Vec::new();
        $(
            let tuple = match $arch {
                oubli::Archetype::Blog => {
                    // Generate the content source
                    let content_source = maudit::content::ContentSource::new(
                        stringify!($name),
                        Box::new({
                            let glob = $glob.to_string();
                            move || maudit::content::glob_markdown::<oubli::archetypes::blog::BlogEntryContent>(&glob)
                        }),
                    );
                    // Generate the pages
                    mod $name {
                        use maudit::page::prelude::*;
                        use oubli::archetypes::blog::*;

                        #[route(stringify!($name))]
                        pub struct Index;
                        impl Page for Index {
                            fn render(&self, ctx: &mut RouteContext) -> RenderResult {
                                blog_index_content::<Entry>(Entry, ctx, stringify!($name)).into()
                            }
                        }

                        #[route(concat!(stringify!($name), "/[entry]"))]
                        pub struct Entry;
                        impl Page<BlogEntryParams> for Entry {
                            fn render(&self, ctx: &mut RouteContext) -> RenderResult {
                                blog_entry_render(ctx, stringify!($name)).into()
                            }

                            fn routes(&self, ctx: &mut DynamicRouteContext) -> Vec<BlogEntryParams> {
                                blog_entry_routes(ctx, stringify!($name))
                            }
                        }
                    }
                    (stringify!($name), vec![&$name::Index as &dyn maudit::page::FullPage, &$name::Entry as &dyn maudit::page::FullPage], Box::new(content_source) as Box<dyn maudit::content::ContentSourceInternal>)
                },
                oubli::Archetype::MarkdownDoc => {
                    todo!();
                }
            };
            vec.push(tuple);
        )*
        vec
    }};
}

/// 🪶 Oubli entrypoint. Starts the build process and generates the output files.
///
/// This function wraps Maudit's [`coronate`](maudit::coronate) function and adds an `archetypes` argument.
/// The user can specify one or more archetypes with their corresponding glob pattern. The routes and content
/// sources generated by these archetypes are merged with the routes and content sources provided directly.
///
/// ## Example
/// ```rust
/// use oubli::{Archetype, archetypes, content_sources, forget, routes, BuildOptions, BuildOutput};
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///     forget(
///         // Define archetypes and their glob patterns using the provided macro.
///         archetypes![
///             (news, Archetype::Blog, "content/blog/**/*.md")
///         ],
///         routes![],
///         content_sources![],
///         BuildOptions::default(),
///     )
/// }
/// ```
#[allow(clippy::type_complexity)]
pub fn forget(
    archetypes: Vec<(&str, Vec<&dyn FullPage>, Box<dyn ContentSourceInternal>)>,
    routes: &[&dyn FullPage],
    mut content_sources: ContentSources,
    options: BuildOptions,
) -> Result<BuildOutput, Box<dyn std::error::Error>> {
    // Let's merge the routes and content sources from the archetypes to the user-provided ones.
    let mut combined_routes = routes.to_vec();
    let mut content_sources_archetypes = vec![];
    let mut data_store = generate_data_store(archetypes);
    let mut combined_content_sources = ContentSources::new(content_sources_archetypes);

    for (_name, pages, content_source) in archetypes {
        content_sources_archetypes.push(content_source);
        combined_routes.extend(pages.iter());
    }

    combined_content_sources.0.append(&mut content_sources.0);
    combined_content_sources.0.append(&mut data_store);

    // At the end of the day, we are just a Maudit wrapper.
    coronate(&combined_routes, combined_content_sources, options)
}

/// # Generates a content source with every provided archetype.
fn generate_data_store(
    archetypes: Vec<(&str, Vec<&dyn FullPage>, Box<dyn ContentSourceInternal>)>,
) -> Vec<Box<dyn ContentSourceInternal>> {
    let data_source = maudit::content::ContentSource::new(
        "data_store",
        Box::new({
            let mut entries = Vec::new();
            for (name, _pages, _content_source) in archetypes {
                let entry = maudit::content::ContentEntry {
                    id: name.to_string(),
                    render: None,
                    raw_content: None,
                    data: maudit::content::Untyped::default(),
                    file_path: None,
                };
                entries.push(entry);
            }
            entries
        }),
    );
    vec![Box::new(data_source) as Box<dyn maudit::content::ContentSourceInternal>]
}

#[derive(Params)]
struct DataStoreEntry {}
