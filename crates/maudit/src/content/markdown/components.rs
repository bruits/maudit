#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockQuoteKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl From<pulldown_cmark::BlockQuoteKind> for BlockQuoteKind {
    fn from(kind: pulldown_cmark::BlockQuoteKind) -> Self {
        match kind {
            pulldown_cmark::BlockQuoteKind::Note => BlockQuoteKind::Note,
            pulldown_cmark::BlockQuoteKind::Tip => BlockQuoteKind::Tip,
            pulldown_cmark::BlockQuoteKind::Important => BlockQuoteKind::Important,
            pulldown_cmark::BlockQuoteKind::Warning => BlockQuoteKind::Warning,
            pulldown_cmark::BlockQuoteKind::Caution => BlockQuoteKind::Caution,
        }
    }
}

impl From<&pulldown_cmark::BlockQuoteKind> for BlockQuoteKind {
    fn from(kind: &pulldown_cmark::BlockQuoteKind) -> Self {
        (*kind).into()
    }
}

impl BlockQuoteKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockQuoteKind::Note => "note",
            BlockQuoteKind::Tip => "tip",
            BlockQuoteKind::Important => "important",
            BlockQuoteKind::Warning => "warning",
            BlockQuoteKind::Caution => "caution",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    Inline,
    Reference,
    ReferenceUnknown,
    Collapsed,
    CollapsedUnknown,
    Shortcut,
    ShortcutUnknown,
    Autolink,
    Email,
}

impl From<pulldown_cmark::LinkType> for LinkType {
    fn from(link_type: pulldown_cmark::LinkType) -> Self {
        match link_type {
            pulldown_cmark::LinkType::Inline => LinkType::Inline,
            pulldown_cmark::LinkType::Reference => LinkType::Reference,
            pulldown_cmark::LinkType::ReferenceUnknown => LinkType::ReferenceUnknown,
            pulldown_cmark::LinkType::Collapsed => LinkType::Collapsed,
            pulldown_cmark::LinkType::CollapsedUnknown => LinkType::CollapsedUnknown,
            pulldown_cmark::LinkType::Shortcut => LinkType::Shortcut,
            pulldown_cmark::LinkType::ShortcutUnknown => LinkType::ShortcutUnknown,
            pulldown_cmark::LinkType::Autolink => LinkType::Autolink,
            pulldown_cmark::LinkType::Email => LinkType::Email,
        }
    }
}

impl From<&pulldown_cmark::LinkType> for LinkType {
    fn from(link_type: &pulldown_cmark::LinkType) -> Self {
        (*link_type).into()
    }
}

impl LinkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkType::Inline => "inline",
            LinkType::Reference => "reference",
            LinkType::ReferenceUnknown => "reference_unknown",
            LinkType::Collapsed => "collapsed",
            LinkType::CollapsedUnknown => "collapsed_unknown",
            LinkType::Shortcut => "shortcut",
            LinkType::ShortcutUnknown => "shortcut_unknown",
            LinkType::Autolink => "autolink",
            LinkType::Email => "email",
        }
    }
}

pub trait HeadingComponent {
    fn render_start(&self, level: u8, id: Option<&str>, classes: &[&str]) -> String {
        let class_attr = if !classes.is_empty() {
            format!(" class=\"{}\"", classes.join(" "))
        } else {
            String::new()
        };

        let id_attr = id
            .as_ref()
            .map(|i| format!(" id=\"{}\"", i))
            .unwrap_or_default();

        format!("<h{}{}{}>", level, id_attr, class_attr)
    }

    fn render_end(&self, level: u8) -> String {
        format!("</h{}>", level)
    }
}

pub trait ParagraphComponent {
    fn render_start(&self) -> String {
        "<p>".to_string()
    }

    fn render_end(&self) -> String {
        "</p>".to_string()
    }
}

pub trait LinkComponent {
    fn render_start(&self, url: &str, title: Option<&str>, link_type: LinkType) -> String;

    fn render_end(&self) -> String {
        "</a>".to_string()
    }
}

pub trait ImageComponent {
    fn render(&self, url: &str, alt: &str, title: Option<&str>) -> String;
}

pub trait StrongComponent {
    fn render_start(&self) -> String {
        "<strong>".to_string()
    }

    fn render_end(&self) -> String {
        "</strong>".to_string()
    }
}

pub trait EmphasisComponent {
    fn render_start(&self) -> String {
        "<em>".to_string()
    }

