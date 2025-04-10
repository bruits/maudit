use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    html::{styled_line_to_highlighted_html, IncludeBackground},
    parsing::SyntaxSet,
    util::LinesWithEndings,
    Error,
};

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

    pub fn highlight(&self, content: &str) -> Result<String, Error> {
        // TODO: Re-use the syntax set and everything else
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let syntax = ss
            .find_syntax_by_name(&self.meta.language)
            .or_else(|| ss.find_syntax_by_extension(&self.meta.language))
            .or_else(|| ss.find_syntax_by_first_line(content))
            .unwrap_or_else(|| ss.find_syntax_plain_text());

        // TODO: Allow configuring the theme
        let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

        let mut highlighted = String::new();
        for line in LinesWithEndings::from(content) {
            let regions = h.highlight_line(line, &ss).unwrap();
            let html = styled_line_to_highlighted_html(&regions, IncludeBackground::No)?; // TODO: Handle the background coloring
            highlighted.push_str(&html);
        }

        Ok(highlighted)
    }
}
