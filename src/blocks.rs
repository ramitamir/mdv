/// Inline text style flags
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
}

/// A span of styled inline text
#[derive(Debug, Clone)]
pub struct StyledSpan {
    pub text: String,
    pub style: InlineStyle,
    pub link_url: Option<String>,
}

/// A single list item, which contains inline spans and optional nested blocks
#[derive(Debug, Clone)]
pub struct ListItem {
    pub spans: Vec<StyledSpan>,
    pub children: Vec<Block>,
}

/// Heading level (1-6)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadingLevel {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl HeadingLevel {
    pub fn from_pulldown(level: pulldown_cmark::HeadingLevel) -> Self {
        match level {
            pulldown_cmark::HeadingLevel::H1 => Self::H1,
            pulldown_cmark::HeadingLevel::H2 => Self::H2,
            pulldown_cmark::HeadingLevel::H3 => Self::H3,
            pulldown_cmark::HeadingLevel::H4 => Self::H4,
            pulldown_cmark::HeadingLevel::H5 => Self::H5,
            pulldown_cmark::HeadingLevel::H6 => Self::H6,
        }
    }

    /// Returns font size as a multiplier of the base terminal font size
    pub fn font_scale(&self) -> f32 {
        match self {
            Self::H1 => 1.8,
            Self::H2 => 1.5,
            Self::H3 => 1.3,
            Self::H4 => 1.1,
            Self::H5 => 1.0,
            Self::H6 => 1.0,
        }
    }
}

/// A parsed markdown block element
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Block {
    Heading {
        level: HeadingLevel,
        spans: Vec<StyledSpan>,
        id: Option<String>,
    },
    Paragraph {
        spans: Vec<StyledSpan>,
    },
    CodeBlock {
        lang: Option<String>,
        code: String,
    },
    Table {
        headers: Vec<Vec<StyledSpan>>,
        rows: Vec<Vec<Vec<StyledSpan>>>,
    },
    BlockQuote {
        blocks: Vec<Block>,
    },
    List {
        ordered: bool,
        start: Option<u64>,
        items: Vec<ListItem>,
    },
    Image {
        alt: String,
        url: String,
    },
    ThematicBreak,
}

impl Block {
    pub fn text_content(&self) -> String {
        match self {
            Block::Heading { spans, .. } => spans.iter().map(|s| s.text.as_str()).collect(),
            Block::Paragraph { spans } => spans.iter().map(|s| s.text.as_str()).collect(),
            Block::CodeBlock { code, .. } => code.clone(),
            Block::List { items, .. } => {
                items.iter().map(|item| {
                    let mut text: String = item.spans.iter().map(|s| s.text.as_str()).collect();
                    for child in &item.children {
                        text.push('\n');
                        text.push_str(&child.text_content());
                    }
                    text
                }).collect::<Vec<_>>().join("\n")
            }
            Block::BlockQuote { blocks } => {
                blocks.iter().map(|b| b.text_content()).collect::<Vec<_>>().join("\n")
            }
            Block::Table { headers, rows } => {
                let h: String = headers.iter().map(|cell| {
                    cell.iter().map(|s| s.text.as_str()).collect::<String>()
                }).collect::<Vec<_>>().join(" ");
                let r: String = rows.iter().map(|row| {
                    row.iter().map(|cell| {
                        cell.iter().map(|s| s.text.as_str()).collect::<String>()
                    }).collect::<Vec<_>>().join(" ")
                }).collect::<Vec<_>>().join("\n");
                format!("{}\n{}", h, r)
            }
            Block::Image { alt, .. } => alt.clone(),
            Block::ThematicBreak => String::new(),
        }
    }
}
