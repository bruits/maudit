use maud::{Markup, PreEscaped, Render, html};

use crate::{
    GENERATOR,
    assets::{Asset, RenderedImage, Script, Style},
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

impl Render for RenderedImage {
    fn render(&self) -> Markup {
        PreEscaped(self.to_string())
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
