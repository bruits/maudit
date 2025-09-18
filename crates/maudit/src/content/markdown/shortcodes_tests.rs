#[cfg(test)]
mod tests {
    use crate::{
        content::shortcodes::{preprocess_shortcodes, MarkdownShortcodes},
        page::RouteContext,
    };

    fn create_test_shortcodes() -> MarkdownShortcodes {
        let mut shortcodes = MarkdownShortcodes::new();

        // Simple shortcode that just returns its name
        shortcodes.register("simple", |_args, _| "SIMPLE_OUTPUT".to_string());

        // Shortcode with arguments
        shortcodes.register("greet", |args, _| {
            let name = args.get_str("name").unwrap_or("World");
            format!("Hello, {}!", name)
        });

        // Date shortcode with format
        shortcodes.register("date", |args, _| {
            let format = args.get_str("format").unwrap_or("default");
            format!("DATE[{}]", format)
        });

        // Block shortcode that wraps content
        shortcodes.register("highlight", |args, _| {
            let lang = args.get_str("lang").unwrap_or("text");
            let body = args.get_str("body").unwrap_or("");
            format!("<code lang=\"{}\">{}</code>", lang, body)
        });

        // Section shortcode for testing nested content
        shortcodes.register("section", |args, _| {
            let title = args.get_str("title").unwrap_or("");
            let body = args.get_str("body").unwrap_or("");
            if title.is_empty() {
                format!("<section>{}</section>", body)
            } else {
                format!("<section title=\"{}\">{}</section>", title, body)
            }
        });

        // Alert shortcode with type
        shortcodes.register("alert", |args, _| {
            let alert_type = args.get_str("type").unwrap_or("info");
            let body = args.get_str("body").unwrap_or("");
            format!("<div class=\"alert alert-{}\">{}</div>", alert_type, body)
        });

        shortcodes
    }

    // Helper function to create a minimal RouteContext for testing
    fn with_test_route_context<F, R>(f: F) -> R
    where
        F: for<'a> FnOnce(&mut RouteContext<'a>) -> R,
    {
        use crate::{
            assets::PageAssets,
            content::{ContentSources, PageContent},
        };
        use std::path::PathBuf;

        let content_sources = ContentSources::new(vec![]);
        let content = PageContent::new(&content_sources);
        let mut page_assets = PageAssets {
            assets_dir: PathBuf::from("assets"),
            ..Default::default()
        };

        let mut ctx = RouteContext {
            content: &content,
            assets: &mut page_assets,
            current_url: "/test".to_string(),
            params: &(),
            props: &(),
        };

        f(&mut ctx)
    }

    // Helper function for tests that don't need RouteContext
    fn preprocess_shortcodes_simple(
        content: &str,
        shortcodes: &MarkdownShortcodes,
    ) -> Result<String, String> {
        preprocess_shortcodes(content, shortcodes, None, None)
    }

    // Helper function that automatically wraps RouteContext in Some() for existing tests
    fn preprocess_shortcodes_with_ctx(
        content: &str,
        shortcodes: &MarkdownShortcodes,
        route_ctx: &mut RouteContext,
    ) -> Result<String, String> {
        preprocess_shortcodes(content, shortcodes, Some(route_ctx), None)
    }

    #[test]
    fn test_no_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "This is just plain text with no shortcodes.";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx).unwrap()
        });
        assert_eq!(result, content);
    }

    #[test]
    fn test_simple_self_closing_shortcode() {
        let shortcodes = create_test_shortcodes();
        let content = "Before {{ simple /}} after";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx).unwrap()
        });
        assert_eq!(result, "Before SIMPLE_OUTPUT after");
    }

    #[test]
    fn test_shortcode_with_arguments() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ greet name=Alice /}}";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx).unwrap()
        });
        assert_eq!(result, "Hello, Alice!");
    }

    #[test]
    fn test_multiple_arguments() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ date day=Monday month=January year=2024 /}}";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx).unwrap()
        });
        assert_eq!(result, "DATE[default]");
    }

    #[test]
    fn test_frontmatter_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"---
title: {{ greet name=Blog /}}
date: {{ date format=iso /}}
---

# Content here"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx).unwrap()
        });
        let expected = r#"---
title: Hello, Blog!
date: DATE[iso]
---

