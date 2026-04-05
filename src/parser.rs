use crate::blocks::*;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// Parse a markdown string into a list of blocks
pub fn parse(markdown: &str) -> Vec<Block> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION;

    let parser = Parser::new_ext(markdown, options);
    let mut builder = DocBuilder::new();

    for event in parser {
        builder.process(event);
    }

    builder.finish()
}

/// Tracks the current inline style state (toggles on Start/End of emphasis, strong, etc.)
#[derive(Default, Clone)]
struct StyleState {
    bold: bool,
    italic: bool,
    strikethrough: bool,
    link_url: Option<String>,
}

impl StyleState {
    fn to_inline_style(&self) -> InlineStyle {
        InlineStyle {
            bold: self.bold,
            italic: self.italic,
            strikethrough: self.strikethrough,
            code: false,
        }
    }
}

/// Represents a container context on the parsing stack
enum Context {
    Root {
        blocks: Vec<Block>,
    },
    Heading {
        level: HeadingLevel,
        spans: Vec<StyledSpan>,
    },
    Paragraph {
        spans: Vec<StyledSpan>,
    },
    CodeBlock {
        lang: Option<String>,
        code: String,
    },
    BlockQuote {
        blocks: Vec<Block>,
    },
    List {
        start: Option<u64>,
        items: Vec<ListItem>,
    },
    ListItem {
        spans: Vec<StyledSpan>,
        children: Vec<Block>,
    },
    Table {
        in_head: bool,
        headers: Vec<Vec<StyledSpan>>,
        rows: Vec<Vec<Vec<StyledSpan>>>,
        current_row: Vec<Vec<StyledSpan>>,
    },
    TableCell {
        spans: Vec<StyledSpan>,
    },
    Image {
        url: String,
        alt: String,
    },
    HtmlBlock {
        content: String,
    },
}

impl Context {
    fn push_block(&mut self, block: Block) {
        match self {
            Context::Root { blocks } | Context::BlockQuote { blocks } => blocks.push(block),
            Context::ListItem { children, .. } => children.push(block),
            _ => {} // silently drop blocks in unexpected contexts
        }
    }

    fn push_spans_as_paragraph(&mut self, spans: Vec<StyledSpan>) {
        if !spans.is_empty() {
            self.push_block(Block::Paragraph { spans });
        }
    }
}

struct DocBuilder {
    stack: Vec<Context>,
    style: StyleState,
}

impl DocBuilder {
    fn new() -> Self {
        Self {
            stack: vec![Context::Root {
                blocks: Vec::new(),
            }],
            style: StyleState::default(),
        }
    }

    fn current(&mut self) -> &mut Context {
        self.stack.last_mut().expect("stack should never be empty")
    }

    fn push_span(&mut self, text: String, code: bool) {
        let span = StyledSpan {
            text,
            style: InlineStyle {
                code,
                ..self.style.to_inline_style()
            },
            link_url: self.style.link_url.clone(),
        };

        match self.current() {
            Context::Heading { spans, .. }
            | Context::Paragraph { spans, .. }
            | Context::ListItem { spans, .. }
            | Context::TableCell { spans, .. } => spans.push(span),
            _ => {}
        }
    }

