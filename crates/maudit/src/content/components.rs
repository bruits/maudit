use pulldown_cmark::{Event, Tag, TagEnd};

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
    pub(crate) fn find_component(&self, event: &Event) -> Option<&(dyn MarkdownComponent + Send + Sync)> {
        self.components.iter().find(|c| c.handles(event)).map(|c| c.as_ref())
    }
}

/// Custom heading component example
pub struct CustomHeading;

impl MarkdownComponent for CustomHeading {
    fn handles(&self, event: &Event) -> bool {
        matches!(event, Event::Start(Tag::Heading { .. }) | Event::End(TagEnd::Heading { .. }))
    }
    
    fn render_start(&self, event: &Event) -> Option<String> {
        if let Event::Start(Tag::Heading { level, id, classes, .. }) = event {
            let id_attr = id.as_ref().map(|i| format!(" id=\"{}\"", i)).unwrap_or_default();
            let class_attr = if classes.is_empty() { 
                String::new() 
            } else { 
                format!(" class=\"{}\"", classes.iter().map(|c| c.as_ref()).collect::<Vec<_>>().join(" ")) 
            };
            
            Some(format!(
                "<h{level}{id_attr}{class_attr}><span class=\"heading-icon\">§</span>"
            ))
        } else {
            None
        }
    }
    
    fn render_end(&self, event: &Event) -> Option<String> {
        if let Event::End(TagEnd::Heading(level)) = event {
            Some(format!("</h{level}>"))
        } else {
            None
        }
    }
}
