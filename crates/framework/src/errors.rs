use std::fmt::{self, Debug, Formatter};
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
    #[error("`{route}` returns `RenderResult::Raw`, but includes styles or scripts, which can only be included in HTML. If you meant to return HTML, use `RenderResult::Html` instead. Alternatively, if you meant to add a reference to a script or style without including it directly, use the  `add_script` or `add_style` methods instead.")]
    InvalidRenderResult { route: String },
}

impl_debug_for_error!(UrlError, BuildError);
