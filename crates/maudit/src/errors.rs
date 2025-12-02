//! Error types for Maudit.
use std::fmt::{self, Debug, Formatter};
use std::path::PathBuf;
use thiserror::Error;

macro_rules! impl_debug_for_error {
    ($($t:ty),*) => {
        $(
            impl Debug for $t {
                fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                    // Rust's uses the Debug trait to show errors when they're returned from main
                    // But, thiserror uses the Display trait to show errors. This redirects Debug to Display, essentially.
                    // TODO: Take over error rendering completely when using coronate
                    write!(f, "{}", self)
                }
            }
        )*
    };
}

#[derive(Error)]
pub enum UrlError {
    // TODO: Add contextual information and more details
    #[error("Route not found")]
    RouteNotFound,
}

#[derive(Error)]
pub enum BuildError {
    #[error(
        "`{route}` returns `RenderResult::Raw`, but includes styles or scripts, which can only be included in HTML. If you meant to return HTML, use `RenderResult::Text` instead. Alternatively, if you meant to add a reference to a script or style without including it directly, use the  `add_script` or `add_style` methods instead."
    )]
    InvalidRenderResult { route: String },
}

#[derive(Error)]
pub enum AssetError {
    #[error("Failed to read asset file: {path}")]
    ReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to get metadata for asset file: {path}")]
    MetadataFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to canonicalize asset path: {path}")]
    CanonicalizeFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Error, Debug)]
pub enum MauditError {
    #[error(transparent)]
    Asset(#[from] AssetError),

    #[error(transparent)]
    Url(#[from] UrlError),

    #[error(transparent)]
    Build(#[from] BuildError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl_debug_for_error!(UrlError, BuildError, AssetError);
