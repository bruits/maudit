use std::env;

use md_benchmark::build_website;

fn main() {
    unsafe {
        env::set_var("MAUDIT_QUIET", "TRUE");
    }
    divan::main();
}

#[divan::bench(args = [250, 500, 1000, 2000, 4000], sample_count = 3)]
fn full_build(markdown_count: u32) {
    build_website(markdown_count);
}
