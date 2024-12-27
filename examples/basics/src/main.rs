mod layout;

use maudit::{coronate, generate_pages_mod, routes, BuildOptions, BuildOutput};

generate_pages_mod!();

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(routes![Index], vec![].into(), BuildOptions::default())
}
