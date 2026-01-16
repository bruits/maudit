use rolldown::{Bundler, BundlerOptions, InputItem, RawMinifyOptions};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

type DynError = Box<dyn std::error::Error>;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("build-js") => build_js()?,
        Some("build-cli-js") => build_cli_js()?,
        Some("build-maudit-js") => build_maudit_js()?,
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    println!("Usage: cargo xtask <task>");
    println!();
    println!("Available tasks:");
    println!("  build-js          Bundle JavaScript/TypeScript assets for all crates");
    println!("  build-cli-js      Bundle JavaScript/TypeScript assets for the CLI crate");
    println!("  build-maudit-js   Bundle JavaScript/TypeScript assets for the maudit crate");
}

fn build_js() -> Result<(), DynError> {
    println!("Building JavaScript for all crates...");
    build_cli_js()?;
    build_maudit_js()?;
    println!("All JavaScript builds completed successfully!");
    Ok(())
}

fn build_cli_js() -> Result<(), DynError> {
    let workspace_root = project_root();
    let cli_crate = workspace_root.join("crates/maudit-cli");
    let js_src_dir = cli_crate.join("js");
    let js_dist_dir = js_src_dir.join("dist");

    println!("Building JavaScript for maudit-cli...");

    // Ensure the dist directory exists
    fs::create_dir_all(&js_dist_dir)?;

    // Configure Rolldown bundler input
    let input_items = vec![InputItem {
        name: Some("client".to_string()),
        import: js_src_dir.join("client.ts").to_string_lossy().to_string(),
    }];

    let bundler_options = BundlerOptions {
        input: Some(input_items),
        dir: Some(js_dist_dir.to_string_lossy().to_string()),
        format: Some(rolldown::OutputFormat::Esm),
        platform: Some(rolldown::Platform::Browser),
        minify: Some(RawMinifyOptions::Bool(true)),
        ..Default::default()
    };

    // Create and run the bundler
    let runtime = tokio::runtime::Runtime::new()?;

    runtime.block_on(async {
        let mut bundler = Bundler::new(bundler_options)
            .map_err(|e| format!("Failed to create bundler: {:?}", e))?;

        bundler
            .write()
            .await
            .map_err(|e| format!("Failed to bundle JavaScript: {:?}", e))?;

        println!(
            "Successfully bundled JavaScript files to {}",
            js_dist_dir.display()
        );

        Ok::<(), DynError>(())
    })?;

    Ok(())
}

fn build_maudit_js() -> Result<(), DynError> {
    let workspace_root = project_root();
    let maudit_crate = workspace_root.join("crates/maudit");
    let js_src_dir = maudit_crate.join("js");
    let js_dist_dir = js_src_dir.join("dist");

    println!("Building JavaScript for maudit...");

    // Ensure the dist directory exists
    fs::create_dir_all(&js_dist_dir)?;

    // Configure Rolldown bundler input
    let input_items = vec![
        InputItem {
            name: Some("prefetch".to_string()),
            import: js_src_dir.join("prefetch.ts").to_string_lossy().to_string(),
        },
        InputItem {
            name: Some("hover".to_string()),
            import: js_src_dir
                .join("prefetch")
                .join("hover.ts")
                .to_string_lossy()
                .to_string(),
        },
    ];

    let bundler_options = BundlerOptions {
        input: Some(input_items),
        dir: Some(js_dist_dir.to_string_lossy().to_string()),
        format: Some(rolldown::OutputFormat::Esm),
        platform: Some(rolldown::Platform::Browser),
        minify: Some(RawMinifyOptions::Bool(true)),
        ..Default::default()
    };

    // Create and run the bundler
    let runtime = tokio::runtime::Runtime::new()?;

    runtime.block_on(async {
        let mut bundler = Bundler::new(bundler_options)
            .map_err(|e| format!("Failed to create bundler: {:?}", e))?;

        bundler
            .write()
            .await
            .map_err(|e| format!("Failed to bundle JavaScript: {:?}", e))?;

        println!(
            "Successfully bundled JavaScript files to {}",
            js_dist_dir.display()
        );

        Ok::<(), DynError>(())
    })?;

    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
