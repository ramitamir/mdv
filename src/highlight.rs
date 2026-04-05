use crate::graphics::StyledTextSpan;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    syntax_theme: String,
}

impl Highlighter {
    pub fn new(syntax_theme: &str) -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            syntax_theme: syntax_theme.to_string(),
        }
    }

    /// Highlight code and return styled spans for pixel rendering via cosmic-text.
    pub fn highlight_to_styled_spans(&self, code: &str, lang: &str) -> Vec<StyledTextSpan> {
        let mut hl = match self.create_highlighter(lang) {
            Some(h) => h,
            None => {
                return vec![StyledTextSpan {
                    text: code.to_string(),
                    color: [200, 200, 200],
                    bold: false,
                    italic: false,
                }];
            }
        };

        let mut result = Vec::new();
        for (i, line) in code.lines().enumerate() {
            if i > 0 {
                result.push(StyledTextSpan {
                    text: "\n".to_string(),
                    color: [200, 200, 200],
                    bold: false,
                    italic: false,
                });
            }
            let ops = hl.highlight_line(line, &self.syntax_set).unwrap_or_default();
            for (style, text) in ops {
                result.push(StyledTextSpan {
                    text: text.to_string(),
                    color: [style.foreground.r, style.foreground.g, style.foreground.b],
                    bold: style
                        .font_style
                        .contains(syntect::highlighting::FontStyle::BOLD),
                    italic: style
                        .font_style
                        .contains(syntect::highlighting::FontStyle::ITALIC),
                });
            }
        }
        result
    }

    pub fn create_highlighter(&self, lang: &str) -> Option<HighlightLines<'_>> {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(lang)
            .or_else(|| self.syntax_set.find_syntax_by_extension(lang))?;
        let theme = &self.theme_set.themes[&self.syntax_theme];
        Some(HighlightLines::new(syntax, theme))
    }
}
