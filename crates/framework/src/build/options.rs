/// Maudit build options. Should be passed to [`coronate()`](crate::coronate()).
///
/// ## Examples
/// Default values:
/// ```rust
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
/// ```rust
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
///     },
///   )
/// }
/// ```
pub struct BuildOptions {
    pub output_dir: String,
    pub assets_dir: String,
    pub static_dir: String,
    pub tailwind_binary_path: String,
}

/// Provides default values for [`crate::coronate()`]. Designed to work for most projects.
///
/// ## Examples
/// ```rust
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
            tailwind_binary_path: "./node_modules/.bin/tailwindcss".to_string(),
        }
    }
}
