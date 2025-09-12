use rustc_hash::FxHashMap;
use std::str::FromStr;

pub type ShortcodeFn = Box<dyn Fn(&ShortcodeArgs) -> String + Send + Sync>;

#[derive(Default)]
pub struct MarkdownShortcodes(FxHashMap<String, ShortcodeFn>);

impl MarkdownShortcodes {
    pub fn new() -> Self {
        Self(FxHashMap::default())
    }

    pub fn register<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&ShortcodeArgs) -> String + Send + Sync + 'static,
    {
        self.0.insert(name.to_string(), Box::new(func));
    }

    pub(crate) fn get(&self, name: &str) -> Option<&ShortcodeFn> {
        self.0.get(name)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// Helper function to validate shortcode names
// Valid names match ^[A-Za-z_][0-9A-Za-z_]+$ pattern
fn is_valid_shortcode_name(name: &str) -> bool {
    if name.len() < 2 {
        return false; // Must have at least 2 characters
    }

    let mut chars = name.chars();

    // First character must be A-Z, a-z, or _
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }

    // Remaining characters must be A-Z, a-z, 0-9, or _
    for ch in chars {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            return false;
        }
    }

    true
}

pub fn preprocess_shortcodes(
    content: &str,
    shortcodes: &MarkdownShortcodes,
) -> Result<String, String> {
    let mut output = String::new();
    let mut rest = content;

    while let Some(start) = rest.find("{{") {
        // Check for escaped shortcode syntax like `\{{` - if found, skip this occurrence
        if start > 0 && rest.chars().nth(start - 1) == Some('\\') {
            // This is an escaped shortcode, add everything up to and including the {{
            output.push_str(&rest[..start + 2]);
            rest = &rest[start + 2..];
            continue;
        }

        // Add everything before the shortcode
        output.push_str(&rest[..start]);

        // Find the end of the opening shortcode tag
        let remaining = &rest[start + 2..];
        let tag_end = remaining
            .find("}}")
            .ok_or("Unclosed shortcode: missing '}}'")?;

        let shortcode_content = remaining[..tag_end].trim();

        // Parse shortcode name and arguments
        let mut parts = shortcode_content.split_whitespace();
        let name = parts.next().ok_or("Empty shortcode")?;

        // Check if this is a closing tag
        if name.starts_with('/') {
            return Err(format!("Unexpected closing tag: {}", name));
        }

        // Validate shortcode name format
        let actual_name = name.strip_prefix('/').unwrap_or(name);

        if !is_valid_shortcode_name(actual_name) {
            // Invalid shortcode name, treat as literal text and continue
            output.push_str("{{");
            rest = remaining;
            continue;
        }

        // Parse arguments
        let mut args = FxHashMap::default();
        for part in parts {
            if let Some(eq_pos) = part.find('=') {
                let key = part[..eq_pos].trim();
                let value = part[eq_pos + 1..].trim();
                args.insert(key.to_string(), value.to_string());
            } else {
                return Err(format!(
                    "Invalid argument format: '{}'. Expected 'key=value'",
                    part
                ));
            }
        }

        // Move past the opening tag
        let after_opening_tag = &remaining[tag_end + 2..];

        // Look for closing tag - handle both {{/name}} and {{ /name }} formats
        let closing_tag_compact = format!("{{{{/{}}}}}", name);
        let closing_tag_spaced = format!("{{{{ /{} }}}}", name);

        let close_pos = after_opening_tag
            .find(&closing_tag_compact)
            .or_else(|| after_opening_tag.find(&closing_tag_spaced));

        if let Some(close_pos) = close_pos {
            // Determine which closing tag format was found to calculate the correct length
            let closing_tag_len =
                if after_opening_tag[close_pos..].starts_with(&closing_tag_compact) {
                    closing_tag_compact.len()
                } else {
                    closing_tag_spaced.len()
                };
            // Block shortcode - extract body and recursively process it
            let body = &after_opening_tag[..close_pos];
            let processed_body = preprocess_shortcodes(body, shortcodes)?; // <- RECURSIVE CALL

            // Execute shortcode with processed body
            if let Some(func) = shortcodes.get(name) {
                let mut shortcode_args = ShortcodeArgs::new(args);
                shortcode_args.0.insert("body".to_string(), processed_body);
                let result = func(&shortcode_args);
                output.push_str(&result);
            } else {
                return Err(format!("Unknown shortcode: '{}'", name));
            }

            // Continue after the closing tag
            rest = &after_opening_tag[close_pos + closing_tag_len..];
        } else {
            // Self-closing shortcode
            if let Some(func) = shortcodes.get(name) {
                let shortcode_args = ShortcodeArgs::new(args);
                let result = func(&shortcode_args);
                output.push_str(&result);
            } else {
                return Err(format!("Unknown shortcode: '{}'", name));
            }

            // Continue after the opening tag
            rest = after_opening_tag;
        }
    }

    output.push_str(rest);
    Ok(output)
}

