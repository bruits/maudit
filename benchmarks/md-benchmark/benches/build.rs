use std::env;
use std::fs;
use std::path::Path;

use divan::Bencher;
use md_benchmark::build_website;

fn main() {
    unsafe {
        env::set_var("MAUDIT_QUIET", "TRUE");
    }
    divan::main();
}

#[divan::bench(args = [250, 500, 1000, 2000, 4000], sample_count = 3)]
fn markdown(bencher: Bencher, markdown_count: u32) {
    bencher
        .with_inputs(|| {
            // Clear dist directory before each sample, otherwise later samples will either be very quick if we don't clean
            // or very slow if we do. It's better to measure only the actual work being done. It's also closer to how it'd look like
            // on platforms like Netlify or Vercel where the output directory is always cleaned before each build.
            let dist_dir = Path::new("dist");
            if dist_dir.exists() {
                let _ = fs::remove_dir_all(dist_dir);
            }
            markdown_count
        })
        .bench_values(|markdown_count| {
            build_website(markdown_count);
        });
}
