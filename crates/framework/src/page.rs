use crate::assets::PageAssets;
use crate::content::ContentSources;
use crate::errors::UrlError;
use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};
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
    pub params: &'a RouteParams,
    pub content: &'a ContentSources,
    pub assets: &'a mut PageAssets,
    pub current_url: String,
}

impl RouteContext<'_> {
    pub fn params<T>(&self) -> T
    where
        T: From<RouteParams>,
    {
        T::from(self.params.clone())
    }
}

pub struct DynamicRouteContext<'a> {
    pub content: &'a ContentSources,
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

pub trait DynamicRoute {
    // Intentionally does not have a default implementation even though it'd be useful in our macros in order to force
    // the user to implement it explicitly, even if it's just returning an empty Vec.
    fn routes(&self, context: &DynamicRouteContext) -> Vec<RouteParams>;
}

pub enum RouteType {
    Static,
    Dynamic,
}

pub trait InternalPage {
    fn internal_render(&self, ctx: FxHashMap<TypeId, Box<dyn Any>>) -> RenderResult;
    fn route_type(&self) -> RouteType;
    fn route_raw(&self) -> String;
    fn route(&self, params: &RouteParams) -> String;
    fn file_path(&self, params: &RouteParams) -> PathBuf;
    fn url_unsafe<P: Into<RouteParams>>(params: P) -> String
    where
        Self: Sized;
    fn url<P: Into<RouteParams>>(
        &self,
        params: P,
        dynamic_route_context: &DynamicRouteContext,
    ) -> Result<String, UrlError>
    where
        Self: Sized;
    fn url_untyped(&self, params: &RouteParams) -> String;
}

pub trait FullPage: InternalPage + DynamicRoute + Sync {}

pub mod prelude {
    pub use super::{
        DynamicRoute, DynamicRouteContext, FullPage, InternalPage, RenderResult, RouteContext,
        RouteParams,
    };
    pub use crate::assets::Asset;
    pub use maudit_macros::{route, Params};
}
