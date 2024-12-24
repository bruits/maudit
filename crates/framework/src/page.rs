use std::path::PathBuf;

use rustc_hash::FxHashMap;

use crate::assets::PageAssets;

pub enum RenderResult<T = maud::Markup> {
    Html(T),
    Text(String),
}

pub struct RouteContext<'a> {
    pub params: RouteParams,
    pub assets: &'a mut PageAssets,
}

pub trait Page {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult;
}

#[derive(Clone, Default, Debug)]
pub struct RouteParams(pub FxHashMap<String, String>);

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
    fn route(&self, params: &RouteParams) -> String;
    fn file_path(&self, params: &RouteParams) -> PathBuf;
}

pub trait FullPage: Page + InternalPage + DynamicPage + Sync {}
