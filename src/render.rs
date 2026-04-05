use crate::blocks::*;
use crate::graphics::TextRenderer;
use crate::highlight::Highlighter;
use crate::theme::Theme;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::collections::HashSet;

// ── Element model: mixed text + images ─────────────────────────────

pub(crate) enum Element {
    TextLine(Line<'static>),
    Image {
        protocol: Option<StatefulProtocol>,
        height_rows: u16,
        text: String,
        level: HeadingLevel,
    },
}

impl Element {
    pub(crate) fn height(&self) -> u16 {
        match self {
            Element::TextLine(_) => 1,
            Element::Image { height_rows, .. } => *height_rows,
        }
    }
}

pub(crate) struct SearchMatch {
    pub(crate) element_idx: usize,
    pub(crate) positions: HashSet<usize>,
}

// ── Rendering: blocks → Elements ───────────────────────────────────

/// Estimate heading height in terminal rows without rendering.
pub(crate) fn estimate_heading_rows(
    text: &str,
    level: HeadingLevel,
    pixel_width: u32,
    cell_height: u16,
) -> u16 {
    if cell_height == 0 {
        return 3;
    }
    let font_size = cell_height as f32 * level.font_scale();
    let char_width = font_size * 0.55;
    let text_width = text.chars().count() as f32 * char_width;
    let lines = (text_width / pixel_width as f32).ceil().max(1.0);
    let line_height = font_size * 1.15;
    (lines * line_height / cell_height as f32).ceil().max(1.0) as u16
}

pub(crate) fn render_blocks(
    blocks: &[Block],
    width: u16,
    renderer: &mut TextRenderer,
    picker: &mut Picker,
    pixel_width: u32,
    cell_height: u16,
    highlighter: &Highlighter,
    theme: &Theme,
    lazy: bool,
    graphics: bool,
) -> Vec<Element> {
    let mut elements = Vec::new();
    for block in blocks {
        let is_heading = matches!(block, Block::Heading { .. });
        render_block(
            block,
            &mut elements,
            width,
            0,
            renderer,
            picker,
            pixel_width,
            cell_height,
            highlighter,
            theme,
            lazy,
            graphics,
        );
        if !is_heading {
            elements.push(Element::TextLine(Line::raw("")));
        }
    }
    // Remove trailing blank
    if let Some(Element::TextLine(line)) = elements.last() {
        if line.spans.iter().all(|s| s.content.is_empty()) {
            elements.pop();
        }
    }
    elements
}

fn render_block(
    block: &Block,
    elements: &mut Vec<Element>,
    width: u16,
    indent: u16,
    renderer: &mut TextRenderer,
    picker: &mut Picker,
    pixel_width: u32,
    cell_height: u16,
    highlighter: &Highlighter,
    theme: &Theme,
    lazy: bool,
    graphics: bool,
) {
    match block {
        Block::Heading { level, spans } => {
            let text: String = spans.iter().map(|s| s.text.as_str()).collect();
            if !graphics {
                // No graphics protocol — render as bold colored text with per-level colors
                let color = match level {
                    HeadingLevel::H1 => theme.link,
                    HeadingLevel::H2 => theme.inline_code,
                    HeadingLevel::H3 => theme.search_match,
                    HeadingLevel::H4 => theme.list_bullet,
                    _ => theme.heading_text,
                };
                let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
                elements.push(Element::TextLine(Line::styled(text, style)));
            } else if lazy {
                let height_rows = estimate_heading_rows(&text, *level, pixel_width, cell_height);
                elements.push(Element::Image {
                    protocol: None,
                    height_rows,
                    text,
                    level: *level,
                });
            } else {
                let base_font_size = cell_height as f32;
                let img = renderer.render_heading(&text, *level, pixel_width, base_font_size, crate::theme::color_to_rgb(theme.heading_text));
                let img_height_px = img.height();
                let height_rows = if cell_height > 0 {
                    ((img_height_px as u16 + cell_height - 1) / cell_height).max(1)
                } else {
                    3
                };
                let protocol = picker.new_resize_protocol(img);
                elements.push(Element::Image {
                    protocol: Some(protocol),
                    height_rows,
                    text: text.clone(),
                    level: *level,
                });
            }
        }

        Block::Paragraph { spans } => {
            if graphics {
                let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                let img = renderer.render_paragraph(spans, pixel_width, cell_height as f32, theme);
                let img_height_px = img.height();
                let height_rows = if cell_height > 0 {
                    ((img_height_px as u16 + cell_height - 1) / cell_height).max(1)
                } else {
                    3
                };
                let protocol = picker.new_resize_protocol(img);
                elements.push(Element::Image {
                    protocol: Some(protocol),
                    height_rows,
                    text,
                    level: HeadingLevel::H1,
                });
            } else {
                let ratatui_spans = styled_spans_to_ratatui(spans, theme);
                let mut lines = Vec::new();
                wrap_spans_into_lines(ratatui_spans, &mut lines, width, indent);
                for line in lines {
                    elements.push(Element::TextLine(line));
                }
            }
        }

        Block::CodeBlock { lang, code } => {
            if graphics {
                let img = renderer.render_code_block(
                    code,
                    lang.as_deref(),
                    pixel_width,
                    cell_height as f32,
                    highlighter,
                    theme,
                );
                let img_height_px = img.height();
                let height_rows = if cell_height > 0 {
                    ((img_height_px as u16 + cell_height - 1) / cell_height).max(1)
                } else {
                    3
                };
                let protocol = picker.new_resize_protocol(img);
                elements.push(Element::Image {
                    protocol: Some(protocol),
                    height_rows,
                    text: code.clone(),
                    level: HeadingLevel::H1,
                });
            } else {
                let border_style = Style::default().fg(theme.code_block_border);
                let pad = "  ";
                let box_width = (width as usize).saturating_sub(pad.len() + 4);
                let lang_label = lang.as_deref().unwrap_or("");
                let inner_width = box_width.saturating_sub(7);

                let fill = box_width.saturating_sub(4 + lang_label.len() + 1);
                elements.push(Element::TextLine(Line::from(vec![
                    Span::raw(pad.to_string()),
                    Span::styled(
                        format!("╭─ {} {}╮", lang_label, "─".repeat(fill)),
                        border_style,
                    ),
                ])));

                elements.push(Element::TextLine(Line::from(vec![
                    Span::raw(pad.to_string()),
                    Span::styled("│".to_string(), border_style),
                    Span::raw(" ".repeat(box_width.saturating_sub(2))),
                    Span::styled("│".to_string(), border_style),
                ])));

                let mut hl = lang
                    .as_deref()
                    .and_then(|l| highlighter.create_highlighter(l));

                for code_line in code.lines() {
                    let mut line_spans = vec![
                        Span::raw(pad.to_string()),
                        Span::styled("│   ".to_string(), border_style),
                    ];

                    if let Some(ref mut h) = hl {
                        let highlighted = highlighter.highlight_line(
                            code_line,
                            lang.as_deref().unwrap_or(""),
                            h,
                        );
                        // Calculate displayed char count for padding
                        let text_len: usize =
                            highlighted.iter().map(|s| s.content.chars().count()).sum();
                        line_spans.extend(highlighted);
                        if text_len < inner_width {
                            line_spans.push(Span::raw(" ".repeat(inner_width - text_len)));
                        }
                    } else {
                        let truncated = pad_or_truncate(code_line, inner_width);
                        line_spans.push(Span::styled(
                            truncated,
                            Style::default().fg(theme.code_block_text),
                        ));
                    }

                    line_spans.push(Span::styled("  │".to_string(), border_style));
                    elements.push(Element::TextLine(Line::from(line_spans)));
                }

                elements.push(Element::TextLine(Line::from(vec![
                    Span::raw(pad.to_string()),
                    Span::styled("│".to_string(), border_style),
                    Span::raw(" ".repeat(box_width.saturating_sub(2))),
                    Span::styled("│".to_string(), border_style),
                ])));

                elements.push(Element::TextLine(Line::from(vec![
                    Span::raw(pad.to_string()),
                    Span::styled(
                        format!("╰{}╯", "─".repeat(box_width.saturating_sub(2))),
                        border_style,
                    ),
                ])));
            }
        }

        Block::Table { headers, rows } => {
            if graphics {
                let searchable: String = headers
                    .iter()
                    .chain(rows.iter().flat_map(|r| r.iter()))
                    .flat_map(|cell| cell.iter().map(|s| s.text.as_str()))
                    .collect::<Vec<_>>()
                    .join(" ");
                let img =
                    renderer.render_table(headers, rows, pixel_width, cell_height as f32, theme);
                let height_rows = if cell_height > 0 {
                    ((img.height() as u16 + cell_height - 1) / cell_height).max(1)
                } else {
                    3
                };
                let protocol = picker.new_resize_protocol(img);
                elements.push(Element::Image {
                    protocol: Some(protocol),
                    height_rows,
                    text: searchable,
                    level: HeadingLevel::H1,
                });
            } else {
                let mut lines = Vec::new();
                render_table(headers, rows, &mut lines, width, theme);
                for line in lines {
                    elements.push(Element::TextLine(line));
                }
            }
        }

        Block::BlockQuote { blocks } => {
            if graphics {
                // Collect all text from inner blocks into styled spans
                let mut all_spans: Vec<crate::graphics::StyledTextSpan> = Vec::new();
                for b in blocks {
                    match b {
                        Block::Paragraph { spans } => {
                            all_spans.extend(crate::graphics::styled_spans_to_text_spans(spans, theme));
                            all_spans.push(crate::graphics::StyledTextSpan {
                                text: "\n".to_string(),
                                color: crate::theme::color_to_rgb(theme.heading_text),
                                bold: false,
                                italic: false,
                            });
                        }
                        Block::Heading { spans, .. } => {
                            let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                            all_spans.push(crate::graphics::StyledTextSpan {
                                text,
                                color: crate::theme::color_to_rgb(theme.heading_text),
                                bold: true,
                                italic: false,
                            });
                            all_spans.push(crate::graphics::StyledTextSpan {
                                text: "\n".to_string(),
                                color: crate::theme::color_to_rgb(theme.heading_text),
                                bold: false,
                                italic: false,
                            });
                        }
                        _ => {
                            // For other block types in blockquotes, skip (simplification)
                        }
                    }
                }
                // Trim trailing newline span if present
                if all_spans.last().map(|s| s.text == "\n").unwrap_or(false) {
                    all_spans.pop();
                }
                if all_spans.is_empty() {
                    all_spans.push(crate::graphics::StyledTextSpan {
                        text: " ".to_string(),
                        color: crate::theme::color_to_rgb(theme.heading_text),
                        bold: false,
                        italic: false,
                    });
                }
                let searchable: String = all_spans.iter().map(|s| s.text.as_str()).collect();
                let img = renderer.render_blockquote(&all_spans, pixel_width, cell_height as f32, theme);
                let height_rows = if cell_height > 0 {
                    ((img.height() as u16 + cell_height - 1) / cell_height).max(1)
                } else {
                    3
                };
                let protocol = picker.new_resize_protocol(img);
                elements.push(Element::Image {
                    protocol: Some(protocol),
                    height_rows,
                    text: searchable,
                    level: HeadingLevel::H1,
                });
            } else {
                let indent_str = "  ";
                let quote_bar = "▎ ";
                let prefix_len = (indent_str.len() + quote_bar.len()) as u16;
                let inner_width = width.saturating_sub(prefix_len);
                let bg = theme.blockquote_bg;
                let bar_style = Style::default().fg(theme.blockquote_bar).bg(bg);
                let pad_style = Style::default().bg(bg);

                let mut inner_elements = Vec::new();
                for b in blocks {
                    render_block(
                        b,
                        &mut inner_elements,
                        inner_width,
                        0,
                        renderer,
                        picker,
                        pixel_width,
                        cell_height,
                        highlighter,
                        theme,
                        lazy,
                        false, // force non-graphics for text fallback
                    );
                }
                for elem in inner_elements {
                    if let Element::TextLine(line) = elem {
                        let mut spans = vec![
                            Span::styled(indent_str.to_string(), Style::default()),
                            Span::styled(quote_bar.to_string(), bar_style),
                        ];
                        for span in line.spans {
                            spans.push(Span::styled(span.content.to_string(), span.style.bg(bg)));
                        }
                        let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
                        let w = width as usize;
                        if used < w {
                            spans.push(Span::styled(" ".repeat(w - used), pad_style));
                        }
                        elements.push(Element::TextLine(Line::from(spans)));
                    }
                }
            }
        }

        Block::List {
            ordered,
            start,
            items,
        } => {
            if graphics {
                let mut num = start.unwrap_or(1);
                for item in items {
                    let bullet = if *ordered {
                        let b = format!("{}. ", num);
                        num += 1;
                        b
                    } else {
                        "● ".to_string()
                    };
                    let bullet_color = crate::theme::color_to_rgb(theme.list_bullet);

                    let mut spans = vec![crate::graphics::StyledTextSpan {
                        text: bullet,
                        color: bullet_color,
                        bold: false,
                        italic: false,
                    }];
                    spans.extend(crate::graphics::styled_spans_to_text_spans(&item.spans, theme));

                    let searchable: String = item.spans.iter().map(|s| s.text.as_str()).collect();
                    let opts = crate::graphics::RenderOptions {
                        font_size: cell_height as f32,
                        line_height_factor: 1.4,
                        ..Default::default()
                    };
                    let img = renderer.render_styled_text(&spans, pixel_width, &opts).image;
                    let height_rows = if cell_height > 0 {
                        ((img.height() as u16 + cell_height - 1) / cell_height).max(1)
                    } else {
                        3
                    };
                    let protocol = picker.new_resize_protocol(img);
                    elements.push(Element::Image {
                        protocol: Some(protocol),
                        height_rows,
                        text: searchable,
                        level: HeadingLevel::H1,
                    });

                    // Render child blocks (nested lists, etc.)
                    for child in &item.children {
                        render_block(
                            child,
                            elements,
                            width,
                            indent + 4,
                            renderer,
                            picker,
                            pixel_width,
                            cell_height,
                            highlighter,
                            theme,
                            lazy,
                            graphics,
                        );
                    }
                }
            } else {
                let list_indent = "  ";
                let mut num = start.unwrap_or(1);
                for item in items {
                    let bullet = if *ordered {
                        let b = format!("{}. ", num);
                        num += 1;
                        b
                    } else {
                        "● ".to_string()
                    };
                    let bullet_width = (list_indent.len() + bullet.len()) as u16;

                    let mut first_spans = vec![
                        Span::raw(list_indent.to_string()),
                        Span::styled(bullet, Style::default().fg(theme.list_bullet)),
                    ];
                    first_spans.extend(styled_spans_to_ratatui(&item.spans, theme));
                    elements.push(Element::TextLine(Line::from(first_spans)));

                    let child_indent = indent + bullet_width;
                    for child in &item.children {
                        render_block(
                            child,
                            elements,
                            width.saturating_sub(child_indent),
                            child_indent,
                            renderer,
                            picker,
                            pixel_width,
                            cell_height,
                            highlighter,
                            theme,
                            lazy,
                            graphics,
                        );
                    }
                }
            }
        }

        Block::ThematicBreak => {
            if graphics {
                let img = renderer.render_hr(pixel_width, cell_height, theme);
                let protocol = picker.new_resize_protocol(img);
                elements.push(Element::Image {
                    protocol: Some(protocol),
                    height_rows: 1,
                    text: String::new(),
                    level: HeadingLevel::H1,
                });
            } else {
                let rule = "─".repeat(width as usize);
                elements.push(Element::TextLine(Line::styled(
                    rule,
                    Style::default().fg(theme.hr),
                )));
            }
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────

fn styled_spans_to_ratatui<'a>(spans: &[StyledSpan], theme: &Theme) -> Vec<Span<'a>> {
    spans
        .iter()
        .map(|s| {
            let mut style = Style::default();
            if s.style.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if s.style.italic {
                style = style.add_modifier(Modifier::ITALIC);
            }
            if s.style.strikethrough {
                style = style.add_modifier(Modifier::CROSSED_OUT);
            }
            if s.style.code {
                style = style.fg(theme.inline_code).bg(theme.inline_code_bg);
            }
            if s.link_url.is_some() {
                style = style.fg(theme.link).add_modifier(Modifier::UNDERLINED);
            }
            Span::styled(s.text.clone(), style)
        })
        .collect()
}

fn wrap_spans_into_lines<'a>(
    spans: Vec<Span<'a>>,
    lines: &mut Vec<Line<'a>>,
    width: u16,
    indent: u16,
) {
    let indent_str = " ".repeat(indent as usize);
    let usable = width.saturating_sub(indent) as usize;

    let mut current_line: Vec<Span<'a>> = Vec::new();
    let mut current_len: usize = 0;

    if indent > 0 {
        current_line.push(Span::raw(indent_str.clone()));
    }

    for span in spans {
        let words: Vec<&str> = span.content.split_inclusive(' ').collect();
        for word in words {
            let wlen = word.chars().count();
            if current_len + wlen > usable && current_len > 0 {
                lines.push(Line::from(std::mem::take(&mut current_line)));
                current_len = 0;
                if indent > 0 {
                    current_line.push(Span::raw(indent_str.clone()));
                }
            }
            current_line.push(Span::styled(word.to_string(), span.style));
            current_len += wlen;
        }
    }

    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }
}

fn render_table<'a>(
    headers: &[Vec<StyledSpan>],
    rows: &[Vec<Vec<StyledSpan>>],
    lines: &mut Vec<Line<'a>>,
    width: u16,
    theme: &Theme,
) {
    let num_cols = headers.len();
    if num_cols == 0 {
        return;
    }

    let col_width = ((width as usize).saturating_sub(num_cols + 1)) / num_cols;
    let text_width = col_width.saturating_sub(2);

    let border_style = Style::default().fg(theme.table_border);
    let header_style = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);

