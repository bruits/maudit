---
title: "Content"
description: "Learn how to load and use content and data in your Maudit site."
section: "core-concepts"
---

Maudit lets you load content and data in different formats to use across your website.

For example, you can configure a folder of Markdown (.md) files to be converted into HTML and included in your pages. You can also fetch a remote JSON file at build time and use its data within your site.

In Maudit, this concept is called Content Sources. A content source is a collection of content or data, usually (but not necessarily) structured in a homogeneous way, that your website can use.

## Defining a content source

Content sources are defined in the coronate entry point through the `content_sources!` macro.

```rs
use maudit::content::content_sources;

#[markdown_entry]
pub struct BlogPost {
  pub title: String,
  pub description: Option<String>,
}

fn main() {
  coronate(
    routes![
      // ...
    ],
    content_sources![
      "source_name" => loader(...),
      "another_source" => glob_markdown<BlogPost>("path/to/files/*.md", None)
    ],
    Default::default()
  );
}
```

Where `loader` and `glob_markdown` are functions returning a Vec of `ContentEntry`. Typically, a loader also accepts a type argument specifying the shape of the data for each entries it returns, which will be used inside your pages to provide typed content.

## Using a content source in pages

Once a content source is defined, it can be accessed in pages through the `PageContext#content` property.

```rs
use maudit::route::prelude::*;
use maud::{html, PreEscaped};

#[route("/some-article")]
pub struct SomeArticlePage;

impl Route for SomeArticlePage {
  fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
    let entry = ctx
      .content
      .get_source::<BlogPost>("source_name")
      .get_entry("entry_id");

    let entry_data = entry.data(ctx);

    html! {
      h1 { (entry_data.title) }
      @if let Some(description) = &entry_data.description {
        p { (description) }
      }

      (PreEscaped(entry.render(ctx)))
    }
  }
}
```

## Loaders

### Built-in loaders

#### `glob_markdown`

The `glob_markdown` loader can be used to load one or multiple folders of Markdown (`.md`) files.

```rs
use maudit::content::{glob_markdown, markdown_entry};

#[markdown_entry]
pub struct DocsContent {
    pub title: String,
    pub description: Option<String>,
    pub section: Option<DocsSection>,
}

"docs" => glob_markdown::<DocsContent>("content/docs/*.md", None)
```

