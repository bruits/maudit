use maud::html;
use maudit::route::prelude::*;

use crate::layout::layout;

#[route("/contribute")]
pub struct Contribute;

impl Route for Contribute {
    fn render(&self, ctx: &mut PageContext) -> RenderResult {
        layout(
            html!(
                    div.container.w-full.max-w-larger-prose.mx-auto.my-14.flex.flex-col."gap-y-12"."px-8"."sm:px-0" {
                        header {
                            h1.text-6xl.text-center.mb-4 { "Thank you!" }
                            main.text-xl.text-center {
                                "Your support helps us improve and maintain the project."
                                br;
                                "Here are some ways you can contribute."
                            }
                        }

                        section.flex.flex-col."gap-y-4" {
                            h2.text-4xl { "Contribute code or documentation" }

                            p.text-lg {
                                "We welcome content contributions of all kinds, from small bug fixes to new documentation pages. If you are interested in contributing directly to the repo, please check out our " a.underline href="https://github.com/bruits/maudit/blob/main/CONTRIBUTING.md" { "contribution guidelines" } " for more information."
                            }
                        }

                        section.flex.flex-col."gap-y-4" {
                            h2.text-4xl { "Report issues" }

                            p.text-lg {
                                "We cannot fix what we do not know is broken. If you encounter any bugs or issues, please report them on our " a.underline href="https://github.com/bruits/maudit/issues" { "GitHub Issues page" } ". Commenting and providing details for existing issues is also very helpful."
                            }
                        }

                        section.flex.flex-col."gap-y-4" {
                            h2.text-4xl { "Help others" }

                            p.text-lg {
                                "If you find Maudit useful, consider helping others by answering questions on " a.underline href="/chat" { "our Discord" } ", writing blog posts, or creating tutorials. Sharing your knowledge helps grow the community."
                            }
                        }

                        section.flex.flex-col."gap-y-4" {
                            h2.text-4xl { "Donate" }

                            p.text-lg {
                                "We'll never ask you directly for money, but if you want to support the project financially, consider sponsoring us on " a.underline href="https://github.com/sponsors/Princesseuh" { "GitHub Sponsors" } ". Your support helps us dedicate more time to maintaining and improving Maudit."
                            }
                        }

                        p.text-lg.text-center {
                            "Everything helps, and we appreciate any support you can provide. From posting a message on your favorite social media to contributing code or documentation, every little bit counts."
                            br; br;
                            span.font-bold { "Thank you for wanting to be part of the Maudit community!" }
                        }
                    }
            ),
            true,
            false,
            ctx,
        )
    }
}
