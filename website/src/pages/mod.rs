mod index;
pub use index::Index;
mod docs;
pub use docs::{DocsIndex, DocsPage};
mod chat;
pub use chat::ChatRedirect;

mod news;
pub use news::{NewsIndex, NewsPage};

#[path = "404.rs"]
mod not_found;
pub use not_found::NotFound;
