use maudit::content::markdown::components::*;

// Custom heading component that adds icons and anchor links
pub struct CustomHeading;

impl HeadingComponent for CustomHeading {
    fn render_start(&self, level: u8, id: Option<&str>, classes: &[&str]) -> String {
        println!(
            "Rendering heading level {level} with id {:?} and classes {:?}",
            id, classes
        );
        let id_attr = id.map(|i| format!(" id=\"{}\"", i)).unwrap_or_default();
        println!("id_attr: {}", id_attr);
        let class_attr = if classes.is_empty() {
            String::new()
        } else {
            format!(" class=\"{}\"", classes.join(" "))
        };
        let icon = match level {
            1 => "🎯",
            2 => "📌",
            3 => "⭐",
            4 => "🔹",
            5 => "🔸",
            6 => "💎",
            _ => "🔷",
        };
        format!("<h{level}{id_attr}{class_attr}>{icon} ")
    }

    fn render_end(&self, level: u8) -> String {
        if let Some(id) = level.checked_sub(1) {
            format!(" <a href=\"#heading-{id}\" class=\"anchor-link\">🔗</a></h{level}>")
        } else {
            format!("</h{level}>")
        }
    }
}

// Custom paragraph with fancy styling
pub struct CustomParagraph;

impl ParagraphComponent for CustomParagraph {
    fn render_start(&self) -> String {
        "<p class=\"prose prose-lg text-gray-800 leading-relaxed\">".to_string()
    }

    fn render_end(&self) -> String {
        "</p>".to_string()
    }
}

// Custom link with external link detection
pub struct CustomLink;

impl LinkComponent for CustomLink {
    fn render_start(&self, url: &str, title: Option<&str>, _link_type: LinkType) -> String {
        let title_attr = title
            .map(|t| format!(" title=\"{}\"", t))
            .unwrap_or_default();

        let class = if url.starts_with("http") {
            "external-link"
        } else {
            "internal-link"
        };

        format!("<a href=\"{}\" class=\"{}\"{}>", url, class, title_attr)
    }

    fn render_end(&self) -> String {
        "</a>".to_string()
    }
}

// Custom image with figure wrapper
pub struct CustomImage;

impl ImageComponent for CustomImage {
    fn render(&self, url: &str, alt: &str, title: Option<&str>) -> String {
        let title_attr = title
            .map(|t| format!(" title=\"{}\"", t))
            .unwrap_or_default();

        format!(
            "<figure class=\"image-wrapper\"><img src=\"{}\" alt=\"{}\" class=\"responsive-image\"{} /><figcaption>{}</figcaption></figure>",
            url, alt, title_attr, alt
        )
    }
}

// Custom strong with gradient text
pub struct CustomStrong;

impl StrongComponent for CustomStrong {
    fn render_start(&self) -> String {
        "<strong class=\"font-bold bg-gradient-to-r from-purple-600 to-blue-600 bg-clip-text text-transparent\">".to_string()
    }

    fn render_end(&self) -> String {
        "</strong>".to_string()
    }
}

// Custom emphasis with italic styling
pub struct CustomEmphasis;

impl EmphasisComponent for CustomEmphasis {
    fn render_start(&self) -> String {
        "<em class=\"italic text-indigo-600\">".to_string()
    }

    fn render_end(&self) -> String {
        "</em>".to_string()
    }
}

// Custom inline code with syntax highlighting
pub struct CustomCode;

impl CodeComponent for CustomCode {
    fn render(&self, code: &str) -> String {
        format!("<code class=\"bg-gray-100 text-red-600 px-1 py-0.5 rounded font-mono text-sm\">{}</code>", code)
    }
}

// Custom blockquote with different styles per type
pub struct CustomBlockquote;

impl BlockquoteComponent for CustomBlockquote {
    fn render_start(&self, kind: Option<BlockQuoteKind>) -> String {
        match kind {
            Some(BlockQuoteKind::Note) => "<blockquote class=\"border-l-4 border-blue-500 bg-blue-50 p-4 my-4\"><div class=\"flex\"><span class=\"text-blue-500 mr-2\">ℹ️</span><div>".to_string(),
            Some(BlockQuoteKind::Tip) => "<blockquote class=\"border-l-4 border-green-500 bg-green-50 p-4 my-4\"><div class=\"flex\"><span class=\"text-green-500 mr-2\">💡</span><div>".to_string(),
            Some(BlockQuoteKind::Warning) => "<blockquote class=\"border-l-4 border-yellow-500 bg-yellow-50 p-4 my-4\"><div class=\"flex\"><span class=\"text-yellow-500 mr-2\">⚠️</span><div>".to_string(),
            Some(BlockQuoteKind::Important) => "<blockquote class=\"border-l-4 border-purple-500 bg-purple-50 p-4 my-4\"><div class=\"flex\"><span class=\"text-purple-500 mr-2\">❗</span><div>".to_string(),
            Some(BlockQuoteKind::Caution) => "<blockquote class=\"border-l-4 border-red-500 bg-red-50 p-4 my-4\"><div class=\"flex\"><span class=\"text-red-500 mr-2\">🚨</span><div>".to_string(),
            None => "<blockquote class=\"border-l-4 border-gray-400 bg-gray-50 p-4 my-4 italic\">".to_string(),
        }
    }

