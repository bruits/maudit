use maudit::{coronate, generate_pages_mod, routes, BuildOutput};

generate_pages_mod!();

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(routes![Index, DynamicExample, Endpoint, HelloWorld])
}
