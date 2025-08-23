use pulldown_cmark::Event;

/// Trait for custom markdown components
pub trait MarkdownComponent {
    /// Render the start tag with custom HTML
    fn render_start(&self, event: &Event) -> Option<String>;

    /// Render the end tag with custom HTML (optional)
    fn render_end(&self, _event: &Event) -> Option<String> {
        None // Most components only need to customize the start tag
    }

    /// Whether this component handles the given event
    fn handles(&self, event: &Event) -> bool;
}

/// Registry for custom markdown components
#[derive(Default)]
pub struct MarkdownComponents {
    components: Vec<Box<dyn MarkdownComponent + Send + Sync>>,
}

impl MarkdownComponents {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a custom component
    pub fn register<C: MarkdownComponent + Send + Sync + 'static>(mut self, component: C) -> Self {
        self.components.push(Box::new(component));
        self
    }

    /// Find a component that can handle the given event
    pub(crate) fn find_component(
        &self,
        event: &Event,
    ) -> Option<&(dyn MarkdownComponent + Send + Sync)> {
        self.components
            .iter()
            .find(|c| c.handles(event))
            .map(|c| c.as_ref())
    }
}
