use maud::{Markup, Render, html};

use crate::{
    GENERATOR,
    assets::{Asset, Image, Script, Style},
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
        html! {
            img src=(self.url().unwrap()) width=(self.width) height=(self.height) loading="lazy" decoding="async";
        }
    }
}

/// Can be used to create a generator tag in the output HTML. See [`GENERATOR`](crate::GENERATOR).
pub fn generator() -> Markup {
    html! {
        meta name="generator" content=(GENERATOR);
    }
}
