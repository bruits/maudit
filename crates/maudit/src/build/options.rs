/// Maudit build options. Should be passed to [`coronate()`](crate::coronate()).
///
/// ## Examples
/// Default values:
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
/// Custom values:
/// ```rs
/// use maudit::{
///   content_sources, coronate, routes, BuildOptions, BuildOutput,
/// };
///
/// fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
///   coronate(
///     routes![],
///     content_sources![],
///     BuildOptions {
///       output_dir: "public".to_string(),
///       assets_dir: "_assets".to_string(),
///       static_dir: "static".to_string(),
///       tailwind_binary_path: "./node_modules/.bin/tailwindcss".to_string(),
///       ..Default::default()
///     },
///   )
/// }
/// ```
pub struct BuildOptions {
    pub output_dir: String,
    pub assets_dir: String,
    pub static_dir: String,
    /// Path to [the TailwindCSS CLI binary](https://tailwindcss.com/docs/installation/tailwind-cli). By default `tailwindcss`, which assumes you've installed it globally (for example, through Homebrew) and that it is in your `PATH`.
    ///
    /// This is commonly set to `./node_modules/.bin/tailwindcss` or similar, in order to use a locally installed version.
    pub tailwind_binary_path: String,
    /// Whether to clean the output directory before building.
    ///
    /// At the speed Maudit operates at, not cleaning the output directory may offer a significant performance improvement at the cost of potentially serving stale content.
    pub clean_output_dir: bool,
}

/// Provides default values for [`crate::coronate()`]. Designed to work for most projects.
///
/// ## Examples
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
impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            output_dir: "dist".to_string(),
            assets_dir: "_maudit".to_string(),
            static_dir: "static".to_string(),
            tailwind_binary_path: "tailwindcss".to_string(),
            clean_output_dir: true,
        }
    }
}