    let top = format!(
        "┌{}┐",
        (0..num_cols)
            .map(|_| "─".repeat(col_width))
            .collect::<Vec<_>>()
            .join("┬")
    );
    lines.push(Line::styled(truncate(&top, width as usize), border_style));

    let mut header_spans = vec![Span::styled("│".to_string(), border_style)];
    for (i, cell) in headers.iter().enumerate() {
        let text = cell_text(cell);
        header_spans.push(Span::styled(
            format!(" {} ", pad_or_truncate(&text, text_width)),
            header_style,
        ));
        if i < num_cols - 1 {
            header_spans.push(Span::styled("│".to_string(), border_style));
        }
    }
    header_spans.push(Span::styled("│".to_string(), border_style));
    lines.push(Line::from(header_spans));

    let sep = format!(
        "├{}┤",
        (0..num_cols)
            .map(|_| "─".repeat(col_width))
            .collect::<Vec<_>>()
            .join("┼")
    );
    lines.push(Line::styled(truncate(&sep, width as usize), border_style));

    for row in rows {
        let mut row_spans = vec![Span::styled("│".to_string(), border_style)];
        for (i, cell) in row.iter().enumerate() {
            let text = cell_text(cell);
            row_spans.push(Span::raw(format!(
                " {} ",
                pad_or_truncate(&text, text_width)
            )));
            if i < num_cols - 1 {
                row_spans.push(Span::styled("│".to_string(), border_style));
            }
        }
        row_spans.push(Span::styled("│".to_string(), border_style));
        lines.push(Line::from(row_spans));
    }

