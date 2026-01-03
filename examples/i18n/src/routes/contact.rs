use maudit::route::prelude::*;

#[route(
    "/contact",
    locales(en(prefix = "/en"), sv(prefix = "/sv"), de(path = "/de/kontakt"))
)]
pub struct Contact;

impl Route for Contact {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        match &ctx.variant {
            Some(language) => match language.as_str() {
                "en" => "Contact us.",
                "sv" => "Kontakta oss.",
                "de" => "Kontaktieren Sie uns.",
                _ => unreachable!(),
            },
            _ => "Contact us.",
        }
    }
}
