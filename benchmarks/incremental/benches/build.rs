use std::env;
use std::fs;
use std::path::Path;

use divan::Bencher;
use incremental_benchmark::build_website;

fn main() {
    unsafe {
        env::set_var("MAUDIT_QUIET", "TRUE");
    }
    divan::main();
}

#[divan::bench(sample_count = 10)]
fn incremental_no_changes(bencher: Bencher) {
    // Start fresh: clean dist and cache, then do one full build to populate both.
    let dist_dir = Path::new("dist");
    let cache_dir = Path::new("target/maudit_cache");
    if dist_dir.exists() {
        let _ = fs::remove_dir_all(dist_dir);
    }
    if cache_dir.exists() {
        let _ = fs::remove_dir_all(cache_dir);
    }
    build_website();

    // Bench subsequent builds with no changes — measures cache load, dirty check, and restore.
    bencher.bench(|| {
        build_website();
    });
}
