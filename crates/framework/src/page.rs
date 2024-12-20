use std::path::PathBuf;

pub enum RenderResult<T = maud::Markup> {
    Html(T),
    Text(String),
}

pub trait Page {
    fn render(&self) -> RenderResult;
}

pub trait InternalPage {
    fn route(&self) -> String;
    fn file_path(&self) -> PathBuf;
}

pub trait FullPage: Page + InternalPage {}

pub trait Params {
    fn params(&self) -> Vec<String>;
}
