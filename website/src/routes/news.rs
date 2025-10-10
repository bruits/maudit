use chrono::Datelike;
use maud::PreEscaped;
use maud::html;
use maudit::route::prelude::*;
use std::collections::BTreeMap;

use crate::content::NewsContent;
use crate::layout::SeoMeta;
use crate::layout::layout;

#[route("/news/")]
pub struct NewsIndex;

impl Route for NewsIndex {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let content = ctx.content.get_source::<NewsContent>("news");

        // Group articles by year
        let mut articles_by_year: BTreeMap<String, Vec<_>> = BTreeMap::new();

        for article in &content.entries {
            let year = article.data(ctx).date.year().to_string();
            articles_by_year
                .entry(year)
                .or_insert_with(Vec::new)
                .push(article);
        }

        // Sort articles within each year by date (newest first)
        for articles in articles_by_year.values_mut() {
            articles.sort_by(|a, b| {
                let date_a = &a.data(ctx).date;
                let date_b = &b.data(ctx).date;
                date_b.cmp(date_a) // Reverse order for newest first
            });
        }

        layout(
            html! {
                div.container.mx-auto."px-4"."sm:px-24"."py-10"."pb-24".flex {
                    div.flex-1 {
                        @for (year, articles) in articles_by_year.iter().rev() {
                            h2.text-3xl.font-bold.mb-4#(year) { (year) }
                            ul.space-y-8 {
                                @for article in articles {
                                    li.border-b.border-gray-200.pb-4 {
                                        p.text-sm.font-bold { (article.data(ctx).date) }
                                        h3.text-5xl {
                                            a."hover:text-brand-red" href=(article.id) {
                                                (article.data(ctx).title)
                                            }
                                        }
                                        @if let Some(description) = &article.data(ctx).description {
                                            p.text-lg.text-gray-600.italic { (description) }
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
            Some(SeoMeta {
                title: "News".to_string(),
                description: Some(
                    "Stay updated with the latest news and articles about Maudit.".to_string(),
                ),
                canonical_url: ctx.canonical_url(),
            }),
        )
    }
}

#[route("/news/[slug]/")]
pub struct NewsPage;

#[derive(Params, Clone)]
struct NewsPageParams {
    slug: String,
}

impl Route<NewsPageParams> for NewsPage {
    fn pages(&self, ctx: &mut DynamicRouteContext) -> Pages<NewsPageParams> {
        let content = ctx.content.get_source::<NewsContent>("news");

        content.into_pages(|entry| {
            Page::from_params(NewsPageParams {
                slug: entry.id.clone(),
            })
        })
    }

    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let slug = ctx.params::<NewsPageParams>().slug.clone();
        let entry = ctx
            .content
            .get_source::<NewsContent>("news")
            .get_entry(&slug);

        let NewsContent {
            title,
            description,
            author,
            date,
            ..
        } = entry.data(ctx);

        layout(
            html! {
                div.container.mx-auto."py-10"."pb-24"."max-w-[80ch]"."px-6"."sm:px-0" {
                    section.mb-4.border-b."border-[#e9e9e7]".pb-2 {
                        p.text-sm.font-bold { (date) }
                        h1."text-5xl"."sm:text-6xl".font-bold.mb-3 { (title) }
                        @if let Some(description) = &description {
                            p.text-xl."sm:text-2xl".italic { (description) }
                        }
                    }

                    section.prose.prose-lg.max-w-none {
                        (PreEscaped(entry.render(ctx)))
                    }

                    @if let Some(author) = &author {
                        h2."text-xl".font-bold.mt-12.text-center { (author) }
                    }
                }
            },
            false,
            true,
            ctx,
            Some(SeoMeta {
                title: title.to_string(),
                description: description.clone(),
                canonical_url: ctx.canonical_url(),
            }),
        )
    }
}
