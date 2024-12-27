use crate::assets::PageAssets;
use crate::errors::UrlError;
use rustc_hash::FxHashMap;
use std::path::PathBuf;

pub enum RenderResult {
    Html(String),
    Text(String),
}

impl From<maud::Markup> for RenderResult {
    fn from(val: maud::Markup) -> Self {
        RenderResult::Html(val.into_string())
    }
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
    fn url_unsafe<P: Into<RouteParams>>(params: P) -> String
    where
        Self: Sized;
    fn url<P: Into<RouteParams>>(&self, params: P) -> Result<String, UrlError>
    where
        Self: Sized;
}

pub trait FullPage: Page + InternalPage + DynamicPage + Sync {}

pub mod prelude {
    pub use super::{
        DynamicPage, FullPage, InternalPage, Page, RenderResult, RouteContext, RouteParams,
    };
    pub use crate::assets::Asset;
    pub use maudit_macros::{route, Params};
}
