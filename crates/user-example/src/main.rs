mod pages;
use maudit::routes::Router;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new(vec![Box::new(pages::Index), Box::new(pages::Endpoint)]);

    maudit::coronate(router)
}