# Content here"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_shortcodes_in_headings() {
        let shortcodes = create_test_shortcodes();
        let content = "# {{ greet name=Header /}}\n\n## Section {{ date format=short /}}";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx).unwrap()
        });
        assert_eq!(result, "# Hello, Header!\n\n## Section DATE[short]");
    }

    #[test]
    fn test_shortcodes_in_links() {
        let shortcodes = create_test_shortcodes();
        let content = "[{{ greet name=Link /}}](https://example.com)";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx).unwrap()
        });
        assert_eq!(result, "[Hello, Link!](https://example.com)");
    }

    #[test]
    fn test_shortcodes_in_code_blocks() {
        let shortcodes = create_test_shortcodes();
        let content = "```\nSome code with {{ simple /}}\n```";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx).unwrap()
        });
        assert_eq!(result, "```\nSome code with SIMPLE_OUTPUT\n```");
    }

    #[test]
    fn test_block_shortcode() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ highlight lang=rust }}\nlet x = 5;\n{{ /highlight }}";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx).unwrap()
        });
        assert_eq!(result, "<code lang=\"rust\">\nlet x = 5;\n</code>");
    }

    #[test]
    fn test_nested_shortcodes_in_block() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ section title=Main }}\nHello {{ greet name=World /}}!\n{{ /section }}";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(
            result,
            "<section title=\"Main\">\nHello Hello, World!!\n</section>"
        );
    }

    #[test]
    fn test_deeply_nested_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"{{ section title=Outer }}
{{ alert type=warning }}
{{ highlight lang=javascript }}
console.log("{{ greet name=Nested /}}");
{{ /highlight }}
{{ /alert }}
{{ /section }}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"<section title="Outer">
<div class="alert alert-warning">
<code lang="javascript">
console.log("Hello, Nested!");
</code>
</div>
</section>"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_multiple_shortcodes_same_line() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ greet name=Alice /}} and {{ greet name=Bob /}}";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "Hello, Alice! and Hello, Bob!");
    }

    #[test]
    fn test_shortcodes_in_lists() {
        let shortcodes = create_test_shortcodes();
        let content = r#"- Item 1: {{ greet name=First /}}
- Item 2: {{ date format=short /}}
- Item 3: {{ simple /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"- Item 1: Hello, First!
- Item 2: DATE[short]
- Item 3: SIMPLE_OUTPUT"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_shortcodes_in_tables() {
        let shortcodes = create_test_shortcodes();
        let content = r#"| Name | Greeting |
|------|----------|
| Alice | {{ greet name=Alice /}} |
| Bob | {{ greet name=Bob /}} |"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"| Name | Greeting |
|------|----------|
| Alice | Hello, Alice! |
| Bob | Hello, Bob! |"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_shortcodes_with_special_characters() {
        let shortcodes = create_test_shortcodes();
        let content = "Before\n{{ simple /}}\nAfter\n\n{{ greet name=Test /}}";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "Before\nSIMPLE_OUTPUT\nAfter\n\nHello, Test!");
    }

    #[test]
    fn test_error_unknown_shortcode() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ unknown_shortcode /}}";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        });
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Unknown shortcode: 'unknown_shortcode'"));
    }

    #[test]
    fn test_unclosed_shortcode_treated_as_literal() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ simple ";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        // Should treat as literal text since there's no closing }}
        assert_eq!(result, "{{ simple ");
    }

    #[test]
    fn test_unclosed_shortcode_with_valid_shortcode_after() {
        let shortcodes = create_test_shortcodes();
        let content = "Before {{ unclosed. Then {{ simple /}} after.";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        // Should treat first {{ as literal and process the second shortcode
        assert_eq!(result, "Before {{ unclosed. Then SIMPLE_OUTPUT after.");
    }

    #[test]
    fn test_multiple_unclosed_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ first {{ second {{ third";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        // All should be treated as literal text
        assert_eq!(result, "{{ first {{ second {{ third");
    }

    #[test]
    fn test_error_empty_shortcode() {
        let shortcodes = create_test_shortcodes();
        let content = "{{  }}";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty shortcode"));
    }

    #[test]
    fn test_error_invalid_argument_format() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ greet name Alice /}}";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid argument format"));
    }

    #[test]
    fn test_error_unexpected_closing_tag() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ /section }}";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unexpected closing tag"));
    }

    #[test]
    fn test_whitespace_handling() {
        let shortcodes = create_test_shortcodes();
        let content = "{{   simple   /}}";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "SIMPLE_OUTPUT");
    }

    #[test]
    fn test_whitespace_in_arguments() {
        let shortcodes = create_test_shortcodes();
        let content = "{{  greet   name=Alice  /}}";
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "Hello, Alice!");
    }

    #[test]
    fn test_complex_markdown_document() {
        let shortcodes = create_test_shortcodes();
        let content = r#"---
title: {{ greet name=Blog /}}
author: {{ greet name=Author /}}
---

# {{ greet name=Reader /}}

Welcome to my blog! Today is {{ date format=full /}}.

## Code Example

{{ highlight lang=rust }}
fn main() {
    println!("{{ greet name=Rust /}}");
}
{{ /highlight }}

## Alert Section

{{ alert type=info }}
This is an important message with {{ simple /}} content.
{{ /alert }}

- List item with {{ greet name=Item /}}
- Another item: {{ date format=short /}}

> Quote with {{ simple /}} shortcode

[Link with {{ greet name=Link /}}](http://example.com)"#;

        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"---
title: Hello, Blog!
author: Hello, Author!
---

# Hello, Reader!

Welcome to my blog! Today is DATE[full].

## Code Example

<code lang="rust">
fn main() {
    println!("Hello, Rust!");
}
</code>

## Alert Section

<div class="alert alert-info">
This is an important message with SIMPLE_OUTPUT content.
</div>

- List item with Hello, Item!
- Another item: DATE[short]

> Quote with SIMPLE_OUTPUT shortcode

[Link with Hello, Link!](http://example.com)"#;
        assert_eq!(result, expected);
    }

    // Integration tests with full markdown rendering
    #[test]
    fn test_markdown_integration_headings_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "# {{ greet name=Title /}}\n\n## Section {{ date format=short /}}";

        // Test shortcode preprocessing first
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(processed, "# Hello, Title!\n\n## Section DATE[short]");
    }

    #[test]
    fn test_markdown_integration_emphasis_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "*{{ greet name=Italic /}}* and **{{ greet name=Bold /}}**";
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(processed, "*Hello, Italic!* and **Hello, Bold!**");
    }

    #[test]
    fn test_markdown_integration_code_spans_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "Use `{{ simple /}}` in your code";
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(processed, "Use `SIMPLE_OUTPUT` in your code");
    }

    #[test]
    fn test_markdown_integration_blockquotes_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "> {{ greet name=Quote /}}\n> \n> {{ simple /}}";
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(processed, "> Hello, Quote!\n> \n> SIMPLE_OUTPUT");
    }

    #[test]
    fn test_markdown_integration_nested_lists_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"1. {{ greet name=First /}}
   - Nested {{ simple /}}
   - {{ date format=iso /}}
