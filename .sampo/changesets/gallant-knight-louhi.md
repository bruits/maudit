---
packages:
  - maudit
release: minor
---

Added support for shortcodes in Markdown. Shortcodes allows you to substitute custom content in your Markdown files. This feature is useful for embedding dynamic content or reusable components within your Markdown documents.

For instance, you might define a shortcode for embedding YouTube videos using only the video ID, or for inserting custom alerts or notes.

```markdown
{{ youtube id="FbJ63spk48s" }}
```

Would render to:

```html
<iframe
  width="560"
  height="315"
  src="https://www.youtube.com/embed/FbJ63spk48s?si=hUGRndTWIThVY-72"
  title="YouTube video player"
  frameborder="0"
  allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
  referrerpolicy="strict-origin-when-cross-origin"
  allowfullscreen
></iframe>
```

To define and register shortcodes, pass a MarkdownShortcodes instance to the MarkdownOptions when rendering Markdown content.

```rust
let mut shortcodes = MarkdownShortcodes::new();

shortcodes.register("youtube", |args, _ctx| {
    let id: String = args.get_required("id");
    format!(
        r#"<iframe width="560" height="315" src="https://www.youtube.com/embed/{}" frameborder="0" allowfullscreen></iframe>"#,
        id
    )
});

MarkdownOptions {
    shortcodes,
    ..Default::default()
}

// Then pass options to, i.e. glob_markdown in a content source
```

Note that shortcodes are expanded before Markdown is rendered, so you can use shortcodes anywhere in your Markdown content, for instance in your frontmatter. Additionally, shortcodes may expand to Markdown content, which will then be rendered as part of the overall Markdown rendering process.
