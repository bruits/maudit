use maud::{html, Markup, Render};

use crate::{
    assets::{Asset, Image, Script, Style},
    GENERATOR,
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
            script src=(self.url().unwrap()) r#type="module" {}
        }
    }
}

impl Render for Image {
    fn render(&self) -> Markup {
        html! {
            img src=(self.url().unwrap()) loading="lazy" decoding="async";
        }
    }
}

pub fn generator() -> Markup {
    html! {
        meta name="generator" content=(GENERATOR);
    }
}
