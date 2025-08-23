// Component traits that hide pulldown-cmark implementation details

/// Trait for custom heading components
pub trait HeadingComponent {
    /// Render the opening tag
    fn render_start(&self, level: u8, id: Option<&str>, classes: &[&str]) -> String;

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
}
