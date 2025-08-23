use pulldown_cmark::{Event, Tag, TagEnd};

/// Trait for custom markdown components
pub trait MarkdownComponent {
    /// Render the start tag with custom HTML
    fn render_start(&self, event: &Event) -> Option<String>;

    /// Render the end tag with custom HTML (optional)
    fn render_end(&self, _event: &Event) -> Option<String> {
        None // Most components only need to customize the start tag
    }
}

/// Registry for custom markdown components
#[derive(Default)]
pub struct MarkdownComponents {
    pub heading: Option<Box<dyn MarkdownComponent + Send + Sync>>,
    pub paragraph: Option<Box<dyn MarkdownComponent + Send + Sync>>,
    pub link: Option<Box<dyn MarkdownComponent + Send + Sync>>,
    pub image: Option<Box<dyn MarkdownComponent + Send + Sync>>,
    pub strong: Option<Box<dyn MarkdownComponent + Send + Sync>>,
    pub emphasis: Option<Box<dyn MarkdownComponent + Send + Sync>>,
    pub code: Option<Box<dyn MarkdownComponent + Send + Sync>>,
    pub blockquote: Option<Box<dyn MarkdownComponent + Send + Sync>>,
}

impl MarkdownComponents {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a custom heading component
    pub fn heading<C: MarkdownComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.heading = Some(Box::new(component));
        self
    }

    /// Set a custom paragraph component
    pub fn paragraph<C: MarkdownComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.paragraph = Some(Box::new(component));
        self
    }

    /// Set a custom link component
    pub fn link<C: MarkdownComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.link = Some(Box::new(component));
        self
    }

    /// Set a custom image component
    pub fn image<C: MarkdownComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.image = Some(Box::new(component));
        self
    }

    /// Set a custom strong component
    pub fn strong<C: MarkdownComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.strong = Some(Box::new(component));
        self
    }

    /// Set a custom emphasis component
    pub fn emphasis<C: MarkdownComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.emphasis = Some(Box::new(component));
        self
    }

    /// Set a custom code component
    pub fn code<C: MarkdownComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.code = Some(Box::new(component));
        self
    }

    /// Set a custom blockquote component
    pub fn blockquote<C: MarkdownComponent + Send + Sync + 'static>(
        mut self,
        component: C,
    ) -> Self {
        self.blockquote = Some(Box::new(component));
        self
    }

    /// Find a component that can handle the given event
    pub(crate) fn find_component(
        &self,
        event: &Event,
    ) -> Option<&(dyn MarkdownComponent + Send + Sync)> {
        match event {
            Event::Start(Tag::Heading { .. }) | Event::End(TagEnd::Heading(_)) => {
                self.heading.as_ref().map(|c| c.as_ref())
            }
            Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph) => {
                self.paragraph.as_ref().map(|c| c.as_ref())
            }
            Event::Start(Tag::Link { .. }) | Event::End(TagEnd::Link) => {
                self.link.as_ref().map(|c| c.as_ref())
            }
            Event::Start(Tag::Image { .. }) | Event::End(TagEnd::Image) => {
                self.image.as_ref().map(|c| c.as_ref())
            }
            Event::Start(Tag::Strong) | Event::End(TagEnd::Strong) => {
                self.strong.as_ref().map(|c| c.as_ref())
            }
            Event::Start(Tag::Emphasis) | Event::End(TagEnd::Emphasis) => {
                self.emphasis.as_ref().map(|c| c.as_ref())
            }
            Event::Code(_) => self.code.as_ref().map(|c| c.as_ref()),
            Event::Start(Tag::BlockQuote { .. }) | Event::End(TagEnd::BlockQuote { .. }) => {
                self.blockquote.as_ref().map(|c| c.as_ref())
            }
            _ => None,
        }
    }
}