    fn render_end(&self) -> String {
        "</em>".to_string()
    }
}

pub trait CodeComponent {
    fn render(&self, code: &str) -> String;
}

pub trait BlockquoteComponent {
    fn render_start(&self, kind: Option<BlockQuoteKind>) -> String {
        match kind {
            Some(k) => format!("<blockquote data-kind=\"{}\">", k.as_str()),
            None => "<blockquote>".to_string(),
        }
    }

    fn render_end(&self, _kind: Option<BlockQuoteKind>) -> String {
        "</blockquote>".to_string()
    }
}

pub trait HardBreakComponent {
    fn render(&self) -> String {
        "<br />".to_string()
    }
}

pub trait HorizontalRuleComponent {
    fn render(&self) -> String {
        "<hr />".to_string()
    }
}

pub trait ListComponent {
    fn render_start(&self, list_type: ListType, start_number: Option<u64>) -> String {
        match list_type {
            ListType::Ordered => {
                if let Some(start) = start_number {
                    if start != 1 {
                        format!("<ol start=\"{}\">", start)
                    } else {
                        "<ol>".to_string()
                    }
                } else {
                    "<ol>".to_string()
                }
            }
            ListType::Unordered => "<ul>".to_string(),
        }
    }

    fn render_end(&self, list_type: ListType) -> String {
        match list_type {
            ListType::Ordered => "</ol>".to_string(),
            ListType::Unordered => "</ul>".to_string(),
        }
    }
}

pub trait ListItemComponent {
    fn render_start(&self) -> String {
        "<li>".to_string()
    }

    fn render_end(&self) -> String {
        "</li>".to_string()
    }
}

pub trait StrikethroughComponent {
    fn render_start(&self) -> String {
        "<del>".to_string()
    }

    fn render_end(&self) -> String {
        "</del>".to_string()
    }
}

pub trait TaskListMarkerComponent {
    fn render(&self, checked: bool) -> String {
        if checked {
            "<input type=\"checkbox\" checked disabled />".to_string()
        } else {
            "<input type=\"checkbox\" disabled />".to_string()
        }
    }
}

pub trait TableComponent {
    fn render_start(&self, _column_alignments: &[TableAlignment]) -> String {
        "<table>".to_string()
    }

    fn render_end(&self) -> String {
        "</table>".to_string()
    }
}

pub trait TableHeadComponent {
    fn render_start(&self) -> String {
        "<thead>".to_string()
    }

    fn render_end(&self) -> String {
        "</thead>".to_string()
    }
}

pub trait TableRowComponent {
    fn render_start(&self) -> String {
        "<tr>".to_string()
    }

    fn render_end(&self) -> String {
        "</tr>".to_string()
    }
}

pub trait TableCellComponent {
    fn render_start(&self, is_header: bool, alignment: Option<TableAlignment>) -> String {
        let tag = if is_header { "th" } else { "td" };
        match alignment {
            Some(TableAlignment::Left) => format!("<{} style=\"text-align: left\">", tag),
            Some(TableAlignment::Center) => format!("<{} style=\"text-align: center\">", tag),
            Some(TableAlignment::Right) => format!("<{} style=\"text-align: right\">", tag),
            None => format!("<{}>", tag),
        }
    }

