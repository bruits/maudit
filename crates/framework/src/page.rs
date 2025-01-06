use crate::assets::PageAssets;
use crate::content::Content;
use rustc_hash::FxHashMap;
use std::path::PathBuf;

pub enum RenderResult {
    Text(String),
    Raw(Vec<u8>),
}

impl From<maud::Markup> for RenderResult {
    fn from(val: maud::Markup) -> Self {
        RenderResult::Text(val.into_string())
    }
}

impl From<String> for RenderResult {
    fn from(val: String) -> Self {
        RenderResult::Text(val)
    }
}

impl From<Vec<u8>> for RenderResult {
    fn from(val: Vec<u8>) -> Self {
        RenderResult::Raw(val)
    }
}

pub struct RouteContext<'a> {
    pub raw_params: &'a RouteParams,
    pub content: &'a mut Content<'a>,
    pub assets: &'a mut PageAssets,
    pub current_url: String,
}

impl RouteContext<'_> {
    pub fn params<T>(&self) -> T
    where
        T: From<RouteParams>,
    {
        T::from(self.raw_params.clone())
    }
}

pub struct DynamicRouteContext<'a> {
    pub content: &'a mut Content<'a>,
}

pub trait Page<T = RenderResult>
where
    T: Into<RenderResult>,
{
    fn render(&self, ctx: &mut RouteContext) -> T;
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
}

impl From<&RouteParams> for RouteParams {
    fn from(params: &RouteParams) -> Self {
        params.clone()
    }
}

impl<T> FromIterator<T> for RouteParams
where
    T: Into<RouteParams>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut map = FxHashMap::default();
        for item in iter {
            let item = item.into();
            map.extend(item.0);
        }
        RouteParams(map)
    }
}

pub trait DynamicRoute<P = RouteParams>
where
    P: Into<RouteParams>,
{
    // Intentionally does not have a default implementation even though it'd be useful in our macros in order to force
    // the user to implement it explicitly, even if it's just returning an empty Vec.
    fn routes(&self, context: &mut DynamicRouteContext) -> Vec<P>;
}

pub enum RouteType {
    Static,
    Dynamic,
}

pub trait InternalPage {
    fn route_type(&self) -> RouteType;
    fn route_raw(&self) -> String;
    fn route(&self, params: &RouteParams) -> String;
    fn file_path(&self, params: &RouteParams) -> PathBuf;
    fn url_unsafe<P: Into<RouteParams>>(params: P) -> String
    where
        Self: Sized;
    fn url_untyped(&self, params: &RouteParams) -> String;
}

pub trait FullPage: InternalPage + Sync {
    fn render_internal(&self, ctx: &mut RouteContext) -> RenderResult;
    fn routes_internal(&self, context: &mut DynamicRouteContext) -> Vec<RouteParams>;
}

pub mod prelude {
    pub use super::{
        DynamicRoute, DynamicRouteContext, FullPage, InternalPage, Page, RenderResult,
        RouteContext, RouteParams,
    };
    pub use crate::assets::Asset;
    pub use maudit_macros::{route, Params};
}
