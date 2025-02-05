#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../../../README.md")]
//!
//! <div class="warning">
//! You are currently reading Maudit API reference. For a more gentle introduction, please refer to our <a href="https://maudit.dev/docs">documentation</a>.
//! </div>

// Modules the end-user will interact directly or indirectly with
mod assets;
pub mod content;
pub mod errors;
pub mod page;
pub mod params;

// Exports for end-users
pub use build::metadata::{BuildOutput, PageOutput, StaticAssetOutput};
pub use build::options::BuildOptions;

// Re-exported dependencies for user convenience
pub use rustc_hash::FxHashMap;

mod build;
mod templating;

#[cfg(feature = "maud")]
#[cfg_attr(docsrs, doc(cfg(feature = "maud")))]
pub mod maud {
    //! Allows to use [Maud](https://maud.lambda.xyz), a macro for writing HTML templates in Rust.
    //!
    //! Maudit supports Maud by default, but you can use your own templating engine.
    //!
    //! ## Example
    //! ```rust
    //! use maudit::page::prelude::*;
    //! use maud::{html, Markup};
    //!
    //! #[route("/")]
    //! pub struct Index;
    //!
    //! impl Page<Markup> for Index {
    //!   fn render(&self, ctx: &mut RouteContext) -> Markup {
    //!     html! {
    //!       h1 { "Hello, world!" }
    //!     }
    //!   }
    //! }
    //! ```
    pub use crate::templating::maud_ext::*;
}

// Internal modules
mod logging;

use std::env;

use build::execute_build;
use content::ContentSources;
use logging::init_logging;
use page::FullPage;

#[macro_export]
/// Helps to define every route that should be build by [`coronate()`].
///
/// ## Example
/// ```rust
/// use maudit::{
///   content_sources, coronate, routes, BuildOptions, BuildOutput,
/// }
/// use crate::pages::{Index, Article};
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///     coronate(
///         routes![pages::Index, pages::Article],
///         content_sources![],
///         BuildOptions::default(),
///     )
/// }
/// ```
///
/// ## Expand
/// ```rust
/// routes![pages::Index, pages::Article]
/// ```
/// expands to
/// ```rust
/// &[
///   &pages::Index,
///   &pages::Article,
/// ]
/// ```
///
macro_rules! routes {
    [$($route:path),*] => {
        &[$(&$route),*]
    };
}

/// Helps to define all sources of content that should be loaded by [`coronate()`].
///
/// ## Example
/// ```rust
/// use maudit::{
///  content_sources, coronate, routes, BuildOptions, BuildOutput,
/// }
/// use crate::content::ArticleContent;
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///    coronate(
///       routes![],
///      content_sources![
///        "articles" => glob_markdown::<ArticleContent>("content/articles/*.md")
///     ],
///    BuildOptions::default(),
/// )
/// }
/// ```
///
/// ## Expand
/// ```rust
/// content_sources![
///    "articles" => glob_markdown::<ArticleContent>("content/articles/*.md")
/// ]
/// ```
/// expands to
/// ```rust
/// maudit::content::ContentSources(vec![
///    Box::new(maudit::content::ContentSource::new("articles", Box::new(move || glob_markdown::<ArticleContent>("content/articles/*.md"))))
/// ])
#[macro_export]
macro_rules! content_sources {
    ($($name:expr => $entries:expr),*) => {
        maudit::content::ContentSources(vec![$(Box::new(maudit::content::ContentSource::new($name, Box::new(move || $entries)))),*])
    };
}
/// The version of Maudit being used.
///
/// Can be used to create a generator tag in the output HTML.
pub const GENERATOR: &str = concat!("Maudit v", env!("CARGO_PKG_VERSION"));

/// 👑 Maudit entrypoint. Starts the build process and generates the output files.
///
/// ## Example
/// Should be called from the main function of the binary crate.
/// ```rust
/// use maudit::{
///  content_sources, coronate, routes, BuildOptions, BuildOutput,
/// }
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///   coronate(
///     routes![],
///     content_sources![],
///     BuildOptions::default(),
///   )
/// }
/// ```
pub fn coronate(
    routes: &[&dyn FullPage],
    mut content_sources: ContentSources,
    options: BuildOptions,
) -> Result<BuildOutput, Box<dyn std::error::Error>> {
    init_logging();

    let async_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    execute_build(routes, &mut content_sources, &options, &async_runtime)
}
