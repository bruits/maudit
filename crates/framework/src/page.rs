use std::{collections::HashMap, path::PathBuf};

pub enum RenderResult<T = maud::Markup> {
    Html(T),
    Text(String),
}

pub struct RouteContext {
    pub params: HashMap<String, String>,
}

pub trait Page {
    fn render(&self, ctx: &RouteContext) -> RenderResult;
}

pub trait DynamicPage {
    fn routes(&self) -> Vec<HashMap<String, String>>;
}

pub trait InternalPage {
    fn route_raw(&self) -> String;
    fn route(&self, params: HashMap<String, String>) -> String;
    fn file_path(&self, params: HashMap<String, String>) -> PathBuf;
}

pub trait FullPage: Page + InternalPage + DynamicPage + Sync {}
