pub struct BuildOptions {
    pub output_dir: String,
    pub assets_dir: String,
    pub static_dir: String,
    pub tailwind_binary_path: String,
}

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