2. {{ greet name=Second /}}
   1. Numbered {{ simple /}}
   2. {{ greet name=Nested /}}"#;
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"1. Hello, First!
   - Nested SIMPLE_OUTPUT
   - DATE[iso]
2. Hello, Second!
   1. Numbered SIMPLE_OUTPUT
   2. Hello, Nested!"#;
        assert_eq!(processed, expected);
    }

    #[test]
    fn test_markdown_integration_complex_tables_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"| **{{ greet name=Header /}}** | _{{ date format=long /}}_ |
|:---------------------------|-------------------------:|
| {{ simple /}}               | {{ greet name=Cell /}}     |
| `{{ greet name=Code /}}`    | > {{ simple /}}            |"#;
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"| **Hello, Header!** | _DATE[long]_ |
|:---------------------------|-------------------------:|
| SIMPLE_OUTPUT               | Hello, Cell!     |
| `Hello, Code!`    | > SIMPLE_OUTPUT            |"#;
        assert_eq!(processed, expected);
    }

    #[test]
    fn test_markdown_integration_task_lists_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"- [x] {{ greet name=Done /}}
- [ ] {{ simple /}}
- [ ] {{ date format=todo /}}"#;
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"- [x] Hello, Done!
- [ ] SIMPLE_OUTPUT
- [ ] DATE[todo]"#;
        assert_eq!(processed, expected);
    }

    #[test]
    fn test_markdown_integration_strikethrough_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "~~{{ greet name=Deleted /}}~~ and {{ simple /}}";
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(processed, "~~Hello, Deleted!~~ and SIMPLE_OUTPUT");
    }

    #[test]
    fn test_markdown_integration_horizontal_rules_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ greet name=Before /}}\n\n---\n\n{{ simple /}}";
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(processed, "Hello, Before!\n\n---\n\nSIMPLE_OUTPUT");
    }

    #[test]
    fn test_markdown_integration_footnotes_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ greet name=Text /}}[^1]\n\n[^1]: {{ simple /}}";
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(processed, "Hello, Text![^1]\n\n[^1]: SIMPLE_OUTPUT");
    }

    #[test]
    fn test_markdown_integration_complex_links_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"[{{ greet name=Link /}}](https://example.com "{{ simple /}}")

