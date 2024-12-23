use std::{collections::HashMap, path::PathBuf};

pub enum RenderResult<T = maud::Markup> {
    Html(T),
    Text(String),
}

pub struct RouteContext {
    pub params: RouteParams,
}

pub trait Page {
    fn render(&self, ctx: &RouteContext) -> RenderResult;
}

#[derive(Clone, Default, Debug)]
pub struct RouteParams(pub HashMap<String, String>);

impl RouteParams {
    pub fn from_vec<T>(params: Vec<T>) -> Vec<RouteParams>
    where
        T: Into<RouteParams>,
    {
        params.into_iter().map(|p| p.into()).collect()
    }

    pub fn parse_into<T>(&self) -> T
    where
        T: From<RouteParams>,
    {
        T::from(self.clone())
    }
}

pub trait DynamicPage {
    fn routes(&self) -> Vec<RouteParams>;
}

pub trait InternalPage {
    fn route_raw(&self) -> String;
    fn route(&self, params: RouteParams) -> String;
    fn file_path(&self, params: RouteParams) -> PathBuf;
}

pub trait FullPage: Page + InternalPage + DynamicPage + Sync {}
