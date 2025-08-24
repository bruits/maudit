use maudit::content::markdown::components::*;

// Custom heading component that adds icons and anchor links
pub struct CustomHeading;

impl HeadingComponent for CustomHeading {
    fn render_start(&self, level: u8, id: Option<&str>, classes: &[&str]) -> String {
        let id_attr = id.map(|i| format!(" id=\"{}\"", i)).unwrap_or_default();
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
        "<p class=\"prose\">".to_string()
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
        "<strong class=\"gradient-text\">".to_string()
    }

    fn render_end(&self) -> String {
        "</strong>".to_string()
    }
}

// Custom emphasis with italic styling
pub struct CustomEmphasis;

impl EmphasisComponent for CustomEmphasis {
    fn render_start(&self) -> String {
        "<em class=\"emphasis-text\">".to_string()
    }

    fn render_end(&self) -> String {
        "</em>".to_string()
    }
}

// Custom inline code with syntax highlighting
pub struct CustomCode;

impl CodeComponent for CustomCode {
    fn render(&self, code: &str) -> String {
        format!("<code class=\"inline-code\">{}</code>", code)
    }
}

// Custom blockquote with different styles per type
pub struct CustomBlockquote;

impl BlockquoteComponent for CustomBlockquote {
    fn render_start(&self, kind: Option<BlockQuoteKind>) -> String {
        match kind {
            Some(BlockQuoteKind::Note) => "<blockquote class=\"blockquote-note\"><div class=\"blockquote-icon\">ℹ️</div><div class=\"blockquote-content\">".to_string(),
            Some(BlockQuoteKind::Tip) => "<blockquote class=\"blockquote-tip\"><div class=\"blockquote-icon\">💡</div><div class=\"blockquote-content\">".to_string(),
            Some(BlockQuoteKind::Warning) => "<blockquote class=\"blockquote-warning\"><div class=\"blockquote-icon\">⚠️</div><div class=\"blockquote-content\">".to_string(),
            Some(BlockQuoteKind::Important) => "<blockquote class=\"blockquote-important\"><div class=\"blockquote-icon\">❗</div><div class=\"blockquote-content\">".to_string(),
            Some(BlockQuoteKind::Caution) => "<blockquote class=\"blockquote-caution\"><div class=\"blockquote-icon\">🚨</div><div class=\"blockquote-content\">".to_string(),
            None => "<blockquote class=\"blockquote-default blockquote-content\">".to_string(),
        }
    }

    fn render_end(&self, kind: Option<BlockQuoteKind>) -> String {
        if kind.is_some() {
            "</div></blockquote>".to_string()
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
        "<hr class=\"custom-hr\" />".to_string()
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
                format!("<ol class=\"custom-list\" style=\"list-style-type: decimal; list-style-position: inside;\"{}>", start_attr)
            }
            ListType::Unordered => {
                "<ul class=\"custom-list\" style=\"list-style-type: disc; list-style-position: inside;\">".to_string()
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
        "<li>".to_string()
    }

    fn render_end(&self) -> String {
        "</li>".to_string()
    }
}

// Custom strikethrough
pub struct CustomStrikethrough;

impl StrikethroughComponent for CustomStrikethrough {
    fn render_start(&self) -> String {
        "<del class=\"strikethrough\">".to_string()
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
            "<input type=\"checkbox\" checked disabled class=\"task-checkbox\" />".to_string()
        } else {
            "<input type=\"checkbox\" disabled class=\"task-checkbox\" />".to_string()
        }
    }
}

// Custom table
pub struct CustomTable;

impl TableComponent for CustomTable {
    fn render_start(&self, _alignments: &[TableAlignment]) -> String {
        "<table class=\"custom-table\">".to_string()
    }

    fn render_end(&self) -> String {
        "</table>".to_string()
    }
}

// Custom table head
pub struct CustomTableHead;

impl TableHeadComponent for CustomTableHead {
    fn render_start(&self) -> String {
        "<thead class=\"table-header\">".to_string()
    }

    fn render_end(&self) -> String {
        "</thead>".to_string()
    }
}

// Custom table row
pub struct CustomTableRow;

impl TableRowComponent for CustomTableRow {
    fn render_start(&self) -> String {
        "<tr class=\"table-row\">".to_string()
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
        let mut class = "table-cell".to_string();

        match alignment {
            Some(TableAlignment::Center) => class.push_str(" center"),
            Some(TableAlignment::Right) => class.push_str(" right"),
            _ => {}
        };

        format!("<{} class=\"{}\">", tag, class)
    }

    fn render_end(&self, is_header: bool) -> String {
        let tag = if is_header { "th" } else { "td" };
        format!("</{}>", tag)
    }
}
