use maudit::{
    content::{ContentSourceInternal, ContentSources},
    coronate,
    page::FullPage,
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

                        pub struct Index;
                        impl maudit::page::InternalPage for Index {
                            fn route_type(&self) -> maudit::page::RouteType {
                                maudit::page::RouteType::Static
                            }
                            fn route_raw(&self) -> String {
                                stringify!($name).to_string()
                            }
                            fn route(&self, _params: &maudit::page::RouteParams) -> String {
                                format!("{}", stringify!($name))
                            }
                            fn file_path(&self, _params: &maudit::page::RouteParams) -> std::path::PathBuf {
                                std::path::PathBuf::from(format!("{}/index.html", stringify!($name)))
                            }
                            fn url_unsafe<P: Into<maudit::page::RouteParams>>(_params: P) -> String {
                                format!("{}", stringify!($name))
                            }
                            fn url_untyped(&self, _params: &maudit::page::RouteParams) -> String {
                                format!("{}", stringify!($name))
                            }
                        }
                        impl maudit::page::FullPage for Index {
                            fn render_internal(&self, ctx: &mut maudit::page::RouteContext) -> maudit::page::RenderResult {
                                self.render(ctx).into()
                            }
                            fn routes_internal(&self, _ctx: &mut maudit::page::DynamicRouteContext) -> Vec<maudit::page::RouteParams> {
                                Vec::new()
                            }
                        }
                        impl Page for Index {
                            fn render(&self, ctx: &mut RouteContext) -> RenderResult {
                                blog_index_content::<Entry>(ctx, stringify!($name)).into()
                            }
                        }

                        pub struct Entry;
                        impl maudit::page::InternalPage for Entry {
                            fn route_type(&self) -> maudit::page::RouteType {
                                maudit::page::RouteType::Dynamic
                            }
                            fn route_raw(&self) -> String {
                                format!("{}/[entry]", stringify!($name))
                            }
                            fn route(&self, params: &maudit::page::RouteParams) -> String {
                                let entry = params.0.get("entry").expect("required param 'entry'").to_string();
                                format!("{}/{}", stringify!($name), entry)
                            }
                            fn file_path(&self, params: &maudit::page::RouteParams) -> std::path::PathBuf {
                                let entry = params.0.get("entry").expect("required param 'entry'").to_string();
                                std::path::PathBuf::from(format!("{}/{}/index.html", stringify!($name), entry))
                            }
                            fn url_unsafe<P: Into<maudit::page::RouteParams>>(params: P) -> String {
                                let params = params.into();
                                let entry = params.0.get("entry").expect("required param 'entry'").to_string();
                                format!("{}/{}", stringify!($name), entry)
                            }
                            fn url_untyped(&self, params: &maudit::page::RouteParams) -> String {
                                let entry = params.0.get("entry").expect("required param 'entry'").to_string();
                                format!("{}/{}", stringify!($name), entry)
                            }
                        }
                        impl maudit::page::FullPage for Entry {
                            fn render_internal(&self, ctx: &mut maudit::page::RouteContext) -> maudit::page::RenderResult {
                                self.render(ctx).into()
                            }
                            fn routes_internal(&self, ctx: &mut maudit::page::DynamicRouteContext) -> Vec<maudit::page::RouteParams> {
                                self.routes(ctx).iter().map(Into::into).collect()
                            }
                        }
                        impl DynamicRoute<BlogEntryParams> for Entry {
                            fn routes(&self, ctx: &mut DynamicRouteContext) -> Vec<BlogEntryParams> {
                                blog_entry_routes(ctx, stringify!($name))
                            }
                        }
                        impl Page for Entry {
                            fn render(&self, ctx: &mut RouteContext) -> RenderResult {
                                blog_entry_render(ctx, stringify!($name)).into()
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
/// use oubli::{Archetype, build_archetype, content_sources, forget, routes, BuildOptions, BuildOutput};
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///   // Define archetypes and their glob patterns
///   let archetypes = &[
///     ("news", Archetype::Blog, "content/blog/**/*.md"),
///     ("docs", Archetype::MarkdownDoc, "content/docs/**/*.md"),
///     ("reference"), Archetype::OpenAPI, "content/reference/**/*.yaml"),
///   ];
///   forget(
///     archetypes,
///     routes![],
///     content_sources![],
///     BuildOptions::default(),
///   )
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

    for (_name, pages, content_source) in archetypes {
        content_sources_archetypes.push(content_source);
        combined_routes.extend(pages.iter());
    }

    let mut combined_content_sources = ContentSources::new(content_sources_archetypes);
    combined_content_sources.0.append(&mut content_sources.0);

    // At the end of the day, we are just a Maudit wrapper.
    coronate(&combined_routes, combined_content_sources, options)
}
