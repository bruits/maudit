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
}

pub fn preprocess_shortcodes(
    content: &str,
    shortcodes: &MarkdownShortcodes,
) -> Result<String, String> {
    let mut output = String::new();
    let mut rest = content;

    while let Some(start) = rest.find("{{") {
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
}
