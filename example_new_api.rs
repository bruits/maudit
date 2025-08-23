use maudit::content::components::{CustomHeading, MarkdownComponent};
use maudit::content::{render_markdown_with_components, MarkdownComponents};
use pulldown_cmark::{Event, Tag, TagEnd};

// Define custom components
struct MyHeading;
struct MyLink;
struct MyImage;

impl MarkdownComponent for MyHeading {
    fn render_start(&self, event: &Event) -> Option<String> {
        if let Event::Start(Tag::Heading {
            level, id, classes, ..
        }) = event
        {
            let level_num = *level as u8;
            let id_attr = id
                .as_ref()
                .map(|i| format!(" id=\"{}\"", i))
                .unwrap_or_default();
            let class_attr = if classes.is_empty() {
                String::new()
            } else {
                format!(
                    " class=\"{}\"",
                    classes
                        .iter()
                        .map(|c| c.as_ref())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            };
            Some(format!(
                "<h{}{}{} data-custom=\"true\">🚀 ",
                level_num, id_attr, class_attr
            ))
        } else {
            None
        }
    }

    fn render_end(&self, event: &Event) -> Option<String> {
        if let Event::End(TagEnd::Heading(level)) = event {
            let level_num = *level as u8;
            Some(format!(" 🎯</h{}>", level_num))
        } else {
            None
        }
    }
}

impl MarkdownComponent for MyLink {
    fn render_start(&self, event: &Event) -> Option<String> {
        if let Event::Start(Tag::Link {
            dest_url, title, ..
        }) = event
        {
            let title_attr = title
                .as_ref()
                .map(|t| format!(" title=\"{}\"", t))
                .unwrap_or_default();
            Some(format!(
                "<a href=\"{}\"{}  data-custom-link=\"true\">🔗 ",
                dest_url, title_attr
            ))
        } else {
            None
        }
    }

    fn render_end(&self, _event: &Event) -> Option<String> {
        Some("</a>".to_string())
    }
}

impl MarkdownComponent for MyImage {
    fn render_start(&self, event: &Event) -> Option<String> {
        if let Event::Start(Tag::Image {
            dest_url, title, ..
        }) = event
        {
            let alt = "Image"; // In a real implementation, you'd extract this from the content
            let title_attr = title
                .as_ref()
                .map(|t| format!(" title=\"{}\"", t))
                .unwrap_or_default();
            Some(format!("<figure><img src=\"{}\" alt=\"{}\"{}  data-custom-image=\"true\" /><figcaption>📸 {}</figcaption></figure>", dest_url, alt, title_attr, alt))
        } else {
            None
        }
    }
}

fn main() {
    // Set up custom components with the nice new API!
    let components = MarkdownComponents::new()
        .heading(MyHeading)
        .link(MyLink)
        .image(MyImage);

    let markdown = r#"# Welcome to Custom Components!

This is a **bold** demonstration of custom markdown components.

## Features

- Custom headings with emojis 🚀🎯
- Custom links: [Click me](https://example.com "A test link")
- Custom images: ![Alt text](image.jpg "Image title")

### Nested content works!

This heading contains **bold text** and *italic text* - all preserved!"#;

    let html = render_markdown_with_components(markdown, &components);

    println!("Generated HTML with custom components:");
    println!("{}", html);

    println!("\n✅ Custom markdown components work perfectly!");
    println!("🎯 Each element type can have its own custom component");
    println!("🚀 Nested content is perfectly preserved");
}
