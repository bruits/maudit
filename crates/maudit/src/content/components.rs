// Component traits that hide pulldown-cmark implementation details

/// Trait for custom heading components
pub trait HeadingComponent {
    /// Render the opening tag
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

    /// Render the closing tag (optional)
    fn render_end(&self, level: u8) -> String {
        format!("</h{}>", level)
    }
}

/// Trait for custom paragraph components
pub trait ParagraphComponent {
    /// Render the opening tag
    fn render_start(&self) -> String {
        "<p>".to_string()
    }

    /// Render the closing tag
    fn render_end(&self) -> String {
        "</p>".to_string()
    }
}

/// Trait for custom link components
pub trait LinkComponent {
    /// Render the opening tag
    fn render_start(&self, url: &str, title: Option<&str>, link_type: &str) -> String;

    /// Render the closing tag
    fn render_end(&self) -> String {
        "</a>".to_string()
    }
}

/// Trait for custom image components
pub trait ImageComponent {
    /// Render the image tag
    fn render(&self, url: &str, alt: &str, title: Option<&str>) -> String;
}

/// Trait for custom strong/bold components
pub trait StrongComponent {
    /// Render the opening tag
    fn render_start(&self) -> String {
        "<strong>".to_string()
    }

    /// Render the closing tag
    fn render_end(&self) -> String {
        "</strong>".to_string()
    }
}

/// Trait for custom emphasis/italic components
pub trait EmphasisComponent {
    /// Render the opening tag
    fn render_start(&self) -> String {
        "<em>".to_string()
    }

    /// Render the closing tag
    fn render_end(&self) -> String {
        "</em>".to_string()
    }
}

/// Trait for custom inline code components
pub trait CodeComponent {
    /// Render the code span
    fn render(&self, code: &str) -> String;
}

/// Trait for custom blockquote components
pub trait BlockquoteComponent {
    /// Render the opening tag
    fn render_start(&self, kind: Option<&str>) -> String {
        match kind {
            Some(k) => format!("<blockquote data-kind=\"{}\">", k),
            None => "<blockquote>".to_string(),
        }
    }

    /// Render the closing tag
    fn render_end(&self) -> String {
        "</blockquote>".to_string()
    }
}

/// Registry for custom markdown components
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
}

impl MarkdownComponents {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any components are defined
    pub fn has_any_components(&self) -> bool {
        self.heading.is_some()
            || self.paragraph.is_some()
            || self.link.is_some()
            || self.image.is_some()
            || self.strong.is_some()
            || self.emphasis.is_some()
            || self.code.is_some()
            || self.blockquote.is_some()
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{render_markdown, MarkdownOptions};

    // Define a custom heading component for testing
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

    // Define a custom paragraph component for testing
    struct TestCustomParagraph;

    impl ParagraphComponent for TestCustomParagraph {
        fn render_start(&self) -> String {
            "<p class=\"custom-paragraph\">".to_string()
        }

        fn render_end(&self) -> String {
            "</p><!-- end custom paragraph -->".to_string()
        }
    }

    // Define a custom link component for testing
    struct TestCustomLink;

    impl LinkComponent for TestCustomLink {
        fn render_start(&self, url: &str, title: Option<&str>, _link_type: &str) -> String {
            let title_attr = title
                .map(|t| format!(" title=\"{}\"", t))
                .unwrap_or_default();
            format!("<a href=\"{}\" class=\"custom-link\"{}>🔗", url, title_attr)
        }

        fn render_end(&self) -> String {
            "</a>".to_string()
        }
    }

    // Define a custom image component for testing
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

    // Define a custom strong component for testing
    struct TestCustomStrong;

    impl StrongComponent for TestCustomStrong {
        fn render_start(&self) -> String {
            "<strong class=\"custom-strong\">💪".to_string()
        }

        fn render_end(&self) -> String {
            "</strong>".to_string()
        }
    }

    // Define a custom emphasis component for testing
    struct TestCustomEmphasis;

    impl EmphasisComponent for TestCustomEmphasis {
        fn render_start(&self) -> String {
            "<em class=\"custom-emphasis\">✨".to_string()
        }

        fn render_end(&self) -> String {
            "</em>".to_string()
        }
    }

    // Define a custom code component for testing
    struct TestCustomCode;

    impl CodeComponent for TestCustomCode {
        fn render(&self, code: &str) -> String {
            format!("<code class=\"custom-code\">💻{}</code>", code)
        }
    }

    // Define a custom blockquote component for testing
    struct TestCustomBlockquote;

    impl BlockquoteComponent for TestCustomBlockquote {
        fn render_start(&self, kind: Option<&str>) -> String {
            match kind {
                Some(k) => format!(
                    "<blockquote class=\"custom-blockquote {}\" data-kind=\"{}\">📝",
                    k, k
                ),
                None => "<blockquote class=\"custom-blockquote\">📝".to_string(),
            }
        }

        fn render_end(&self) -> String {
            "</blockquote>".to_string()
        }
    }

    #[test]
    fn test_components_builder_pattern() {
        let components = MarkdownComponents::new().heading(TestCustomHeading);

        // Test that builder pattern works
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

        let html = render_markdown("# Hello, world!", Some(&options));
        assert!(html.contains("🎯Hello, world!"));
    }

    #[test]
    fn test_custom_paragraph_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().paragraph(TestCustomParagraph),
        };

        let content = render_markdown("This is a paragraph.", Some(&options));
        assert!(content.contains(
            "<p class=\"custom-paragraph\">This is a paragraph.</p><!-- end custom paragraph -->"
        ));
    }

    #[test]
    fn test_custom_link_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().link(TestCustomLink),
        };

        let content = render_markdown("[Example](https://example.com)", Some(&options));
        assert!(
            content.contains("<a href=\"https://example.com\" class=\"custom-link\">🔗Example</a>")
        );
    }

    #[test]
    fn test_custom_image_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().image(TestCustomImage),
        };

        let content = render_markdown("![Alt text](image.jpg)", Some(&options));
        assert!(
            content.contains("<img src=\"image.jpg\" alt=\"Alt text\" class=\"custom-image\" />📸")
        );
    }

    #[test]
    fn test_custom_strong_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().strong(TestCustomStrong),
        };

        let content = render_markdown("**Bold text**", Some(&options));
        assert!(content.contains("<strong class=\"custom-strong\">💪Bold text</strong>"));
    }

    #[test]
    fn test_custom_emphasis_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().emphasis(TestCustomEmphasis),
        };

        let content = render_markdown("*Italic text*", Some(&options));
        assert!(content.contains("<em class=\"custom-emphasis\">✨Italic text</em>"));
    }

    #[test]
    fn test_custom_code_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().code(TestCustomCode),
        };

        let content = render_markdown("`console.log('hello')`", Some(&options));
        assert!(content.contains("<code class=\"custom-code\">💻console.log('hello')</code>"));
    }

    #[test]
    fn test_custom_blockquote_component() {
        let options = MarkdownOptions {
            components: MarkdownComponents::new().blockquote(TestCustomBlockquote),
        };

        let content = render_markdown("> This is a quote", Some(&options));
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
        );
        assert!(content.contains("<blockquote class=\"custom-blockquote\">📝"));
        assert!(content.contains("<strong class=\"custom-strong\">💪bold</strong>"));
        assert!(content.contains("<em class=\"custom-emphasis\">✨italic</em>"));
        assert!(content.contains("<code class=\"custom-code\">💻code</code>"));
        assert!(content.contains("</blockquote>"));
    }
}