This loader take a glob pattern (compatible with [the `glob` crate](https://github.com/rust-lang/glob)) as its first argument, and an optional `MarkdownOptions` struct as its second argument to customise Markdown rendering. The frontmatter of each Markdown file will be deserialized using [Serde](https://serde.rs) into the type argument provided to `glob_markdown`, which can use the `#[markdown_entry]` macro to derive the necessary traits and add the necessary properties to the struct. Note that using this feature require the installation of Serde into your project as the macro uses Serde's derive macros.

### Custom loaders

As said previously, a loader is simply a function returning a Vec of `ContentEntry`. This means you can create your own loaders to load content from any source you want, as long as you return the right type.

For instance, you could create a loader that fetches a remote JSON file and deserializes it into a struct, producing a content source with a single entry:

```rs
use maudit::content::{ContentEntry};

#[derive(serde::Deserialize)]
pub struct MyType {
    pub id: u32,
    pub name: String,
}

pub fn my_loader(path: &str) -> Vec<ContentEntry<MyType>> {
    let response = reqwest::blocking::get(path).unwrap();
    let data = response.json::<MyType>().unwrap();

    vec![ContentEntry::new(data.id.into(), None, None, data, None)]
}

// Use it as a content source:
use maudit::content::content_sources;

content_sources![
    "my_data" => my_loader("https://example.com/data.json")
];
```

and then in pages, you could access the data like this:

```rs
use maudit::route::prelude::*;

#[route("/data")]
pub struct DataPage;

impl Route for DataPage {
    fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
        let entry = ctx
            .content
            .get_source::<MyType>("my_data")
            .get_entry("0");

        let entry_data = entry.data();

        format!(
            "<h1>Data</h1><p>ID: {}, Name: {}</p>",
            entry_data.id, entry_data.name
        ).into()
    }
}
```

Content entries can also be rendered by passing a render function to the `render` method of `ContentEntry`.

```rs
ContentEntry::new(
  data.id.into(),
  Some(Box::new(|content, ctx| {
    // render the content string into HTML
    maudit::render_markdown(content, markdown_options, None, ctx)
  })),
  None,
  data,
  None,
)
```

## Markdown rendering

Either through loaders or by using the [`render_markdown`](https://docs.rs/maudit/latest/maudit/content/markdown/fn.render_markdown.html) function directly, Maudit supports rendering local and remote Markdown and enriching it with shortcodes and custom components.

### Shortcodes

Shortcodes provide a way to extend Markdown with custom functionality. They serve a similar role to [components in MDX](https://mdxjs.com) or [tags in Markdoc](https://markdoc.dev/docs/tags), allowing authors to define and reuse snippets throughout their content. Shortcodes can accept attributes and content, and can be self-closing or not.

```markdown
---
title: { { enhance title="Super Title" / } }
---

Here's an image with a caption:

{{ image src="./image.png" }}
This is a caption!
{{ /image }}
```

This snippet could expand into something like this:

```markdown
---
title: Very Cool Super Title
---

Here's an image with a caption:

<figure>
    <img src="/_maudit/image.hash.webp" width="200" height="200" loading="lazy" decoding="async" />
    <figcaption>This is a caption!</figcaption>
</figure>
```

To define a shortcode, create an instance of `MarkdownShortcodes` and use the `register` method to register shortcodes. This can then be passed as the `shortcodes` parameter of `MarkdownOptions`.

```rs
use maudit::shortcodes::MarkdownShortcodes;

fn main() {
    let create_markdown_options = || {
        let mut shortcodes = MarkdownShortcodes::default();

        shortcodes.register("enhance", |attrs, _| {
            let title = attrs.get_required("title");

            format!("Very Cool {}", title)
        })

        MarkdownOptions {
            shortcodes,
            ..Default::default()
        }
    }

    coronate(
        routes![
            // ...
        ],
        content_sources![
            "blog" => glob_markdown::<BlogPost>("content/blog/**/*.md", Some(create_markdown_options())),
        ],
        ..Default::default()
    );
}
```

Note that shortcodes expand before Markdown is rendered, so you can use shortcodes to generate Markdown content as well as HTML.

### Components

Maudit supports using custom components to render Markdown content. For instance, by default `# Title` will be rendered as `<h1>Title</h1>`, but you can override this behaviour by providing your own component for headings.

To do so, create an instance of `MarkdownComponents` and use the various builder (`.heading`, `.link`, `.paragraph`, etc.) methods to register components. This can then be passed as the `components` parameter of `MarkdownOptions`.

```rs
use maudit::components::MarkdownComponents;

struct TestCustomHeading;

impl HeadingComponent for TestCustomHeading {
    fn render_start(&self, level: u8, id: Option<&str>, classes: &[&str]) -> String {
        let id_attr = id.map(|i| format!(" id=\"{}\"", i)).unwrap_or_default();
        let class_attr = if classes.is_empty() {
            String::new()
        } else {
            format!(" class=\"{}\"", classes.join(" "))
        };
        format!(
            "<h{}{}{}> This is a custom Heading: ",
            level, id_attr, class_attr
        )
    }

    fn render_end(&self, level: u8) -> String {
        format!("</h{}>", level)
    }
}

fn main() {
    let create_markdown_options = || {
        let mut components = MarkdownComponents::new().heading(TestCustomHeading);

        MarkdownOptions {
            components,
            ..Default::default()
        }
    };

    coronate(
        routes![
            // ...
        ],
        content_sources![
            "blog" => glob_markdown::<BlogPost>("content/blog/**/*.md", Some(create_markdown_options())),
        ],
        ..Default::default(),
    );
}

```

Unlike shortcodes, components are used during the Markdown rendering process, so they can only generate HTML, not Markdown.
