use maudit::content::{MarkdownComponents, MarkdownOptions, glob_markdown_with_options};
use maudit::{BuildOptions, BuildOutput, content_sources, coronate, routes};

mod components;
mod routes;

use components::*;
use routes::{ComponentExample, IndexPage};

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    coronate(
        routes![IndexPage],
        content_sources![
            "examples" => glob_markdown_with_options::<ComponentExample>("content/*.md",
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
            ))
        ],
        BuildOptions::default(),
    )
}