    fn render_end(&self, kind: Option<BlockQuoteKind>) -> String {
        if kind.is_some() {
            "</div></div></blockquote>".to_string()
        } else {
            "</blockquote>".to_string()
        }
    }
}

// Custom hard break
pub struct CustomHardBreak;

impl HardBreakComponent for CustomHardBreak {
    fn render(&self) -> String {
        "<br class=\"hard-break\" />".to_string()
    }
}

// Custom horizontal rule
pub struct CustomHorizontalRule;

impl HorizontalRuleComponent for CustomHorizontalRule {
    fn render(&self) -> String {
        "<hr class=\"my-8 border-t-2 border-gradient-to-r from-purple-400 to-pink-400\" />"
            .to_string()
    }
}

// Custom list with different styling
pub struct CustomList;

impl ListComponent for CustomList {
    fn render_start(&self, list_type: ListType, start_number: Option<u64>) -> String {
        match list_type {
            ListType::Ordered => {
                let start_attr = start_number
                    .map(|n| format!(" start=\"{}\"", n))
                    .unwrap_or_default();
                format!(
                    "<ol class=\"list-decimal list-inside space-y-2 ml-4\"{}>",
                    start_attr
                )
            }
            ListType::Unordered => {
                "<ul class=\"list-disc list-inside space-y-2 ml-4\">".to_string()
            }
        }
    }

    fn render_end(&self, list_type: ListType) -> String {
        match list_type {
            ListType::Ordered => "</ol>".to_string(),
            ListType::Unordered => "</ul>".to_string(),
        }
    }
}

// Custom list item
pub struct CustomListItem;

impl ListItemComponent for CustomListItem {
    fn render_start(&self) -> String {
        "<li class=\"text-gray-700 hover:text-gray-900 transition-colors\">".to_string()
    }

    fn render_end(&self) -> String {
        "</li>".to_string()
    }
}

// Custom strikethrough
pub struct CustomStrikethrough;

impl StrikethroughComponent for CustomStrikethrough {
    fn render_start(&self) -> String {
        "<del class=\"line-through text-gray-500 opacity-75\">".to_string()
    }

    fn render_end(&self) -> String {
        "</del>".to_string()
    }
}

// Custom task list marker
pub struct CustomTaskListMarker;

impl TaskListMarkerComponent for CustomTaskListMarker {
    fn render(&self, checked: bool) -> String {
        if checked {
            "<input type=\"checkbox\" checked disabled class=\"mr-2 accent-green-500\" />"
                .to_string()
        } else {
            "<input type=\"checkbox\" disabled class=\"mr-2\" />".to_string()
        }
    }
}

// Custom table
pub struct CustomTable;

impl TableComponent for CustomTable {
    fn render_start(&self, _alignments: &[TableAlignment]) -> String {
        "<table class=\"min-w-full divide-y divide-gray-200 border border-gray-300 rounded-lg overflow-hidden\">".to_string()
    }

    fn render_end(&self) -> String {
        "</table>".to_string()
    }
}

// Custom table head
pub struct CustomTableHead;

impl TableHeadComponent for CustomTableHead {
    fn render_start(&self) -> String {
        "<thead class=\"bg-gray-50\">".to_string()
    }

    fn render_end(&self) -> String {
        "</thead>".to_string()
    }
}

// Custom table row
pub struct CustomTableRow;

impl TableRowComponent for CustomTableRow {
    fn render_start(&self) -> String {
        "<tr class=\"hover:bg-gray-50 transition-colors\">".to_string()
    }

    fn render_end(&self) -> String {
        "</tr>".to_string()
    }
}

// Custom table cell
pub struct CustomTableCell;

impl TableCellComponent for CustomTableCell {
    fn render_start(&self, is_header: bool, alignment: Option<TableAlignment>) -> String {
        let tag = if is_header { "th" } else { "td" };
        let base_class = if is_header {
            "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
        } else {
            "px-6 py-4 whitespace-nowrap text-sm text-gray-900"
        };

        let align_class = match alignment {
            Some(TableAlignment::Center) => " text-center",
            Some(TableAlignment::Right) => " text-right",
            _ => "",
        };

        format!("<{} class=\"{}{}\">", tag, base_class, align_class)
    }

    fn render_end(&self, is_header: bool) -> String {
        let tag = if is_header { "th" } else { "td" };
        format!("</{}>", tag)
    }
}
