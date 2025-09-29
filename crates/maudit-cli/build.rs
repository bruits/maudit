use rolldown::{Bundler, BundlerOptions, InputItem};
use std::path::PathBuf;

fn main() {
    // Tell Cargo to rerun if any of our JS/TS files change
    println!("cargo:rerun-if-changed=src/dev/js");
    println!("cargo:rerun-if-changed=tsconfig.json");

    // Only bundle during regular builds, not during cargo check or similar
    if std::env::var("CARGO_CFG_TARGET_ARCH").is_err() {
        return;
    }

    let js_src_dir = PathBuf::from("src/dev/js");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR environment variable not set");
    let js_dist_dir = PathBuf::from(out_dir).join("js");

    // Ensure the dist directory exists
    std::fs::create_dir_all(&js_dist_dir).expect("Failed to create dist directory");

    // Configure Rolldown bundler input
    let input_items = vec![InputItem {
        name: Some("client".to_string()),
        import: js_src_dir.join("client.ts").to_string_lossy().to_string(),
    }];

    let bundler_options = BundlerOptions {
        input: Some(input_items),
        dir: Some(js_dist_dir.to_string_lossy().to_string()),
        format: Some(rolldown::OutputFormat::Esm),
        ..Default::default()
    };

    // Create and run the bundler
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    runtime.block_on(async {
        let mut bundler = Bundler::new(bundler_options).unwrap();
        if let Err(e) = bundler.write().await {
            panic!("Failed to bundle JavaScript: {:?}", e);
        }
        println!("Successfully bundled JavaScript files");
    });
}
