use maud::{Markup, Render, html};

use crate::{
    GENERATOR,
    assets::{Asset, Script, Style},
    route::RenderResult,
};

impl Render for Style {
    fn render(&self) -> Markup {
        html! {
            link rel="stylesheet" type="text/css" href=(self.url());
        }
    }
}

impl Render for Script {
    fn render(&self) -> Markup {
        html! {
            script src=(self.url()) type="module" {}
        }
    }
}

/// Can be used to create a generator tag in the output HTML. See [`GENERATOR`](crate::GENERATOR).
pub fn generator() -> Markup {
    html! {
        meta name="generator" content=(GENERATOR);
    }
}

impl From<maud::Markup> for RenderResult {
    fn from(val: maud::Markup) -> Self {
        RenderResult::Text(val.into_string())
    }
}