    fn process(&mut self, event: Event) {
        match event {
            // Container start events — push new context
            Event::Start(Tag::Heading { level, .. }) => {
                self.stack.push(Context::Heading {
                    level: HeadingLevel::from_pulldown(level),
                    spans: Vec::new(),
                });
            }
            Event::Start(Tag::Paragraph) => {
                self.stack.push(Context::Paragraph {
                    spans: Vec::new(),
                });
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => {
                        let s = info.split_whitespace().next().unwrap_or("").to_string();
                        if s.is_empty() { None } else { Some(s) }
                    }
                    CodeBlockKind::Indented => None,
                };
                self.stack.push(Context::CodeBlock {
                    lang,
                    code: String::new(),
                });
            }
            Event::Start(Tag::BlockQuote(_)) => {
                self.stack.push(Context::BlockQuote {
                    blocks: Vec::new(),
                });
            }
            Event::Start(Tag::List(start)) => {
                self.stack.push(Context::List {
                    start,
                    items: Vec::new(),
                });
            }
            Event::Start(Tag::Item) => {
                self.stack.push(Context::ListItem {
                    spans: Vec::new(),
                    children: Vec::new(),
                });
            }
            Event::Start(Tag::Table(_alignments)) => {
                self.stack.push(Context::Table {
                    in_head: false,
                    headers: Vec::new(),
                    rows: Vec::new(),
                    current_row: Vec::new(),
                });
            }
            Event::Start(Tag::TableHead) => {
                if let Context::Table { in_head, .. } = self.current() {
                    *in_head = true;
                }
            }
            Event::Start(Tag::TableRow) => {
                // current_row is already empty from previous finalization
            }
            Event::Start(Tag::TableCell) => {
                self.stack.push(Context::TableCell {
                    spans: Vec::new(),
                });
            }

            // Inline style toggles
            Event::Start(Tag::Emphasis) => self.style.italic = true,
            Event::End(TagEnd::Emphasis) => self.style.italic = false,
            Event::Start(Tag::Strong) => self.style.bold = true,
            Event::End(TagEnd::Strong) => self.style.bold = false,
            Event::Start(Tag::Strikethrough) => self.style.strikethrough = true,
            Event::End(TagEnd::Strikethrough) => self.style.strikethrough = false,
            Event::Start(Tag::Link { dest_url, .. }) => {
                self.style.link_url = Some(dest_url.to_string());
            }
            Event::End(TagEnd::Link) => {
                self.style.link_url = None;
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                self.stack.push(Context::Image {
                    url: dest_url.to_string(),
                    alt: String::new(),
                });
            }
            Event::End(TagEnd::Image) => {
                if let Some(Context::Image { url, alt }) = self.stack.pop() {
                    // Images are typically wrapped in a Paragraph by pulldown-cmark.
                    // Pop the paragraph (push any preceding spans), then push Image to parent.
                    if matches!(self.current(), Context::Paragraph { .. }) {
                        if let Some(Context::Paragraph { spans }) = self.stack.pop() {
                            if !spans.is_empty() {
                                self.current().push_block(Block::Paragraph { spans });
                            }
                        }
                    }
                    self.current().push_block(Block::Image { alt, url });
                }
            }

            // Text content
            Event::Text(text) => {
                let s = text.to_string();
                match self.current() {
                    Context::CodeBlock { code, .. } => code.push_str(&s),
                    Context::Image { alt, .. } => alt.push_str(&s),
                    _ => self.push_span(s, false),
                }
            }
            Event::Code(text) => {
                self.push_span(text.to_string(), true);
            }
            Event::SoftBreak => {
                self.push_span(" ".to_string(), false);
            }
            Event::HardBreak => {
                self.push_span("\n".to_string(), false);
            }

            // Thematic break
            Event::Rule => {
                self.current().push_block(Block::ThematicBreak);
            }

            // Container end events — pop context and finalize
            Event::End(TagEnd::Heading(_)) => {
                if let Some(Context::Heading { level, spans }) = self.stack.pop() {
                    self.current().push_block(Block::Heading { level, spans, id: None });
                }
            }
            Event::End(TagEnd::Paragraph) => {
                // Paragraph may already have been popped by an Image block
                if !matches!(self.current(), Context::Paragraph { .. }) {
                    // Already handled (e.g., image popped the paragraph)
                } else if let Some(Context::Paragraph { spans }) = self.stack.pop() {
                    // In a list item, merge paragraph spans into the item's own spans
                    // if the item has no spans yet (avoids double-wrapping)
                    match self.current() {
                        Context::ListItem {
                            spans: item_spans,
                            children,
                            ..
                        } if item_spans.is_empty() && children.is_empty() => {
                            *item_spans = spans;
                        }
                        ctx => ctx.push_spans_as_paragraph(spans),
                    }
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(Context::CodeBlock { lang, mut code }) = self.stack.pop() {
                    // Trim trailing newline that pulldown-cmark adds
                    if code.ends_with('\n') {
                        code.pop();
                    }
                    self.current().push_block(Block::CodeBlock { lang, code });
                }
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                if let Some(Context::BlockQuote { blocks }) = self.stack.pop() {
                    self.current().push_block(Block::BlockQuote { blocks });
                }
            }
            Event::End(TagEnd::List(ordered)) => {
                if let Some(Context::List { start, items, .. }) = self.stack.pop() {
                    self.current().push_block(Block::List {
                        ordered,
                        start,
                        items,
                    });
                }
            }
            Event::End(TagEnd::Item) => {
                if let Some(Context::ListItem { spans, children }) = self.stack.pop()
                    && let Context::List { items, .. } = self.current()
                {
                    items.push(ListItem { spans, children });
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(Context::TableCell { spans }) = self.stack.pop()
                    && let Context::Table { current_row, .. } = self.current()
                {
                    current_row.push(spans);
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Context::Table {
                    in_head,
                    headers,
                    current_row,
                    ..
                } = self.current()
                {
                    *headers = std::mem::take(current_row);
                    *in_head = false;
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Context::Table {
                    in_head,
                    rows,
                    current_row,
                    ..
                } = self.current()
                    && !*in_head
                {
                    rows.push(std::mem::take(current_row));
                }
            }
            Event::End(TagEnd::Table) => {
                if let Some(Context::Table {
                    headers, rows, ..
                }) = self.stack.pop()
                {
                    self.current().push_block(Block::Table { headers, rows });
                }
            }

            // HTML block events
            Event::Start(Tag::HtmlBlock) => {
                self.stack.push(Context::HtmlBlock {
                    content: String::new(),
                });
            }
            Event::Html(text) => {
                if let Context::HtmlBlock { content } = self.current() {
                    content.push_str(&text);
                }
            }
            Event::End(TagEnd::HtmlBlock) => {
                if let Some(Context::HtmlBlock { content }) = self.stack.pop() {
                    if let Some(block) = parse_html_block(&content) {
                        self.current().push_block(block);
                    }
                }
            }

            // Inline HTML tags — map known tags to style toggles
            Event::InlineHtml(html) => {
                let tag = html.trim();
                match tag {
                    "<em>" | "<i>" => self.style.italic = true,
                    "</em>" | "</i>" => self.style.italic = false,
                    "<strong>" | "<b>" => self.style.bold = true,
                    "</strong>" | "</b>" => self.style.bold = false,
                    "<del>" | "<s>" => self.style.strikethrough = true,
                    "</del>" | "</s>" => self.style.strikethrough = false,
                    "<br>" | "<br/>" | "<br />" => {
                        self.push_span("\n".to_string(), false);
                    }
                    _ => {} // strip unknown inline HTML
                }
            }

            // Ignore everything else (images, footnotes, etc.)
            _ => {}
        }
    }

    fn finish(mut self) -> Vec<Block> {
        match self.stack.pop() {
            Some(Context::Root { blocks }) => blocks,
            _ => Vec::new(),
        }
    }
}

/// Parse an HTML block into a Block. Handles heading tags and strips other HTML to paragraphs.
fn parse_html_block(html: &str) -> Option<Block> {
    let trimmed = html.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Check for <h1> through <h6> heading tags
    if trimmed.starts_with("<h") {
        if let Some(level) = trimmed.as_bytes().get(2).and_then(|&b| match b {
            b'1' => Some(HeadingLevel::H1),
            b'2' => Some(HeadingLevel::H2),
            b'3' => Some(HeadingLevel::H3),
            b'4' => Some(HeadingLevel::H4),
            b'5' => Some(HeadingLevel::H5),
            b'6' => Some(HeadingLevel::H6),
            _ => None,
        }) {
            // Extract id attribute if present
            let id = trimmed.find("id=\"").and_then(|i| {
                let rest = &trimmed[i + 4..];
                rest.find('"').map(|end| rest[..end].to_string())
            });
            // Extract text between > and </h
            if let Some(start) = trimmed.find('>') {
                if let Some(end) = trimmed.find("</h") {
                    let text = strip_html_tags(&trimmed[start + 1..end]);
                    if !text.is_empty() {
                        return Some(Block::Heading {
                            level,
                            spans: vec![StyledSpan {
                                text,
                                style: InlineStyle::default(),
                                link_url: None,
                            }],
                            id,
                        });
                    }
                }
            }
        }
    }

    // For other HTML blocks, strip tags and create a paragraph
    let text = strip_html_tags(trimmed);
    if text.is_empty() {
        return None;
    }
    Some(Block::Paragraph {
        spans: vec![StyledSpan {
            text,
            style: InlineStyle::default(),
            link_url: None,
        }],
    })
}

/// Strip HTML tags from a string, keeping only text content.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headings() {
        let blocks = parse("# Hello\n## World");
        assert_eq!(blocks.len(), 2);
        assert!(matches!(&blocks[0], Block::Heading { level: HeadingLevel::H1, .. }));
        assert!(matches!(&blocks[1], Block::Heading { level: HeadingLevel::H2, .. }));
    }

    #[test]
    fn test_paragraph_with_inline_styles() {
        let blocks = parse("Hello **bold** and *italic*");
        assert_eq!(blocks.len(), 1);
        if let Block::Paragraph { spans } = &blocks[0] {
            assert!(spans.len() >= 3);
            assert!(!spans[0].style.bold);
            assert!(spans.iter().any(|s| s.style.bold && s.text == "bold"));
            assert!(spans.iter().any(|s| s.style.italic && s.text == "italic"));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_code_block() {
        let blocks = parse("```rust\nfn main() {}\n```");
        assert_eq!(blocks.len(), 1);
        if let Block::CodeBlock { lang, code } = &blocks[0] {
            assert_eq!(lang.as_deref(), Some("rust"));
            assert_eq!(code, "fn main() {}");
        } else {
            panic!("expected code block");
        }
    }

    #[test]
    fn test_unordered_list() {
        let blocks = parse("- one\n- two\n- three");
        assert_eq!(blocks.len(), 1);
        if let Block::List { ordered, items, .. } = &blocks[0] {
            assert!(!ordered);
            assert_eq!(items.len(), 3);
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_ordered_list() {
        let blocks = parse("1. first\n2. second");
        assert_eq!(blocks.len(), 1);
        if let Block::List { ordered, start, items, .. } = &blocks[0] {
            assert!(ordered);
            assert_eq!(*start, Some(1));
            assert_eq!(items.len(), 2);
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |";
        let blocks = parse(md);
        assert_eq!(blocks.len(), 1);
        if let Block::Table { headers, rows } = &blocks[0] {
            assert_eq!(headers.len(), 2);
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].len(), 2);
        } else {
            panic!("expected table");
        }
    }

    #[test]
    fn test_blockquote() {
        let blocks = parse("> quoted text");
        assert_eq!(blocks.len(), 1);
        if let Block::BlockQuote { blocks: inner } = &blocks[0] {
            assert_eq!(inner.len(), 1);
            assert!(matches!(&inner[0], Block::Paragraph { .. }));
        } else {
            panic!("expected blockquote");
        }
    }

    #[test]
    fn test_thematic_break() {
        let blocks = parse("above\n\n---\n\nbelow");
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[1], Block::ThematicBreak));
    }

    #[test]
    fn test_inline_code() {
        let blocks = parse("Use `println!` here");
        if let Block::Paragraph { spans } = &blocks[0] {
            assert!(spans.iter().any(|s| s.style.code && s.text == "println!"));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_link() {
        let blocks = parse("[click](https://example.com)");
        if let Block::Paragraph { spans } = &blocks[0] {
            let link_span = spans.iter().find(|s| s.link_url.is_some()).unwrap();
            assert_eq!(link_span.text, "click");
            assert_eq!(link_span.link_url.as_deref(), Some("https://example.com"));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_nested_list() {
        let blocks = parse("- First\n  - Nested A\n  - Nested B\n- Second");
        assert_eq!(blocks.len(), 1);
        if let Block::List { items, .. } = &blocks[0] {
            assert_eq!(items.len(), 2);
            // First item has children (nested list)
            assert_eq!(items[0].children.len(), 1);
            if let Block::List { items: nested, .. } = &items[0].children[0] {
                assert_eq!(nested.len(), 2);
            } else {
                panic!("expected nested list");
            }
            // Second item has no children
            assert!(items[1].children.is_empty());
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_nested_blockquote() {
        let blocks = parse("> Outer\n>> Nested");
        assert_eq!(blocks.len(), 1);
        if let Block::BlockQuote { blocks: outer } = &blocks[0] {
            // Should contain a paragraph and a nested blockquote
            assert!(outer.iter().any(|b| matches!(b, Block::BlockQuote { .. })),
                "expected nested blockquote");
        } else {
            panic!("expected blockquote");
        }
    }

    #[test]
    fn test_list_text_content_includes_children() {
        let blocks = parse("- First\n  - Nested\n- Second");
        let text = blocks[0].text_content();
        assert!(text.contains("First"), "should contain top-level item");
        assert!(text.contains("Nested"), "should contain nested item");
        assert!(text.contains("Second"), "should contain second item");
    }

    #[test]
    fn test_html_heading_block() {
        let blocks = parse("<h2 id=\"overview\">Overview</h2>\n\nSome text.");
        assert_eq!(blocks.len(), 2);
        if let Block::Heading { level, spans, id } = &blocks[0] {
            assert_eq!(*level, HeadingLevel::H2);
            assert_eq!(spans[0].text, "Overview");
            assert_eq!(id.as_deref(), Some("overview"));
        } else {
            panic!("expected heading from HTML block, got {:?}", blocks[0]);
        }
    }

    #[test]
    fn test_html_heading_h3() {
        let blocks = parse("<h3 id=\"foo\">Section Title</h3>");
        assert_eq!(blocks.len(), 1);
        if let Block::Heading { level, spans, id } = &blocks[0] {
            assert_eq!(*level, HeadingLevel::H3);
            assert_eq!(spans[0].text, "Section Title");
            assert_eq!(id.as_deref(), Some("foo"));
        } else {
            panic!("expected h3 heading");
        }
    }

    #[test]
    fn test_inline_html_em() {
        let blocks = parse("This is <em>emphasized</em> text.");
        assert_eq!(blocks.len(), 1);
        if let Block::Paragraph { spans } = &blocks[0] {
            assert!(spans.iter().any(|s| s.style.italic && s.text == "emphasized"),
                "expected italic span, got {:?}", spans);
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_html_div_becomes_paragraph() {
        let blocks = parse("<div>\nSome content\n</div>");
        assert!(blocks.iter().any(|b| {
            if let Block::Paragraph { spans } = b {
                spans.iter().any(|s| s.text.contains("Some content"))
            } else { false }
        }), "expected paragraph with div content");
    }

    #[test]
    fn test_image_block() {
        let blocks = parse("![Alt text](image.png)");
        assert_eq!(blocks.len(), 1);
        if let Block::Image { alt, url } = &blocks[0] {
            assert_eq!(alt, "Alt text");
            assert_eq!(url, "image.png");
        } else {
            panic!("expected image block, got {:?}", blocks[0]);
        }
    }
}
