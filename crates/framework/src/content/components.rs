use super::MarkdownHeading;

#[derive(Default)]
pub struct MarkdownComponents {
    pub a: Option<Box<dyn Fn(&str) -> String>>,
    pub blockquote: Option<Box<dyn Fn(&str) -> String>>,
    pub br: Option<Box<dyn Fn(&str) -> String>>,
    pub code: Option<Box<dyn Fn(&str) -> String>>,
    pub em: Option<Box<dyn Fn(&str) -> String>>,
    pub heading: Option<Box<dyn Fn(MarkdownHeading) -> String>>,
    pub hr: Option<Box<dyn Fn(&str) -> String>>,
    pub img: Option<Box<dyn Fn(&str) -> String>>,
    pub li: Option<Box<dyn Fn(&str) -> String>>,
    pub ol: Option<Box<dyn Fn(&str) -> String>>,
    pub p: Option<Box<dyn Fn(&str) -> String>>,
    pub pre: Option<Box<dyn Fn(&str) -> String>>,
    pub strong: Option<Box<dyn Fn(&str) -> String>>,
    pub ul: Option<Box<dyn Fn(&str) -> String>>,
    pub del: Option<Box<dyn Fn(&str) -> String>>,
    pub input: Option<Box<dyn Fn(&str) -> String>>,
    pub section: Option<Box<dyn Fn(&str) -> String>>,
    pub sup: Option<Box<dyn Fn(&str) -> String>>,
    pub table: Option<Box<dyn Fn(&str) -> String>>,
    pub tbody: Option<Box<dyn Fn(&str) -> String>>,
    pub td: Option<Box<dyn Fn(&str) -> String>>,
    pub th: Option<Box<dyn Fn(&str) -> String>>,
    pub thead: Option<Box<dyn Fn(&str) -> String>>,
    pub tr: Option<Box<dyn Fn(&str) -> String>>,
}
