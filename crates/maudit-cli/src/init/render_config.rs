use inquire::ui::{Attributes, Color, RenderConfig, StyleSheet, Styled};

pub fn get_render_config() -> RenderConfig<'static> {
    RenderConfig {
        prompt: StyleSheet::new().with_attr(Attributes::BOLD),
        prompt_prefix: Styled::new("○ ").with_fg(Color::LightCyan),
        text_input: StyleSheet::new().with_fg(Color::LightCyan),
        answer: StyleSheet::new().with_fg(Color::LightGreen),
        answered_prompt_prefix: Styled::new("● ").with_fg(Color::LightGreen),
        ..Default::default()
    }
}
