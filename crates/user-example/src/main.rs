mod pages;
use maudit::Router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new(vec![
        &pages::DynamicExample,
        &pages::Endpoint,
        &pages::Index,
    ]);

    maudit::coronate(router).await
}