pub struct ShortcodeArgs(FxHashMap<String, String>);

impl ShortcodeArgs {
    pub fn new(args: FxHashMap<String, String>) -> Self {
        Self(args)
    }

    /// Get argument with automatic type conversion
    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: FromStr,
        T::Err: std::fmt::Debug,
    {
        self.0.get(key)?.parse().ok()
    }

    /// Get required argument with automatic type conversion
    pub fn get_required<T>(&self, key: &str) -> T
    where
        T: FromStr,
        T::Err: std::fmt::Debug,
    {
        self.0
            .get(key)
            .unwrap_or_else(|| panic!("Required argument '{}' not found", key))
            .parse()
            .unwrap_or_else(|e| panic!("Failed to parse argument '{}': {:?}", key, e))
    }

    /// Get argument with default value and type conversion
    pub fn get_or<T>(&self, key: &str, default: T) -> T
    where
        T: FromStr,
        T::Err: std::fmt::Debug,
    {
        self.0
            .get(key)
            .and_then(|s| s.parse().ok())
            .unwrap_or(default)
    }

    /// Get raw string (no conversion)
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }

    pub fn get_str_required(&self, key: &str) -> &str {
        self.0
            .get(key)
            .map(|s| s.as_str())
            .unwrap_or_else(|| panic!("Required argument '{}' not found", key))
    }

    pub fn get_str_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.0.get(key).map(|s| s.as_str()).unwrap_or(default)
    }
}

// Macro to make typed shortcodes easier to write
#[macro_export]
macro_rules! shortcode {
    ($args:ident, $($param:ident: $type:ty),* => $body:expr) => {
        |$args: &ShortcodeArgs| -> String {
            $(
                let $param: $type = $args.get_required(stringify!($param));
            )*
            $body
        }
    };
    ($args:ident, $($param:ident: $type:ty = $default:expr),* => $body:expr) => {
        |$args: &ShortcodeArgs| -> String {
            $(
                let $param: $type = $args.get_or(stringify!($param), $default);
            )*
            $body
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_shortcodes() -> MarkdownShortcodes {
        let mut shortcodes = MarkdownShortcodes::new();

        // Simple shortcode that just returns its name
        shortcodes.register("simple", |_args| "SIMPLE_OUTPUT".to_string());

        // Shortcode with arguments
        shortcodes.register("greet", |args| {
            let name = args.get_str("name").unwrap_or("World");
            format!("Hello, {}!", name)
        });

        // Date shortcode with format
        shortcodes.register("date", |args| {
            let format = args.get_str("format").unwrap_or("default");
            format!("DATE[{}]", format)
        });

        // Block shortcode that wraps content
        shortcodes.register("highlight", |args| {
            let lang = args.get_str("lang").unwrap_or("text");
            let body = args.get_str("body").unwrap_or("");
            format!("<code lang=\"{}\">{}</code>", lang, body)
        });

        // Section shortcode for testing nested content
        shortcodes.register("section", |args| {
            let title = args.get_str("title").unwrap_or("");
            let body = args.get_str("body").unwrap_or("");
            if title.is_empty() {
                format!("<section>{}</section>", body)
            } else {
                format!("<section title=\"{}\">{}</section>", title, body)
            }
        });

        // Alert shortcode with type
        shortcodes.register("alert", |args| {
            let alert_type = args.get_str("type").unwrap_or("info");
            let body = args.get_str("body").unwrap_or("");
            format!("<div class=\"alert alert-{}\">{}</div>", alert_type, body)
        });

        shortcodes
    }

    #[test]
    fn test_no_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "This is just plain text with no shortcodes.";
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(result, content);
    }

    #[test]
    fn test_simple_self_closing_shortcode() {
        let shortcodes = create_test_shortcodes();
        let content = "Before {{ simple }} after";
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(result, "Before SIMPLE_OUTPUT after");
    }

    #[test]
    fn test_shortcode_with_arguments() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ greet name=Alice }}";
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(result, "Hello, Alice!");
    }

    #[test]
    fn test_multiple_arguments() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ date format=iso year=2023 }}";
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(result, "DATE[iso]");
    }

    #[test]
    fn test_frontmatter_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"---
