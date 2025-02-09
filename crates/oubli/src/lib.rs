use maudit::{content::ContentSources, coronate, page::FullPage};

pub use maudit::{content_sources, routes, BuildOptions, BuildOutput};

// Import archetype types and constructor.
mod archetypes;
use archetypes::{build_archetype, Archetype};

// Expose the layout module.
pub mod layouts {
    mod layout;
    pub use layout::layout;
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
pub fn forget(
    archetypes: &[(&str, Archetype, &str)],
    routes: &[&dyn FullPage],
    mut content_sources: ContentSources,
    options: BuildOptions,
) -> Result<BuildOutput, Box<dyn std::error::Error>> {
    // Start with user-custom routes.
    let mut combined_routes: Vec<&dyn FullPage> = routes.to_vec();

    // Process each archetype by generating its routes and content sources,
    // then combine them with the user-custom ones.
    for (name, arch, glob) in archetypes {
        let (arch_routes, arch_source) = build_archetype(name, *arch, glob);
        combined_routes.extend(arch_routes);
        content_sources.0.push(Box::new(arch_source));
    }

    coronate(&combined_routes, content_sources, options)
}
