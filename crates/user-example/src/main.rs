use maudit::{coronate, generate_pages_mod, routes};

generate_pages_mod!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    coronate(routes![Index, DynamicExample, Endpoint])
}