title: {{ greet name=Blog }}
date: {{ date format=iso }}
---

# Content here"#;
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
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
        let content = "# {{ greet name=Header }}\n\n## Section {{ date format=short }}";
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(result, "# Hello, Header!\n\n## Section DATE[short]");
    }

    #[test]
    fn test_shortcodes_in_links() {
        let shortcodes = create_test_shortcodes();
        let content = "[{{ greet name=Link }}](https://example.com)";
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(result, "[Hello, Link!](https://example.com)");
    }

    #[test]
    fn test_shortcodes_in_code_blocks() {
        let shortcodes = create_test_shortcodes();
        let content = "```\nSome code with {{ simple }}\n```";
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(result, "```\nSome code with SIMPLE_OUTPUT\n```");
    }

    #[test]
    fn test_block_shortcode() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ highlight lang=rust }}\nlet x = 5;\n{{ /highlight }}";
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(result, "<code lang=\"rust\">\nlet x = 5;\n</code>");
    }

    #[test]
    fn test_nested_shortcodes_in_block() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ section title=Main }}\nHello {{ greet name=World }}!\n{{ /section }}";
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
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
console.log("{{ greet name=Nested }}");
{{ /highlight }}
{{ /alert }}
{{ /section }}"#;
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
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
        let content = "{{ greet name=Alice }} and {{ greet name=Bob }}";
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(result, "Hello, Alice! and Hello, Bob!");
    }

    #[test]
    fn test_shortcodes_in_lists() {
        let shortcodes = create_test_shortcodes();
        let content = r#"- Item 1: {{ greet name=First }}
- Item 2: {{ date format=short }}
- Item 3: {{ simple }}"#;
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
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
| Alice | {{ greet name=Alice }} |
| Bob | {{ greet name=Bob }} |"#;
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        let expected = r#"| Name | Greeting |
|------|----------|
| Alice | Hello, Alice! |
| Bob | Hello, Bob! |"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_shortcodes_with_special_characters() {
        let shortcodes = create_test_shortcodes();
        let content = "Before\n{{ simple }}\nAfter\n\n{{ greet name=Test }}";
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(result, "Before\nSIMPLE_OUTPUT\nAfter\n\nHello, Test!");
    }

    #[test]
    fn test_error_unknown_shortcode() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ unknown_shortcode }}";
        let result = preprocess_shortcodes(content, &shortcodes);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Unknown shortcode: 'unknown_shortcode'")
        );
    }

    #[test]
    fn test_error_unclosed_shortcode() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ simple ";
        let result = preprocess_shortcodes(content, &shortcodes);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Unclosed shortcode: missing '}}'")
        );
    }

    #[test]
    fn test_error_empty_shortcode() {
        let shortcodes = create_test_shortcodes();
        let content = "{{  }}";
        let result = preprocess_shortcodes(content, &shortcodes);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty shortcode"));
    }

    #[test]
    fn test_error_invalid_argument_format() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ greet name Alice }}";
        let result = preprocess_shortcodes(content, &shortcodes);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid argument format"));
    }

    #[test]
    fn test_error_unexpected_closing_tag() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ /section }}";
        let result = preprocess_shortcodes(content, &shortcodes);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unexpected closing tag"));
    }

    #[test]
    fn test_whitespace_handling() {
        let shortcodes = create_test_shortcodes();
        let content = "{{   simple   }}";
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(result, "SIMPLE_OUTPUT");
    }

    #[test]
    fn test_whitespace_in_arguments() {
        let shortcodes = create_test_shortcodes();
        let content = "{{  greet   name=Alice  }}";
        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(result, "Hello, Alice!");
    }

    #[test]
    fn test_complex_markdown_document() {
        let shortcodes = create_test_shortcodes();
        let content = r#"---
title: {{ greet name=Blog }}
author: {{ greet name=Author }}
---

# {{ greet name=Reader }}

Welcome to my blog! Today is {{ date format=full }}.

## Code Example

{{ highlight lang=rust }}
fn main() {
    println!("{{ greet name=Rust }}");
}
{{ /highlight }}

## Alert Section

{{ alert type=info }}
This is an important message with {{ simple }} content.
{{ /alert }}

- List item with {{ greet name=Item }}
- Another item: {{ date format=short }}

> Quote with {{ simple }} shortcode

[Link with {{ greet name=Link }}](http://example.com)"#;

        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
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
        let content = "# {{ greet name=Title }}\n\n## Section {{ date format=short }}";

        // Test shortcode preprocessing first
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(processed, "# Hello, Title!\n\n## Section DATE[short]");
    }

    #[test]
    fn test_markdown_integration_emphasis_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "*{{ greet name=Italic }}* and **{{ greet name=Bold }}**";
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(processed, "*Hello, Italic!* and **Hello, Bold!**");
    }

    #[test]
    fn test_markdown_integration_code_spans_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "Use `{{ simple }}` in your code";
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(processed, "Use `SIMPLE_OUTPUT` in your code");
    }

    #[test]
    fn test_markdown_integration_blockquotes_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "> {{ greet name=Quote }}\n> \n> {{ simple }}";
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(processed, "> Hello, Quote!\n> \n> SIMPLE_OUTPUT");
    }

    #[test]
    fn test_markdown_integration_nested_lists_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"1. {{ greet name=First }}
   - Nested {{ simple }}
   - {{ date format=iso }}