![{{ greet name=Alt /}}](image.jpg "{{ date format=title /}}")

[Reference {{ simple /}}][ref]

[ref]: https://example.com "{{ greet name=RefTitle /}}""#;
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"[Hello, Link!](https://example.com "SIMPLE_OUTPUT")

![Hello, Alt!](image.jpg "DATE[title]")

[Reference SIMPLE_OUTPUT][ref]

[ref]: https://example.com "Hello, RefTitle!""#;
        assert_eq!(processed, expected);
    }

    #[test]
    fn test_markdown_integration_fenced_code_blocks_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"```rust
fn main() {
    println!("{{ greet name=Rust /}}");
    // {{ simple /}}
}
```

```{{ greet name=Language /}}
{{ simple /}}
```"#;
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"```rust
fn main() {
    println!("Hello, Rust!");
    // SIMPLE_OUTPUT
}
```

```Hello, Language!
SIMPLE_OUTPUT
```"#;
        assert_eq!(processed, expected);
    }

    #[test]
    fn test_markdown_integration_html_blocks_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"<div class="custom">
  <h2>{{ greet name=HTML /}}</h2>
  <p>{{ simple /}}</p>
</div>

<img src="test.jpg" alt="{{ greet name=Alt /}}" title="{{ date format=attr /}}">"#;
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"<div class="custom">
  <h2>Hello, HTML!</h2>
  <p>SIMPLE_OUTPUT</p>
</div>

<img src="test.jpg" alt="Hello, Alt!" title="DATE[attr]">"#;
        assert_eq!(processed, expected);
    }

    #[test]
    fn test_markdown_integration_math_blocks_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"Inline math: ${{ simple /}}$

Block math:
$$
{{ greet name=Math /}}
$$

{{ greet name=Text /}} with $x = {{ simple /}}$ inline."#;
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"Inline math: $SIMPLE_OUTPUT$

Block math:
$$
Hello, Math!
$$

Hello, Text! with $x = SIMPLE_OUTPUT$ inline."#;
        assert_eq!(processed, expected);
    }

    #[test]
    fn test_markdown_integration_frontmatter_yaml_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"---
title: "{{ greet name=Blog /}}"
description: {{ simple /}}
tags:
  - {{ greet name=Tag1 /}}
  - {{ simple /}}
metadata:
  created: {{ date format=iso /}}
  author: {{ greet name=Author /}}
---

# {{ greet name=Content /}}

{{ simple /}}"#;
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"---
title: "Hello, Blog!"
description: SIMPLE_OUTPUT
tags:
  - Hello, Tag1!
  - SIMPLE_OUTPUT
metadata:
  created: DATE[iso]
  author: Hello, Author!
---

# Hello, Content!

SIMPLE_OUTPUT"#;
        assert_eq!(processed, expected);
    }

    #[test]
    fn test_markdown_integration_block_shortcodes_with_markdown() {
        let shortcodes = create_test_shortcodes();
        let content = r#"{{ section title=Main }}
# {{ greet name=Header /}}

**{{ greet name=Bold /}}** and *{{ simple /}}*

- {{ greet name=Item1 /}}
- {{ simple /}}

> {{ greet name=Quote /}}

{{ /section }}"#;
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"<section title="Main">
# Hello, Header!

**Hello, Bold!** and *SIMPLE_OUTPUT*

- Hello, Item1!
- SIMPLE_OUTPUT

> Hello, Quote!

</section>"#;
        assert_eq!(processed, expected);
    }

    #[test]
    fn test_markdown_integration_edge_cases() {
        let shortcodes = create_test_shortcodes();
        let content = r#"{{ greet name=Start /}}

<!-- {{ simple /}} in comment -->

{{ highlight lang=markdown }}
# {{ greet name=NestedMD /}}
{{ simple /}}
{{ /highlight }}

`{{ greet name=BacktickCode /}}`

{{ greet name=End /}}"#;
        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"Hello, Start!

<!-- SIMPLE_OUTPUT in comment -->

<code lang="markdown">
# Hello, NestedMD!
SIMPLE_OUTPUT
</code>

`Hello, BacktickCode!`

Hello, End!"#;
        assert_eq!(processed, expected);
    }

    #[test]
    fn test_markdown_integration_real_world_blog_post() {
        let shortcodes = create_test_shortcodes();
        let content = r#"---
title: {{ greet name=BlogPost /}}
date: {{ date format=iso /}}
author: {{ greet name=Writer /}}
tags: [{{ simple /}}, {{ greet name=Tutorial /}}]
---

# {{ greet name=Reader /}}!

Welcome to my blog post about {{ simple /}}.

## What we'll cover

1. **{{ greet name=Introduction /}}** - Getting started
2. **{{ simple /}}** basics
3. Advanced {{ greet name=Techniques /}}

{{ alert type=info }}
💡 **Tip**: {{ greet name=Remember /}} to {{ simple /}}!
{{ /alert }}

## Code Example

{{ highlight lang=rust }}
fn main() {
    println!("{{ greet name=World /}}!");
    // {{ simple /}}
}
{{ /highlight }}

### Task List

- [x] {{ greet name=Setup /}}
- [ ] {{ simple /}}
- [ ] {{ greet name=Publish /}}

---

> "{{ greet name=Quote /}}" - {{ simple /}}

{{ section title=Resources }}
- [Documentation](https://docs.rs) - {{ simple /}}
- [GitHub](https://github.com) - {{ greet name=Source /}}
{{ /section }}

*Published on {{ date format=long /}}*"#;

        let processed = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"---
title: Hello, BlogPost!
date: DATE[iso]
author: Hello, Writer!
tags: [SIMPLE_OUTPUT, Hello, Tutorial!]
---

# Hello, Reader!!

Welcome to my blog post about SIMPLE_OUTPUT.

## What we'll cover

1. **Hello, Introduction!** - Getting started
2. **SIMPLE_OUTPUT** basics
3. Advanced Hello, Techniques!

<div class="alert alert-info">
💡 **Tip**: Hello, Remember! to SIMPLE_OUTPUT!
</div>

## Code Example

<code lang="rust">
fn main() {
    println!("Hello, World!!");
    // SIMPLE_OUTPUT
}
</code>

### Task List

- [x] Hello, Setup!
- [ ] SIMPLE_OUTPUT
- [ ] Hello, Publish!

---

> "Hello, Quote!" - SIMPLE_OUTPUT

<section title="Resources">
- [Documentation](https://docs.rs) - SIMPLE_OUTPUT
- [GitHub](https://github.com) - Hello, Source!
</section>

*Published on DATE[long]*"#;
        assert_eq!(processed, expected);
    }

    #[test]
    fn test_invalid_shortcode_names() {
        let shortcodes = create_test_shortcodes();

        // Test invalid names that should be treated as literal text
        let test_cases = vec![
            ("{{ 123invalid /}}", "{{ 123invalid /}}"), // starts with number
            ("{{ -invalid }}", "{{ -invalid }}"),       // starts with dash
            ("{{ invalid-name }}", "{{ invalid-name }}"), // contains dash
            ("{{ invalid.name }}", "{{ invalid.name }}"), // contains dot
            ("{{ invalid@name }}", "{{ invalid@name }}"), // contains special char
        ];

        for (input, expected) in test_cases {
            let result = preprocess_shortcodes_simple(input, &shortcodes).unwrap();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_valid_shortcode_names() {
        let mut shortcodes = create_test_shortcodes();

        // Add shortcodes with valid names
        shortcodes.register("valid_name", |_, _| "VALID_NAME".to_string());
        shortcodes.register("ValidName", |_, _| "VALID_NAME_CAMEL".to_string());
        shortcodes.register("_underscore", |_, _| "UNDERSCORE".to_string());
        shortcodes.register("name123", |_, _| "NAME123".to_string());

        let test_cases = vec![
            ("{{ valid_name /}}", "VALID_NAME"),
            ("{{ ValidName /}}", "VALID_NAME_CAMEL"),
            ("{{ _underscore /}}", "UNDERSCORE"),
            ("{{ name123 /}}", "NAME123"),
            ("{{ a /}}", "{{ a /}}"), // single char is invalid (too short for pattern)
        ];

        for (input, expected) in test_cases {
            let result = preprocess_shortcodes_simple(input, &shortcodes).unwrap();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_escaped_shortcode_syntax() {
        let shortcodes = create_test_shortcodes();

        // Test cases where we encounter \{{
        let test_cases = vec![
            (r#"\{{"hello"}}"#, r#"\{{"hello"}}"#),
            (r#"Before \{{test}} after"#, r#"Before \{{test}} after"#),
            (
                r#"\{{invalid}} and {{ simple /}}"#,
                r#"\{{invalid}} and SIMPLE_OUTPUT"#,
            ),
        ];

        for (input, expected) in test_cases {
            let result = preprocess_shortcodes_simple(input, &shortcodes).unwrap();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_shortcode_name_validation_edge_cases() {
        let shortcodes = create_test_shortcodes();

        // Test various edge cases - invalid names should be treated as literal text
        let test_cases = vec![
            ("{{ 1 /}}", "{{ 1 /}}"),       // single digit (invalid)
            ("{{ _ /}}", "{{ _ /}}"),       // single underscore (invalid - too short)
            ("{{ 1A /}}", "{{ 1A /}}"),     // invalid: digit + letter
            ("{{ café /}}", "{{ café /}}"), // invalid: non-ASCII
        ];

        for (input, expected) in test_cases {
            let result = preprocess_shortcodes_simple(input, &shortcodes).unwrap();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }

        // Test valid names that aren't registered - should get "Unknown shortcode" error
        let valid_but_unregistered = vec![
            "{{ _a /}}",
            "{{ a_ /}}",
            "{{ A1 /}}",
            "{{ valid_name123 /}}",
        ];

        for input in valid_but_unregistered {
            let result = preprocess_shortcodes_simple(input, &shortcodes);
            assert!(
                result.is_err(),
                "Should error for unregistered shortcode: {}",
                input
            );
            assert!(
                result.unwrap_err().contains("Unknown shortcode"),
                "Wrong error type for: {}",
                input
            );
        }

        // Test completely empty shortcode separately since it causes an error
        let empty_result = preprocess_shortcodes_simple("{{ }}", &shortcodes);
        assert!(empty_result.is_err());
        assert!(empty_result.unwrap_err().contains("Empty shortcode"));

        // Test whitespace-only shortcode
        let whitespace_result = preprocess_shortcodes_simple("{{  }}", &shortcodes);
        assert!(whitespace_result.is_err());
        assert!(whitespace_result.unwrap_err().contains("Empty shortcode"));
    }

    #[test]
    fn test_mixed_valid_invalid_shortcodes() {
        let shortcodes = create_test_shortcodes();

        let content = r#"
Valid: {{ simple /}}
Invalid number start: {{ 123invalid /}}
Valid underscore: {{ greet name=Test /}}
Invalid dash: {{ invalid-name }}
Another valid: {{ date format=iso /}}
"#;

        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        let expected = r#"
Valid: SIMPLE_OUTPUT
Invalid number start: {{ 123invalid /}}
Valid underscore: Hello, Test!
Invalid dash: {{ invalid-name }}
Another valid: DATE[iso]
"#;

        assert_eq!(result, expected);
    }

    #[test]
    fn test_quoted_parameter_values() {
        let shortcodes = create_test_shortcodes();

        // Test double quotes
        let content = r#"{{ greet name="Hello World" /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "Hello, Hello World!");

        // Test single quotes
        let content = r#"{{ greet name='Hello World' /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "Hello, Hello World!");
    }

    #[test]
    fn test_mixed_quoted_unquoted_parameters() {
        let mut shortcodes = create_test_shortcodes();
        shortcodes.register("message", |args, _| {
            let text = args.get_str("text").unwrap_or("");
            let author = args.get_str("author").unwrap_or("Anonymous");
            format!("{} - {}", text, author)
        });

        let content = r#"{{ message text="Hello World" author=John /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "Hello World - John");
    }

    #[test]
    fn test_quotes_with_special_characters() {
        let mut shortcodes = create_test_shortcodes();
        shortcodes.register("special", |args, _| {
            args.get_str("value").unwrap_or("").to_string()
        });

        let content = r#"{{ special value="Hello, World! How are you?" /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "Hello, World! How are you?");
    }

    #[test]
    fn test_error_unclosed_quotes() {
        let shortcodes = create_test_shortcodes();

        let content = r#"{{ greet name="Hello World /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unclosed quote"));
    }

    #[test]
    fn test_empty_quoted_values() {
        let mut shortcodes = create_test_shortcodes();
        shortcodes.register("empty", |args, _| {
            let value = args.get_str("value").unwrap_or("default");
            format!("'{}'", value)
        });

        let content = r#"{{ empty value="" /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "''");
    }

    #[test]
    fn test_escaped_quotes_in_arguments() {
        let mut shortcodes = create_test_shortcodes();
        shortcodes.register("quote", |args, _| {
            let message = args.get_str("message").unwrap_or("");
            format!("Quote: {}", message)
        });

        // Test escaped double quotes
        let content = r#"{{ quote message="Me answering \"thanks man glad you like it\" to someone saying a feature is stupid" /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(
            result,
            r#"Quote: Me answering "thanks man glad you like it" to someone saying a feature is stupid"#
        );

        // Test escaped single quotes
        let content = r#"{{ quote message='It\'s a "great" day!' /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, r#"Quote: It's a "great" day!"#);

        // Test escaped backslashes
        let content = r#"{{ quote message="Path: C:\\Users\\name\\file.txt" /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, r#"Quote: Path: C:\Users\name\file.txt"#);

        // Test escaped newlines and tabs
        let content = r#"{{ quote message="Line 1\nLine 2\tTabbed" /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "Quote: Line 1\nLine 2\tTabbed");

        // Test mixed escape sequences
        let content = r#"{{ quote message="Say \"Hello\", then press \\n for newline\nDone!" /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(
            result,
            "Quote: Say \"Hello\", then press \\n for newline\nDone!"
        );
    }

    #[test]
    fn test_self_closing_shortcode_syntax() {
        let mut shortcodes = create_test_shortcodes();
        shortcodes.register("current_date", |_args, _| "2024-01-01".to_string());
        shortcodes.register("user", |args, _| {
            let name = args.get_str("name").unwrap_or("Anonymous");
            format!("User: {}", name)
        });

        // Test basic self-closing shortcode
        let content = r#"Today is {{ current_date /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "Today is 2024-01-01");

        // Test self-closing shortcode with arguments
        let content = r#"{{ user name="Alice" /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "User: Alice");

        // Test self-closing shortcode with spaces before /
        let content = r#"{{ user name="Bob" /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "User: Bob");

        // Test multiple self-closing shortcodes
        let content = r#"{{ user name="Alice" /}} and {{ user name="Bob" /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, "User: Alice and User: Bob");
    }

    #[test]
    fn test_block_shortcode_requires_closing_tag() {
        let shortcodes = create_test_shortcodes();

        // This should now be an error because it's not self-closing and has no closing tag
        let content = r#"{{ highlight lang="rust" }}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        });
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("missing its closing tag"));
        assert!(error_msg.contains("Use '{{ highlight /}}' for self-closing"));
    }

    #[test]
    fn test_ambiguous_shortcode_resolution() {
        let mut shortcodes = create_test_shortcodes();
        shortcodes.register("img", |args, _| {
            let src = args.get_str("src").unwrap_or("");
            format!("<img src=\"{}\">", src)
        });

        // This scenario would have been ambiguous in the old syntax:
        // Two shortcodes where the first could mistakenly consume the second as body

        // Using new self-closing syntax - should work correctly
        let content = r#"{{ img src="photo.jpg" /}} {{ img src="photo2.jpg" /}}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert_eq!(result, r#"<img src="photo.jpg"> <img src="photo2.jpg">"#);

        // Block shortcode with proper closing tags should still work
        let content = r#"{{ highlight lang="rust" }}
let x = 5;
{{ /highlight }}

{{ highlight lang="js" }}
const y = 10;
{{ /highlight }}"#;
        let result = with_test_route_context(|route_ctx| {
            preprocess_shortcodes_with_ctx(content, &shortcodes, route_ctx)
        })
        .unwrap();
        assert!(result.contains(r#"<code lang="rust">"#));
        assert!(result.contains("let x = 5;"));
        assert!(result.contains(r#"<code lang="js">"#));
        assert!(result.contains("const y = 10;"));
    }
}