    fn render_end(&self, is_header: bool) -> String {
        if is_header {
            "</th>".to_string()
        } else {
            "</td>".to_string()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListType {
    Ordered,
    Unordered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableAlignment {
    Left,
    Center,
    Right,
}

#[derive(Default)]
pub struct MarkdownComponents {
    pub heading: Option<Box<dyn HeadingComponent + Send + Sync>>,
    pub paragraph: Option<Box<dyn ParagraphComponent + Send + Sync>>,
    pub link: Option<Box<dyn LinkComponent + Send + Sync>>,
    pub image: Option<Box<dyn ImageComponent + Send + Sync>>,
    pub strong: Option<Box<dyn StrongComponent + Send + Sync>>,
    pub emphasis: Option<Box<dyn EmphasisComponent + Send + Sync>>,
    pub code: Option<Box<dyn CodeComponent + Send + Sync>>,
    pub blockquote: Option<Box<dyn BlockquoteComponent + Send + Sync>>,
    pub hard_break: Option<Box<dyn HardBreakComponent + Send + Sync>>,
    pub horizontal_rule: Option<Box<dyn HorizontalRuleComponent + Send + Sync>>,
    pub list: Option<Box<dyn ListComponent + Send + Sync>>,
    pub list_item: Option<Box<dyn ListItemComponent + Send + Sync>>,
    pub strikethrough: Option<Box<dyn StrikethroughComponent + Send + Sync>>,
    pub task_list_marker: Option<Box<dyn TaskListMarkerComponent + Send + Sync>>,
    pub table: Option<Box<dyn TableComponent + Send + Sync>>,
    pub table_head: Option<Box<dyn TableHeadComponent + Send + Sync>>,
    pub table_row: Option<Box<dyn TableRowComponent + Send + Sync>>,
    pub table_cell: Option<Box<dyn TableCellComponent + Send + Sync>>,
}

impl MarkdownComponents {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_any_components(&self) -> bool {
        self.heading.is_some()
            || self.paragraph.is_some()
            || self.link.is_some()
            || self.image.is_some()
            || self.strong.is_some()
            || self.emphasis.is_some()
            || self.code.is_some()
            || self.blockquote.is_some()
            || self.hard_break.is_some()
            || self.horizontal_rule.is_some()
            || self.list.is_some()
            || self.list_item.is_some()
            || self.strikethrough.is_some()
            || self.task_list_marker.is_some()
            || self.table.is_some()
            || self.table_head.is_some()
            || self.table_row.is_some()
            || self.table_cell.is_some()
    }

    /// Set a custom heading component
    pub fn heading<C: HeadingComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.heading = Some(Box::new(component));
        self
    }

    /// Set a custom paragraph component
    pub fn paragraph<C: ParagraphComponent + Send + Sync + 'static>(
        mut self,
        component: C,
    ) -> Self {
        self.paragraph = Some(Box::new(component));
        self
    }

    /// Set a custom link component
    pub fn link<C: LinkComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.link = Some(Box::new(component));
        self
    }

    /// Set a custom image component
    pub fn image<C: ImageComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.image = Some(Box::new(component));
        self
    }

