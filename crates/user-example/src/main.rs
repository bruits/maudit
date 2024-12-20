mod pages;
use dire_coronet::routes::Router;

fn main() {
    let router = Router::new(vec![Box::new(pages::Index), Box::new(pages::Endpoint)]);

    dire_coronet::coronate(router);
}
