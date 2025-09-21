use maud::{Markup, Render, html};

use crate::{
    GENERATOR,
    assets::{Asset, Image, Script, Style},
    route::RenderResult,
};

impl Render for Style {
    fn render(&self) -> Markup {
        html! {
            link rel="stylesheet" type="text/css" href=(self.url().unwrap());
        }
    }
}

impl Render for Script {
    fn render(&self) -> Markup {
        html! {
            script src=(self.url().unwrap()) type="module" {}
        }
    }
}

impl Render for Image {
    fn render(&self) -> Markup {
        let (width, height) = self.dimensions();
        html! {
            img src=(self.url().unwrap()) width=(width) height=(height) loading="lazy" decoding="async";
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
