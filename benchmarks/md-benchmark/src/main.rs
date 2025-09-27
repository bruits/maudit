use md_benchmark::build_website;

fn main() {
    let markdown_count = std::env::var("MARKDOWN_COUNT")
        .unwrap_or_else(|_| "1000".to_string())
        .parse::<usize>()
        .unwrap();

    println!("Building with {} markdown files", markdown_count);
    build_website(markdown_count);
}