    let bottom = format!(
        "└{}┘",
        (0..num_cols)
            .map(|_| "─".repeat(col_width))
            .collect::<Vec<_>>()
            .join("┴")
    );
    lines.push(Line::styled(truncate(&bottom, width as usize), border_style));
}

fn cell_text(spans: &[StyledSpan]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

fn pad_or_truncate(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count >= width {
        let truncated: String = s.chars().take(width.saturating_sub(1)).collect();
        format!("{}…", truncated)
    } else {
        let padding = width - char_count;
        format!("{}{}", s, " ".repeat(padding))
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        s.chars().take(max).collect()
    } else {
        s.to_string()
    }
}

pub(crate) fn element_text(elem: &Element) -> String {
    match elem {
        Element::TextLine(line) => line.spans.iter().map(|s| s.content.as_ref()).collect(),
        Element::Image { text, .. } => text.clone(),
    }
}

pub(crate) fn regex_char_positions(re: &regex::Regex, text: &str) -> HashSet<usize> {
    let mut positions = HashSet::new();
    for mat in re.find_iter(text) {
        // Convert byte range to char indices
        let start_chars = text[..mat.start()].chars().count();
        let match_chars = mat.as_str().chars().count();
        for i in 0..match_chars {
            positions.insert(start_chars + i);
        }
    }
    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_heading_rows_single_line() {
        let rows = estimate_heading_rows("Hello", HeadingLevel::H1, 800, 20);
        assert!(rows >= 1 && rows <= 3, "expected 1-3 rows, got {}", rows);
    }

    #[test]
    fn estimate_heading_rows_wrapping() {
        let narrow = estimate_heading_rows("This is a very long heading that should wrap", HeadingLevel::H1, 200, 20);
        let wide = estimate_heading_rows("This is a very long heading that should wrap", HeadingLevel::H1, 2000, 20);
        assert!(narrow > wide, "narrow ({}) should produce more rows than wide ({})", narrow, wide);
    }
}
