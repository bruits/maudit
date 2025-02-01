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
pub mod maud {
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
macro_rules! routes {
    [$($route:path),*] => {
        &[$(&$route),*]
    };
}

#[macro_export]
macro_rules! content_sources {
    ($($name:expr => $entries:expr),*) => {
        maudit::content::ContentSources(vec![$(Box::new(maudit::content::ContentSource::new($name, Box::new(move || $entries)))),*])
    };
}

pub const GENERATOR: &str = concat!("Maudit v", env!("CARGO_PKG_VERSION"));

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
