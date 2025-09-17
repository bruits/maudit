use maud::html;
use maud::PreEscaped;
use maudit::page::prelude::*;
use std::collections::BTreeMap;

use crate::content::NewsContent;
use crate::layout::layout;

#[route("/news")]
pub struct NewsIndex;

impl Page for NewsIndex {
    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let content = ctx.content.get_source::<NewsContent>("news");

        // Group articles by year
        let mut articles_by_year: BTreeMap<String, Vec<_>> = BTreeMap::new();

        for article in &content.entries {
            if let Some(date) = &article.data(ctx).date {
                // Extract year from date (format: 2025-08-16)
                let year = date.split('-').next().unwrap_or("Unknown").to_string();
                articles_by_year
                    .entry(year)
                    .or_insert_with(Vec::new)
                    .push(article);
            } else {
                // articles without dates
                articles_by_year
                    .entry("Unknown".to_string())
                    .or_insert_with(Vec::new)
                    .push(article);
            }
        }

        layout(
            html! {
                div.container.mx-auto."px-24"."py-10"."pb-24".flex {
                    div.flex-1 {
                        @for (year, articles) in articles_by_year.iter().rev() {
                            h2.text-3xl.font-bold.mb-4#(year) { (year) }
                            ul.space-y-8 {
                                @for article in articles {
                                    li.border-b.border-gray-200.pb-4 {
                                        @if let Some(date) = &article.data(ctx).date {
                                            p.text-sm.font-bold { (date) }
                                        }
                                        h3.text-5xl {
                                            a."hover:text-brand-red" href=(article.id) {
                                                (article.data(ctx).title)
                                            }
                                        }
                                        @if let Some(description) = &article.data(ctx).description {
                                            p.text-lg.text-gray-600 { (description) }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // To enable whenever we have mutliple years
                    // div.px-4 {
                    //     @for (year, articles) in articles_by_year.iter().rev() {
                    //         @if !articles.is_empty() {
                    //             a."hover:text-brand-red".text-2xl href=(format!("#{}", year)) {
                    //                 (year)
                    //             }
                    //         }
                    //     }
                    // }
                }
            },
            true,
            true,
            ctx,
        )
    }
}

#[route("/news/[slug]")]
pub struct NewsPage;

#[derive(Params, Clone)]
struct NewsPageParams {
    slug: String,
}

impl Page<NewsPageParams> for NewsPage {
    fn routes(&self, ctx: &DynamicRouteContext) -> Vec<Route<NewsPageParams>> {
        let content = ctx.content.get_source::<NewsContent>("news");

        content.into_routes(|entry| {
            Route::from_params(NewsPageParams {
                slug: entry.id.clone(),
            })
        })
    }

    fn render(&self, ctx: &mut RouteContext) -> RenderResult {
        let slug = ctx.params::<NewsPageParams>().slug.clone();
        let entry = ctx
            .content
            .get_source::<NewsContent>("news")
            .get_entry(&slug);

        layout(
            html! {
                div.container.mx-auto."py-10"."pb-24"."max-w-[80ch]"."px-8"."sm:px-0" {
                    section.mb-4.border-b."border-[#e9e9e7]".pb-2 {
                        @if let Some(date) = &entry.data(ctx).date {
                            p.text-sm.font-bold { (date) }
                        }
                        h1."text-6xl"."sm:text-7xl".font-bold { (entry.data(ctx).title) }
                        @if let Some(description) = &entry.data(ctx).description {
                            p.text-xl."sm:text-2xl" { (description) }
                        }
                    }

                    section.prose."lg:prose-lg".max-w-none {
                        (PreEscaped(entry.render(ctx)))
                    }

                    @if let Some(author) = &entry.data(ctx).author {
                        h2."text-xl".font-bold.mt-12.text-center { (author) }
                    }
                }
            },
            false,
            true,
            ctx,
        )
    }
}
