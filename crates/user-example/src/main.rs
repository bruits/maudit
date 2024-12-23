mod pages;
use maudit::routes::Router;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new(vec![&pages::Index, &pages::Endpoint]);

    maudit::coronate(router)
}