    /// Set a custom strong component
    pub fn strong<C: StrongComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.strong = Some(Box::new(component));
        self
    }

    /// Set a custom emphasis component
    pub fn emphasis<C: EmphasisComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.emphasis = Some(Box::new(component));
        self
    }

    /// Set a custom code component
    pub fn code<C: CodeComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.code = Some(Box::new(component));
        self
    }

    /// Set a custom blockquote component
    pub fn blockquote<C: BlockquoteComponent + Send + Sync + 'static>(
        mut self,
        component: C,
    ) -> Self {
        self.blockquote = Some(Box::new(component));
        self
    }

    /// Set a custom hard break component
    pub fn hard_break<C: HardBreakComponent + Send + Sync + 'static>(
        mut self,
        component: C,
    ) -> Self {
        self.hard_break = Some(Box::new(component));
        self
    }

    /// Set a custom horizontal rule component
    pub fn horizontal_rule<C: HorizontalRuleComponent + Send + Sync + 'static>(
        mut self,
        component: C,
    ) -> Self {
        self.horizontal_rule = Some(Box::new(component));
        self
    }

    /// Set a custom list component
    pub fn list<C: ListComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.list = Some(Box::new(component));
        self
    }

    /// Set a custom list item component
    pub fn list_item<C: ListItemComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.list_item = Some(Box::new(component));
        self
    }

    /// Set a custom strikethrough component
    pub fn strikethrough<C: StrikethroughComponent + Send + Sync + 'static>(
        mut self,
        component: C,
    ) -> Self {
        self.strikethrough = Some(Box::new(component));
        self
    }

    /// Set a custom task list marker component
    pub fn task_list_marker<C: TaskListMarkerComponent + Send + Sync + 'static>(
        mut self,
        component: C,
    ) -> Self {
        self.task_list_marker = Some(Box::new(component));
        self
    }

    /// Set a custom table component
    pub fn table<C: TableComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.table = Some(Box::new(component));
        self
    }

    /// Set a custom table head component
    pub fn table_head<C: TableHeadComponent + Send + Sync + 'static>(
        mut self,
        component: C,
    ) -> Self {
        self.table_head = Some(Box::new(component));
        self
    }

    /// Set a custom table row component
    pub fn table_row<C: TableRowComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.table_row = Some(Box::new(component));
        self
    }

    /// Set a custom table cell component
    pub fn table_cell<C: TableCellComponent + Send + Sync + 'static>(
        mut self,
        component: C,
    ) -> Self {
        self.table_cell = Some(Box::new(component));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{render_markdown, MarkdownOptions};

    struct TestCustomHeading;

    impl HeadingComponent for TestCustomHeading {
        fn render_start(&self, level: u8, id: Option<&str>, classes: &[&str]) -> String {
            let id_attr = id.map(|i| format!(" id=\"{}\"", i)).unwrap_or_default();
            let class_attr = if classes.is_empty() {
                String::new()
            } else {
                format!(" class=\"{}\"", classes.join(" "))
            };
            format!("<h{}{}{}>🎯", level, id_attr, class_attr)
        }

        fn render_end(&self, level: u8) -> String {
            format!("</h{}>", level)
        }
    }

    struct TestCustomParagraph;

    impl ParagraphComponent for TestCustomParagraph {
        fn render_start(&self) -> String {
            "<p class=\"custom-paragraph\">".to_string()
        }

        fn render_end(&self) -> String {
            "</p><!-- end custom paragraph -->".to_string()
        }
    }

    struct TestCustomLink;

    impl LinkComponent for TestCustomLink {
        fn render_start(&self, url: &str, title: Option<&str>, _link_type: LinkType) -> String {
            let title_attr = title
                .map(|t| format!(" title=\"{}\"", t))
                .unwrap_or_default();
            format!("<a href=\"{}\" class=\"custom-link\"{}>🔗", url, title_attr)
        }

        fn render_end(&self) -> String {
            "</a>".to_string()
        }
    }

    struct TestCustomImage;

    impl ImageComponent for TestCustomImage {
        fn render(&self, url: &str, alt: &str, title: Option<&str>) -> String {
            let title_attr = title
                .map(|t| format!(" title=\"{}\"", t))
                .unwrap_or_default();
            format!(
                "<img src=\"{}\" alt=\"{}\" class=\"custom-image\"{} />📸",
                url, alt, title_attr
            )
        }
    }

    struct TestCustomStrong;

    impl StrongComponent for TestCustomStrong {
        fn render_start(&self) -> String {
            "<strong class=\"custom-strong\">💪".to_string()
        }

        fn render_end(&self) -> String {
            "</strong>".to_string()
        }
    }

    struct TestCustomEmphasis;

    impl EmphasisComponent for TestCustomEmphasis {
        fn render_start(&self) -> String {
            "<em class=\"custom-emphasis\">✨".to_string()
        }

        fn render_end(&self) -> String {
            "</em>".to_string()
        }
    }

    struct TestCustomCode;

    impl CodeComponent for TestCustomCode {
        fn render(&self, code: &str) -> String {
            format!("<code class=\"custom-code\">💻{}</code>", code)
        }
    }

    struct TestCustomBlockquote;

    impl BlockquoteComponent for TestCustomBlockquote {
        fn render_start(&self, kind: Option<BlockQuoteKind>) -> String {
            match kind {
                Some(k) => format!(
                    "<blockquote class=\"custom-blockquote {}\" data-kind=\"{}\">📝",
                    k.as_str(),
                    k.as_str()
                ),
                None => "<blockquote class=\"custom-blockquote\">📝".to_string(),
            }
        }

        fn render_end(&self, _kind: Option<BlockQuoteKind>) -> String {
            "</blockquote>".to_string()
        }
    }

    #[test]
    fn test_components_builder_pattern() {
        let components = MarkdownComponents::new().heading(TestCustomHeading);

        assert!(components.heading.is_some());
        assert!(components.paragraph.is_none());
        assert!(components.link.is_none());
    }

    #[test]
    fn test_has_any_components() {
        let empty_components = MarkdownComponents::new();
        assert!(!empty_components.has_any_components());

        let with_heading = MarkdownComponents::new().heading(TestCustomHeading);
        assert!(with_heading.has_any_components());
    }

    #[test]
    fn test_custom_heading_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().heading(TestCustomHeading),
        };

        let html = render_markdown("# Hello, world!", Some(&options), None, None);
        assert!(html.contains("🎯Hello, world!"));
    }

    #[test]
    fn test_custom_paragraph_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().paragraph(TestCustomParagraph),
        };

        let content = render_markdown("This is a paragraph.", Some(&options), None, None);
        assert!(content.contains(
            "<p class=\"custom-paragraph\">This is a paragraph.</p><!-- end custom paragraph -->"
        ));
    }

    #[test]
    fn test_custom_link_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().link(TestCustomLink),
        };

        let content = render_markdown("[Example](https://example.com)", Some(&options), None, None);
        assert!(
            content.contains("<a href=\"https://example.com\" class=\"custom-link\">🔗Example</a>")
        );
    }

    #[test]
    fn test_custom_image_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().image(TestCustomImage),
        };

        let content = render_markdown("![Alt text](image.jpg)", Some(&options), None, None);
        assert!(
            content.contains("<img src=\"image.jpg\" alt=\"Alt text\" class=\"custom-image\" />📸")
        );
    }

    #[test]
    fn test_custom_strong_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().strong(TestCustomStrong),
        };

        let content = render_markdown("**Bold text**", Some(&options), None, None);
        assert!(content.contains("<strong class=\"custom-strong\">💪Bold text</strong>"));
    }

    #[test]
    fn test_custom_emphasis_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().emphasis(TestCustomEmphasis),
        };

        let content = render_markdown("*Italic text*", Some(&options), None, None);
        assert!(content.contains("<em class=\"custom-emphasis\">✨Italic text</em>"));
    }

    #[test]
    fn test_custom_code_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().code(TestCustomCode),
        };

        let content = render_markdown("`console.log('hello')`", Some(&options), None, None);
        assert!(content.contains("<code class=\"custom-code\">💻console.log('hello')</code>"));
    }

    #[test]
    fn test_custom_blockquote_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().blockquote(TestCustomBlockquote),
        };

        let content = render_markdown("> This is a quote", Some(&options), None, None);
        assert!(content.contains("<blockquote class=\"custom-blockquote\">📝"));
        assert!(content.contains("</blockquote>"));
        assert!(content.contains("This is a quote"));
    }

    #[test]
    fn test_multiple_custom_components() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new()
                .heading(TestCustomHeading)
                .paragraph(TestCustomParagraph)
                .link(TestCustomLink)
                .strong(TestCustomStrong),
        };

        let content = render_markdown(
            "# Title\n\nThis is a **bold** [link](https://example.com).",
            Some(&options),
            None,
            None,
        );

        assert!(content.contains("🎯Title"));
        assert!(content.contains("<p class=\"custom-paragraph\">"));
        assert!(content.contains("<strong class=\"custom-strong\">💪bold</strong>"));
        assert!(
            content.contains("<a href=\"https://example.com\" class=\"custom-link\">🔗link</a>")
        );
    }

    #[test]
    fn test_nested_components() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new()
                .blockquote(TestCustomBlockquote)
                .strong(TestCustomStrong)
                .emphasis(TestCustomEmphasis)
                .code(TestCustomCode),
        };

        let content = render_markdown(
            "> This is a **bold** and *italic* with `code`",
            Some(&options),
            None,
            None,
        );
        assert!(content.contains("<blockquote class=\"custom-blockquote\">📝"));
        assert!(content.contains("<strong class=\"custom-strong\">💪bold</strong>"));
        assert!(content.contains("<em class=\"custom-emphasis\">✨italic</em>"));
        assert!(content.contains("<code class=\"custom-code\">💻code</code>"));
        assert!(content.contains("</blockquote>"));
    }

    struct TestHardBreak;
    impl HardBreakComponent for TestHardBreak {
        fn render(&self) -> String {
            "<br class=\"custom-break\" />".to_string()
        }
    }

    struct TestHorizontalRule;
    impl HorizontalRuleComponent for TestHorizontalRule {
        fn render(&self) -> String {
            "<hr class=\"custom-rule\" />".to_string()
        }
    }

    struct TestList;
    impl ListComponent for TestList {
        fn render_start(&self, list_type: ListType, start_number: Option<u64>) -> String {
            match list_type {
                ListType::Ordered => format!(
                    "<ol class=\"custom-list\" start=\"{}\">",
                    start_number.unwrap_or(1)
                ),
                ListType::Unordered => "<ul class=\"custom-list\">".to_string(),
            }
        }
        fn render_end(&self, list_type: ListType) -> String {
            match list_type {
                ListType::Ordered => "</ol>".to_string(),
                ListType::Unordered => "</ul>".to_string(),
            }
        }
    }

    struct TestListItem;
    impl ListItemComponent for TestListItem {
        fn render_start(&self) -> String {
            "<li class=\"custom-item\">".to_string()
        }
        fn render_end(&self) -> String {
            "</li>".to_string()
        }
    }

    struct TestStrikethrough;
    impl StrikethroughComponent for TestStrikethrough {
        fn render_start(&self) -> String {
            "<del class=\"custom-strike\">".to_string()
        }
        fn render_end(&self) -> String {
            "</del>".to_string()
        }
    }

    struct TestTaskListMarker;
    impl TaskListMarkerComponent for TestTaskListMarker {
        fn render(&self, checked: bool) -> String {
            if checked {
                "<input type=\"checkbox\" checked class=\"custom-task\" />"
            } else {
                "<input type=\"checkbox\" class=\"custom-task\" />"
            }
            .to_string()
        }
    }

    struct TestTable;
    impl TableComponent for TestTable {
        fn render_start(&self, _alignments: &[TableAlignment]) -> String {
            "<table class=\"custom-table\">".to_string()
        }
        fn render_end(&self) -> String {
            "</table>".to_string()
        }
    }

    struct TestTableHead;
    impl TableHeadComponent for TestTableHead {
        fn render_start(&self) -> String {
            "<thead class=\"custom-thead\">".to_string()
        }
        fn render_end(&self) -> String {
            "</thead>".to_string()
        }
    }

    struct TestTableRow;
    impl TableRowComponent for TestTableRow {
        fn render_start(&self) -> String {
            "<tr class=\"custom-row\">".to_string()
        }
        fn render_end(&self) -> String {
            "</tr>".to_string()
        }
    }

    struct TestTableCell;
    impl TableCellComponent for TestTableCell {
        fn render_start(&self, is_header: bool, alignment: Option<TableAlignment>) -> String {
            let tag = if is_header { "th" } else { "td" };
            let align = match alignment {
                Some(TableAlignment::Left) => " style=\"text-align: left\"",
                Some(TableAlignment::Center) => " style=\"text-align: center\"",
                Some(TableAlignment::Right) => " style=\"text-align: right\"",
                None => "",
            };
            format!("<{} class=\"custom-cell\"{}>", tag, align)
        }
        fn render_end(&self, is_header: bool) -> String {
            if is_header {
                "</th>".to_string()
            } else {
                "</td>".to_string()
            }
        }
    }

    #[test]
    fn test_hard_break_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().hard_break(TestHardBreak),
        };
        let content = render_markdown("Line 1  \nLine 2", Some(&options), None, None);
        assert!(content.contains("<br class=\"custom-break\" />"));
    }

    #[test]
    fn test_horizontal_rule_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().horizontal_rule(TestHorizontalRule),
        };
        let content = render_markdown("---", Some(&options), None, None);
        assert!(content.contains("<hr class=\"custom-rule\" />"));
    }

    #[test]
    fn test_list_components() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new()
                .list(TestList)
                .list_item(TestListItem),
        };
        let content = render_markdown(
            "1. First\n2. Second\n\n- Bullet\n- Point",
            Some(&options),
            None,
            None,
        );
        assert!(content.contains("<ol class=\"custom-list\" start=\"1\">"));
        assert!(content.contains("<ul class=\"custom-list\">"));
        assert!(content.contains("<li class=\"custom-item\">"));
    }

    #[test]
    fn test_strikethrough_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().strikethrough(TestStrikethrough),
        };
        let content = render_markdown("~~strikethrough~~", Some(&options), None, None);
        assert!(content.contains("<del class=\"custom-strike\">"));
    }

    #[test]
    fn test_task_list_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().task_list_marker(TestTaskListMarker),
        };
        let content = render_markdown("- [x] Done\n- [ ] Todo", Some(&options), None, None);
        assert!(content.contains("<input type=\"checkbox\" checked class=\"custom-task\" />"));
        assert!(content.contains("<input type=\"checkbox\" class=\"custom-task\" />"));
    }

    #[test]
    fn test_table_components() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new()
                .table(TestTable)
                .table_head(TestTableHead)
                .table_row(TestTableRow)
                .table_cell(TestTableCell),
        };
        let content = render_markdown(
            "| Header | Header |\n|--------|--------|\n| Cell   | Cell   |",
            Some(&options),
            None,
            None,
        );
        assert!(content.contains("<table class=\"custom-table\">"));
        assert!(content.contains("<thead class=\"custom-thead\">"));
        assert!(content.contains("<tr class=\"custom-row\">"));
        assert!(content.contains("<td class=\"custom-cell\">"));
    }
}
