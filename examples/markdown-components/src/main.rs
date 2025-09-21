use maudit::content::{glob_markdown, MarkdownComponents, MarkdownOptions};
use maudit::{content_sources, coronate, routes, BuildOptions, BuildOutput};

mod components;
mod routes;

use components::*;
use routes::{ComponentExample, IndexPage};

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![IndexPage],
        content_sources![
            "examples" => glob_markdown::<ComponentExample>("content/*.md", Some(
                MarkdownOptions::with_components(
                    MarkdownComponents::new()
                        .heading(CustomHeading)
                        .paragraph(CustomParagraph)
                        .link(CustomLink)
                        .image(CustomImage)
                        .strong(CustomStrong)
                        .emphasis(CustomEmphasis)
                        .code(CustomCode)
                        .blockquote(CustomBlockquote)
                        .hard_break(CustomHardBreak)
                        .horizontal_rule(CustomHorizontalRule)
                        .list(CustomList)
                        .list_item(CustomListItem)
                        .strikethrough(CustomStrikethrough)
                        .task_list_marker(CustomTaskListMarker)
                        .table(CustomTable)
                        .table_head(CustomTableHead)
                        .table_row(CustomTableRow)
                        .table_cell(CustomTableCell), Default::default()
                )
            ))
        ],
        BuildOptions::default(),
    )
}
