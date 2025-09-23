use core::panic;
use std::sync::OnceLock;
use syntect::{
    Error,
    easy::HighlightLines,
    highlighting::ThemeSet,
    html::{IncludeBackground, styled_line_to_highlighted_html},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn get_theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

fn opening_html(language: Option<&str>) -> String {
    let mut attrs = Vec::new();

    // Follow EC here on the attribute name, though EC only adds it to the pre tag. I figure there's no harm in adding it to the code tag too.
    if let Some(lang) = language {
        attrs.push((String::from("data-language"), format!("\"{lang}\"")));
    }

    let format_attrs = |attrs: &[(String, String)]| {
        if attrs.is_empty() {
            String::new()
        } else {
            let attrs_str = attrs
                .iter()
                .map(|(name, value)| format!("{}={}", name, value))
                .collect::<Vec<_>>()
                .join(" ");
            format!(" {}", attrs_str)
        }
    };

    let pre_attrs_str = format_attrs(&attrs);
    let code_attrs_str = format_attrs(&attrs);

    format!("<pre{pre_attrs_str}><code{code_attrs_str}>")
}

pub struct CodeBlockMeta {
    pub language: String,
}

impl CodeBlockMeta {
    pub fn new_from_string(fence: &str) -> Self {
        // Parse the value after the opening of a fenced code block
        // e.g. for ```rs ins=0, you'd get lang: "rs", ins: "0"

        // TODO: Write the parser for this, lol
        let language = fence.to_string();
        Self { language }
    }
}

pub struct CodeBlock {
    pub meta: CodeBlockMeta,
}

impl CodeBlock {
    pub fn new(fence: &str) -> (Self, String) {
        let meta = CodeBlockMeta::new_from_string(fence);
        let opening_html = opening_html(Some(&meta.language));

        (Self { meta }, opening_html)
    }

    pub fn highlight(&self, content: &str, theme_path: &str) -> Result<String, Error> {
        let ss = get_syntax_set();
        let ts = get_theme_set();

        let syntax = ss
            .find_syntax_by_token(&self.meta.language)
            // Maybe token is enough, looking around at other users of Syntect, it seems like they often just use by_token, not sure.
            .or_else(|| ss.find_syntax_by_name(&self.meta.language))
            .or_else(|| ss.find_syntax_by_extension(&self.meta.language))
            .or_else(|| ss.find_syntax_by_first_line(content))
            .unwrap_or_else(|| ss.find_syntax_plain_text());

        let theme = match ts.themes.get(theme_path) {
            Some(theme) => theme,
            None => &match ThemeSet::get_theme(theme_path) {
                Ok(theme) => theme,
                Err(_) => panic!(
                    "Theme '{theme_path}' not found in default themes and could not be loaded from file."
                ),
            },
        };

        let mut h = HighlightLines::new(syntax, theme);

        let mut highlighted = String::new();
        for line in LinesWithEndings::from(content) {
            let regions = h.highlight_line(line, ss)?;
            let html = styled_line_to_highlighted_html(&regions, IncludeBackground::No)?; // TODO: Handle the background coloring
            highlighted.push_str(&html);
        }

        Ok(highlighted)
    }
}
