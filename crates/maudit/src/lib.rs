#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]
//!
//! <div class="warning">
//! You are currently reading Maudit API reference. For a more gentle introduction, please refer to our <a href="https://maudit.org/docs">documentation</a>.
//! </div>

// Modules the end-user will interact directly or indirectly with
pub mod assets;
pub mod content;
pub mod errors;
pub mod route;

mod routing;

// Exports for end-users
pub use build::metadata::{BuildOutput, PageOutput, StaticAssetOutput};
pub use build::options::{AssetHashingStrategy, AssetsOptions, BuildOptions};

// Re-export FxHashMap so that macro-generated code can use it without requiring users to add it as a dependency.
#[doc(hidden)]
pub use rustc_hash::FxHashMap;

mod build;
mod templating;

#[cfg(feature = "maud")]
#[cfg_attr(docsrs, doc(cfg(feature = "maud")))]
pub mod maud {
    //! Traits and methods for [Maud](https://maud.lambda.xyz), a macro for writing HTML templates.
    //!
    //! ## Example
    //! ```rs
    //! use maudit::route::prelude::*;
    //! use maud::{html, Markup};
    //!
    //! #[route("/")]
    //! pub struct Index;
    //!
    //! impl Route<PageParams, (), Markup> for Index {
    //!   fn render(&self, ctx: &mut PageContext) -> Markup {
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
use route::FullRoute;

/// Returns whether Maudit is running in development mode (through `maudit dev`).
///
/// This can be useful to conditionally enable features or logging that should only be active during development.
/// Oftentimes, this is used to disable some expensive operations that would slow down build times during development.
pub fn is_dev() -> bool {
    if option_env!("MAUDIT_DEV") == Some("true") {
        return true;
    }

    env::var("MAUDIT_DEV").map(|v| v == "true").unwrap_or(false)
}

#[macro_export]
/// Helps to define every route that should be build by [`coronate()`].
///
/// ## Example
/// ```rs
/// use maudit::{
///   content_sources, coronate, routes, BuildOptions, BuildOutput,
/// };
///
/// # mod routes {
/// #   use maudit::route::prelude::*;
/// #
/// #   #[route("/")]
/// #   pub struct Index;
/// #   impl Route<PageParams, (), String> for Index {
/// #      fn render(&self, _ctx: &mut PageContext) -> String {
/// #          "Hello, world!".to_string()
/// #      }
/// #   }
/// #   #[route("/article")]
/// #   pub struct Article;
/// #
/// #   impl Route<PageParams, (), String> for Article {
/// #      fn render(&self, _ctx: &mut PageContext) -> String {
/// #          "Hello, world!".to_string()
/// #      }
/// #   }
/// # }
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///     coronate(
///         routes![routes::Index, routes::Article],
///         content_sources![],
///         BuildOptions::default(),
///     )
/// }
/// ```
///
macro_rules! routes {
    [$($route:expr),*] => {
        &[$(&$route),*]
    };
}

/// Helps to define all sources of content that should be loaded by [`coronate()`].
///
/// ## Example
/// ```rs
/// use maudit::{
///  content_sources, coronate, routes, BuildOptions, BuildOutput,
/// };
/// use maudit::content::{glob_markdown, markdown_entry};
///
/// #[markdown_entry]
/// pub struct ArticleContent {
///   pub title: String,
///   pub description: String,
/// }
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///    coronate(
///       routes![],
///      content_sources![
///        "articles" => glob_markdown::<ArticleContent>("content/articles/*.md", None)
///     ],
///    BuildOptions::default(),
/// )
/// }
/// ```
///
/// ## Expand
/// ```rs
/// # use maudit::{content_sources};
/// # use maudit::content::{glob_markdown, markdown_entry};
/// # #[markdown_entry]
/// # pub struct ArticleContent {
/// #   pub title: String,
/// #   pub description: String,
/// # }
///
/// content_sources![
///    "articles" => glob_markdown::<ArticleContent>("content/articles/*.md", None)
/// ];
/// ```
/// expands to
/// ```rs
/// # use maudit::content::{glob_markdown, markdown_entry};
/// # #[markdown_entry]
/// # pub struct ArticleContent {
/// #   pub title: String,
/// #   pub description: String,
/// # }
///
/// maudit::content::ContentSources(vec![
///    Box::new(maudit::content::ContentSource::new("articles", Box::new(move || glob_markdown::<ArticleContent>("content/articles/*.md", None))))
/// ]);
#[macro_export]
macro_rules! content_sources {
    ($($name:expr => $entries:expr),*) => {
        maudit::content::ContentSources::new(vec![$(Box::new(maudit::content::ContentSource::new($name, Box::new(move || $entries)))),*])
    };
}
/// The version of Maudit being used.
///
/// Can be used to create a generator tag in the output HTML.
///
/// ## Example
/// ```rs
/// use maudit::GENERATOR;
///
/// format!("<meta name=\"generator\" content=\"{}\">", GENERATOR);
/// ```
pub const GENERATOR: &str = concat!("Maudit v", env!("CARGO_PKG_VERSION"));

/// 👑 Maudit entrypoint. Starts the build process and generates the output files.
///
/// ## Example
/// Should be called from the main function of the binary crate.
/// ```rs
/// use maudit::{
///  content_sources, coronate, routes, BuildOptions, BuildOutput,
/// };
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
    routes: &[&dyn FullRoute],
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
