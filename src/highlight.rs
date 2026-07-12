use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SynStyle, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Highlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set.themes["base16-ocean.dark"].clone();
        Self { syntax_set, theme }
    }

    pub fn syntax_for_path<'a>(&'a self, path: &str) -> &'a SyntaxReference {
        self.syntax_set
            .find_syntax_for_file(path)
            .ok()
            .flatten()
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }

    pub fn line_highlighter<'a>(&'a self, syntax: &'a SyntaxReference) -> HighlightLines<'a> {
        HighlightLines::new(syntax, &self.theme)
    }

    /// Highlights a single line of source (no trailing newline) into ratatui spans.
    pub fn highlight(&self, highlighter: &mut HighlightLines, content: &str) -> Vec<Span<'static>> {
        // syntect expects a trailing newline for correct state tracking.
        let mut owned = String::with_capacity(content.len() + 1);
        owned.push_str(content);
        owned.push('\n');

        let ranges: Vec<(SynStyle, &str)> = highlighter
            .highlight_line(&owned, &self.syntax_set)
            .unwrap_or_default();

        ranges
            .into_iter()
            .map(|(style, text)| {
                let text = text.trim_end_matches('\n');
                Span::styled(text.to_string(), syn_style_to_ratatui(style))
            })
            .collect()
    }
}

fn syn_style_to_ratatui(style: SynStyle) -> Style {
    let fg = style.foreground;
    let mut s = Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b));
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::BOLD)
    {
        s = s.add_modifier(Modifier::BOLD);
    }
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::ITALIC)
    {
        s = s.add_modifier(Modifier::ITALIC);
    }
    s
}
