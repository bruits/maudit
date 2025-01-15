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

#[cfg(feature = "ipc")]
mod ipc;

// Internal modules
mod logging;

use std::env;

use build::execute_build;
use content::ContentSources;
use ipc::setup_ipc_server;
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

#[cfg(feature = "ipc")]
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

    // If `--ipc-server` argument is found, start IPC server
    if let Some(ipc_server_index) = env::args().position(|arg| arg == "--ipc-server") {
        let ipc_server_name = env::args().nth(ipc_server_index + 1).unwrap();
        return setup_ipc_server(
            &ipc_server_name,
            routes,
            content_sources,
            options,
            &async_runtime,
        );
    }

    execute_build(routes, &mut content_sources, &options, &async_runtime)
}

#[cfg(not(feature = "ipc"))]
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

    if let Some(ipc_server_index) = env::args().position(|arg| arg == "--ipc-server") {
        eprintln!("IPC server is not enabled. Either remove the `--ipc-server` argument, or add the `ipc` feature to your `Cargo.toml`.");
        return Err("IPC server is not enabled.".into());
    }

    execute_build(routes, &mut content_sources, &options, &async_runtime)
}