2. {{ greet name=Second }}
   1. Numbered {{ simple }}
   2. {{ greet name=Nested }}"#;
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
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
        let content = r#"| **{{ greet name=Header }}** | _{{ date format=long }}_ |
|:---------------------------|-------------------------:|
| {{ simple }}               | {{ greet name=Cell }}     |
| `{{ greet name=Code }}`    | > {{ simple }}            |"#;
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
        let expected = r#"| **Hello, Header!** | _DATE[long]_ |
|:---------------------------|-------------------------:|
| SIMPLE_OUTPUT               | Hello, Cell!     |
| `Hello, Code!`    | > SIMPLE_OUTPUT            |"#;
        assert_eq!(processed, expected);
    }

    #[test]
    fn test_markdown_integration_task_lists_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"- [x] {{ greet name=Done }}
- [ ] {{ simple }}
- [ ] {{ date format=todo }}"#;
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
        let expected = r#"- [x] Hello, Done!
- [ ] SIMPLE_OUTPUT
- [ ] DATE[todo]"#;
        assert_eq!(processed, expected);
    }

    #[test]
    fn test_markdown_integration_strikethrough_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "~~{{ greet name=Deleted }}~~ and {{ simple }}";
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(processed, "~~Hello, Deleted!~~ and SIMPLE_OUTPUT");
    }

    #[test]
    fn test_markdown_integration_horizontal_rules_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ greet name=Before }}\n\n---\n\n{{ simple }}";
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(processed, "Hello, Before!\n\n---\n\nSIMPLE_OUTPUT");
    }

    #[test]
    fn test_markdown_integration_footnotes_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = "{{ greet name=Text }}[^1]\n\n[^1]: {{ simple }}";
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
        assert_eq!(processed, "Hello, Text![^1]\n\n[^1]: SIMPLE_OUTPUT");
    }

    #[test]
    fn test_markdown_integration_complex_links_with_shortcodes() {
        let shortcodes = create_test_shortcodes();
        let content = r#"[{{ greet name=Link }}](https://example.com "{{ simple }}")

![{{ greet name=Alt }}](image.jpg "{{ date format=title }}")

[Reference {{ simple }}][ref]

[ref]: https://example.com "{{ greet name=RefTitle }}""#;
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
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
    println!("{{ greet name=Rust }}");
    // {{ simple }}
}
```

```{{ greet name=Language }}
{{ simple }}
```"#;
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
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
  <h2>{{ greet name=HTML }}</h2>
  <p>{{ simple }}</p>
</div>

<img src="test.jpg" alt="{{ greet name=Alt }}" title="{{ date format=attr }}">"#;
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
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
        let content = r#"Inline math: ${{ simple }}$

Block math:
$$
{{ greet name=Math }}
$$

{{ greet name=Text }} with $x = {{ simple }}$ inline."#;
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
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
title: "{{ greet name=Blog }}"
description: {{ simple }}
tags:
  - {{ greet name=Tag1 }}
  - {{ simple }}
metadata:
  created: {{ date format=iso }}
  author: {{ greet name=Author }}
---

# {{ greet name=Content }}

{{ simple }}"#;
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
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
# {{ greet name=Header }}

**{{ greet name=Bold }}** and *{{ simple }}*

- {{ greet name=Item1 }}
- {{ simple }}

> {{ greet name=Quote }}

{{ /section }}"#;
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
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
        let content = r#"{{ greet name=Start }}

<!-- {{ simple }} in comment -->

{{ highlight lang=markdown }}
# {{ greet name=NestedMD }}
{{ simple }}
{{ /highlight }}

`{{ greet name=BacktickCode }}`

{{ greet name=End }}"#;
        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
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
title: {{ greet name=BlogPost }}
date: {{ date format=iso }}
author: {{ greet name=Writer }}
tags: [{{ simple }}, {{ greet name=Tutorial }}]
---

# {{ greet name=Reader }}!

Welcome to my blog post about {{ simple }}.

## What we'll cover

1. **{{ greet name=Introduction }}** - Getting started
2. **{{ simple }}** basics
3. Advanced {{ greet name=Techniques }}

{{ alert type=info }}
💡 **Tip**: {{ greet name=Remember }} to {{ simple }}!
{{ /alert }}

## Code Example

{{ highlight lang=rust }}
fn main() {
    println!("{{ greet name=World }}!");
    // {{ simple }}
}
{{ /highlight }}

### Task List

- [x] {{ greet name=Setup }}
- [ ] {{ simple }}
- [ ] {{ greet name=Publish }}

---

> "{{ greet name=Quote }}" - {{ simple }}

{{ section title=Resources }}
- [Documentation](https://docs.rs) - {{ simple }}
- [GitHub](https://github.com) - {{ greet name=Source }}
{{ /section }}

*Published on {{ date format=long }}*"#;

        let processed = preprocess_shortcodes(content, &shortcodes).unwrap();
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
            ("{{ 123invalid }}", "{{ 123invalid }}"), // starts with number
            ("{{ -invalid }}", "{{ -invalid }}"),     // starts with dash
            ("{{ invalid-name }}", "{{ invalid-name }}"), // contains dash
            ("{{ invalid.name }}", "{{ invalid.name }}"), // contains dot
            ("{{ invalid@name }}", "{{ invalid@name }}"), // contains special char
        ];

        for (input, expected) in test_cases {
            let result = preprocess_shortcodes(input, &shortcodes).unwrap();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_valid_shortcode_names() {
        let mut shortcodes = create_test_shortcodes();

        // Add shortcodes with valid names
        shortcodes.register("valid_name", |_| "VALID_NAME".to_string());
        shortcodes.register("ValidName", |_| "VALID_NAME_CAMEL".to_string());
        shortcodes.register("_underscore", |_| "UNDERSCORE".to_string());
        shortcodes.register("name123", |_| "NAME123".to_string());

        let test_cases = vec![
            ("{{ valid_name }}", "VALID_NAME"),
            ("{{ ValidName }}", "VALID_NAME_CAMEL"),
            ("{{ _underscore }}", "UNDERSCORE"),
            ("{{ name123 }}", "NAME123"),
            ("{{ a }}", "{{ a }}"), // single char is invalid (too short for pattern)
        ];

        for (input, expected) in test_cases {
            let result = preprocess_shortcodes(input, &shortcodes).unwrap();
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
                r#"\{{invalid}} and {{ simple }}"#,
                r#"\{{invalid}} and SIMPLE_OUTPUT"#,
            ),
        ];

        for (input, expected) in test_cases {
            let result = preprocess_shortcodes(input, &shortcodes).unwrap();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_shortcode_name_validation_edge_cases() {
        let shortcodes = create_test_shortcodes();

        // Test various edge cases - invalid names should be treated as literal text
        let test_cases = vec![
            ("{{ 1 }}", "{{ 1 }}"),       // single digit (invalid)
            ("{{ _ }}", "{{ _ }}"),       // single underscore (invalid - too short)
            ("{{ 1A }}", "{{ 1A }}"),     // invalid: digit + letter
            ("{{ café }}", "{{ café }}"), // invalid: non-ASCII
        ];

        for (input, expected) in test_cases {
            let result = preprocess_shortcodes(input, &shortcodes).unwrap();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }

        // Test valid names that aren't registered - should get "Unknown shortcode" error
        let valid_but_unregistered =
            vec!["{{ _a }}", "{{ a_ }}", "{{ A1 }}", "{{ valid_name123 }}"];

        for input in valid_but_unregistered {
            let result = preprocess_shortcodes(input, &shortcodes);
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
        let empty_result = preprocess_shortcodes("{{ }}", &shortcodes);
        assert!(empty_result.is_err());
        assert!(empty_result.unwrap_err().contains("Empty shortcode"));

        // Test whitespace-only shortcode
        let whitespace_result = preprocess_shortcodes("{{  }}", &shortcodes);
        assert!(whitespace_result.is_err());
        assert!(whitespace_result.unwrap_err().contains("Empty shortcode"));
    }

    #[test]
    fn test_mixed_valid_invalid_shortcodes() {
        let shortcodes = create_test_shortcodes();

        let content = r#"
Valid: {{ simple }}
Invalid number start: {{ 123invalid }}
Valid underscore: {{ greet name=Test }}
Invalid dash: {{ invalid-name }}
Another valid: {{ date format=iso }}
"#;

        let result = preprocess_shortcodes(content, &shortcodes).unwrap();
        let expected = r#"
Valid: SIMPLE_OUTPUT
Invalid number start: {{ 123invalid }}
Valid underscore: Hello, Test!
Invalid dash: {{ invalid-name }}
Another valid: DATE[iso]
"#;

        assert_eq!(result, expected);
    }
}
