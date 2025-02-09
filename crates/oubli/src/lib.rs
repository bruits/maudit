use maudit::{content::ContentSources, coronate, page::FullPage};

pub use maudit::{content_sources, routes, BuildOptions, BuildOutput};

/// 🪶 Oubli entrypoint. Starts the build process and generates the output files.
///
/// Wrap Maudit's [`coronate`](maudit::coronate) function, which allows to pass custom routes and content sources.
///
/// ## Example
/// Should be called from the main function of the binary crate.
/// ```rust
/// use oubli::{
///   content_sources, forget, routes, BuildOptions, BuildOutput,
/// }
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///   forget(
///     collections![],
///     routes![],
///     content_sources![],
///     BuildOptions::default(),
///   )
/// }
/// ```
pub fn forget(
    routes: &[&dyn FullPage],
    content_sources: ContentSources,
    options: BuildOptions,
) -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(routes, content_sources, options)
}
