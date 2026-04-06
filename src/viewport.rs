use crate::blocks::Block;
use crate::graphics::{self, RenderOptions, StyledTextSpan, TextRenderer};
use crate::highlight::Highlighter;
use crate::theme::Theme;
use image::DynamicImage;
use std::collections::HashSet;

pub struct Viewport {
    pub block_heights: Vec<u32>,
    pub block_offsets: Vec<u32>,
    block_cache: Vec<Option<DynamicImage>>,
    gap: u32,
    heading_before: u32,
    heading_after: u32,
    pub total_height: u32,
    renderer: TextRenderer,
    highlighter: Highlighter,
    pub width_px: u32,
    pub font_size: f32,
    cell_height: u32,
    base_dir: std::path::PathBuf,
    search_highlights: Vec<(usize, usize, usize, bool)>,
    /// Per-block link bounding boxes (populated during rendering)
    link_boxes: Vec<Vec<graphics::LinkBox>>,
    /// Next block index to measure in background (all blocks before this are measured)
    pub next_unmeasured: usize,
    /// Set by ensure_block_rendered when a height correction occurs; cleared by caller
    pub height_corrected_at: Option<usize>,
}

impl Viewport {
    pub fn new(blocks: &[Block], width_px: u32, font_size: f32, cell_height: u16, theme: &Theme, font_override: Option<&str>, spacing: &crate::config::ResolvedSpacing, base_dir: std::path::PathBuf) -> Self {
        let renderer = TextRenderer::new(font_override);
        let highlighter = Highlighter::new(theme.syntax_theme_name());
        let gap = (font_size * spacing.block_gap).ceil() as u32;
        let heading_before = (font_size * spacing.heading_before).ceil() as u32;
        let heading_after = (font_size * spacing.heading_after).ceil() as u32;

        let block_heights: Vec<u32> = blocks
            .iter()
            .map(|b| Self::estimate_block_height(b, width_px, font_size))
            .collect();
        let block_cache = vec![None; blocks.len()];

        let mut block_offsets = Vec::with_capacity(blocks.len());
        let mut cursor = 0u32;
        for (i, &h) in block_heights.iter().enumerate() {
            block_offsets.push(cursor);
            cursor += h;
            if i + 1 < block_heights.len() {
                cursor += Self::gap_between(&blocks[i], &blocks[i + 1], gap, heading_before, heading_after);
            }
        }
        let total_height = cursor;

        Self {
            block_heights,
            block_offsets,
            block_cache,
            gap,
            heading_before,
            heading_after,
            total_height,
            renderer,
            highlighter,
            width_px,
            font_size,
            cell_height: cell_height as u32,
            base_dir,
            search_highlights: Vec::new(),
            link_boxes: vec![Vec::new(); blocks.len()],
            next_unmeasured: 0,
            height_corrected_at: None,
        }
    }

    /// Set search highlights. Returns all affected block indices (old + new) that need retransmission.
    pub fn set_search_highlights(&mut self, highlights: Vec<(usize, usize, usize, bool)>) -> HashSet<usize> {
        let old_blocks: HashSet<usize> = self.search_highlights.iter().map(|h| h.0).collect();
        let new_blocks: HashSet<usize> = highlights.iter().map(|h| h.0).collect();
        let affected: HashSet<usize> = old_blocks.union(&new_blocks).copied().collect();
        for idx in &affected {
            self.evict_block(*idx);
        }
        self.search_highlights = highlights;
        affected
    }

    /// Clear search highlights. Returns affected block indices that need retransmission.
    pub fn clear_search_highlights(&mut self) -> HashSet<usize> {
        let affected: HashSet<usize> = self.search_highlights.iter().map(|h| h.0).collect();
        for idx in &affected {
            self.evict_block(*idx);
        }
        self.search_highlights.clear();
        affected
    }

    /// Ensure a block is rendered and cached. Returns reference to the image.
    pub fn ensure_block_rendered(&mut self, idx: usize, blocks: &[Block], theme: &Theme) -> &DynamicImage {
        if self.block_cache[idx].is_none() {
            // Collect highlight ranges for this block (search + selection)
            let block_highlights: Vec<graphics::HighlightRange> = self.search_highlights.iter()
                .filter(|h| h.0 == idx)
                .map(|h| {
                    let color = if h.3 {
                        crate::theme::color_to_rgb(theme.search_current)
                    } else {
                        crate::theme::color_to_rgb(theme.search_match)
                    };
                    graphics::HighlightRange {
                        byte_start: h.1,
                        byte_end: h.2,
                        color,
                        alpha: if h.3 { 160 } else { 130 },
                    }
                })
                .collect();

            let img = Self::render_block_image(
                &blocks[idx], &mut self.renderer, &self.highlighter,
                self.width_px, self.font_size, theme, block_highlights, &self.base_dir,
            );
            // Capture link bounding boxes from the render pass
            if idx < self.link_boxes.len() {
                self.link_boxes[idx] = std::mem::take(&mut self.renderer.last_link_boxes);
            }
            // Pad height to a multiple of cell_height — Kitty protocol
            // displays images in whole terminal rows
            let ch = self.cell_height;
            let raw_h = img.height();
            let w = img.width();
            let padded_h = if ch > 0 { ((raw_h + ch - 1) / ch) * ch } else { raw_h };
            let img = if padded_h > raw_h {
                let mut pixels = img.into_rgba8().into_raw();
                pixels.resize((w * padded_h * 4) as usize, 0);
                image::DynamicImage::ImageRgba8(
                    image::ImageBuffer::from_raw(w, padded_h, pixels)
                        .expect("pad buffer"),
                )
            } else {
                img
            };

            let actual_h = img.height();
            if actual_h != self.block_heights[idx] {
                let delta = actual_h as i64 - self.block_heights[idx] as i64;
                self.block_heights[idx] = actual_h;
                for j in (idx + 1)..self.block_offsets.len() {
                    self.block_offsets[j] = (self.block_offsets[j] as i64 + delta) as u32;
                }
                self.total_height = (self.total_height as i64 + delta) as u32;
                self.height_corrected_at = Some(idx);
            }

            self.block_cache[idx] = Some(img);
        }
        self.block_cache[idx].as_ref().unwrap()
    }

    pub fn evict_block(&mut self, idx: usize) {
        if idx < self.block_cache.len() {
            self.block_cache[idx] = None;
        }
    }

    pub fn evict_distant_blocks(&mut self, scroll_px: u32, viewport_height: u32) {
        let margin = viewport_height;
        let keep_start = scroll_px.saturating_sub(margin);
        let keep_end = scroll_px + viewport_height + margin;

        for i in 0..self.block_cache.len() {
            if self.block_cache[i].is_none() { continue; }
            let block_top = self.block_offsets[i];
            let block_bottom = block_top + self.block_heights[i];
            if block_bottom < keep_start || block_top > keep_end {
                self.block_cache[i] = None;
            }
        }
    }

    /// Fast link hit test using pre-computed bounding boxes.
    pub fn link_at_pixel(&self, block_idx: usize, x: f32, y: f32) -> Option<String> {
        let tol = 4.0; // tolerance for font metrics edge cases
        if let Some(boxes) = self.link_boxes.get(block_idx) {
            for lb in boxes {
                if x >= lb.x0 && x <= lb.x1 && y >= lb.y0 - tol && y <= lb.y1 + tol {
                    return Some(lb.url.clone());
                }
            }
        }
        None
    }


    pub fn render_status_bar(
        &mut self,
        left_panes: &[graphics::StatusPane],
        right_panes: &[graphics::StatusPane],
        fill_bg: [u8; 3],
        height_px: u32,
    ) -> image::DynamicImage {
        self.renderer.render_status_bar(left_panes, right_panes, fill_bg, self.width_px, height_px, self.font_size)
    }

    pub fn resize(&mut self, blocks: &[Block], new_width_px: u32) {
        self.width_px = new_width_px;
        // Re-measure all blocks at new width
        for i in 0..blocks.len() {
            self.block_heights[i] = self.measure_block_height(&blocks[i]);
        }
        self.block_cache = vec![None; blocks.len()];
        self.next_unmeasured = blocks.len(); // all measured
        self.recompute_offsets(blocks);
    }

    fn gap_between(above: &Block, below: &Block, default_gap: u32, heading_before: u32, heading_after: u32) -> u32 {
        let is_heading = matches!(below, Block::Heading { .. });
        let after_heading = matches!(above, Block::Heading { .. });

        if is_heading {
            heading_before
        } else if after_heading {
            heading_after
        } else {
            default_gap
        }
    }

    fn estimate_block_height(block: &Block, width_px: u32, font_size: f32) -> u32 {
        let line_height = (font_size * 1.4).ceil();
        let char_width = font_size * 0.65; // conservative — better to overestimate than clip

        let estimate_text_height = |text: &str| -> u32 {
            let chars = text.chars().count().max(1) as f32;
            let text_width = chars * char_width;
            let lines = (text_width / width_px as f32).ceil().max(1.0);
            (lines * line_height) as u32
        };

        match block {
            Block::Heading { level, spans, .. } => {
                let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                let scale = level.font_scale();
                let h_line_height = (font_size * scale * 1.15).ceil();
                let h_char_width = font_size * scale * 0.65;
                let chars = text.chars().count().max(1) as f32;
                let lines = (chars * h_char_width / width_px as f32).ceil().max(1.0);
                (lines * h_line_height) as u32
            }
            Block::Paragraph { spans } => {
                let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                estimate_text_height(&text)
            }
            Block::CodeBlock { code, .. } => {
                let lines = code.lines().count().max(1) as f32;
                let padding = (font_size * 0.8) as u32;
                (lines * (font_size * 1.3).ceil()) as u32 + padding
            }
            Block::BlockQuote { blocks: inner } => {
                let mut h = 8u32;
                for b in inner {
                    h += Self::estimate_block_height(b, width_px, font_size);
                }
                h
            }
            Block::List { items, .. } => {
                let mut h = 0u32;
                for item in items {
                    let text: String = item.spans.iter().map(|s| s.text.as_str()).collect();
                    h += estimate_text_height(&text);
                    for child in &item.children {
                        h += Self::estimate_block_height(child, width_px, font_size);
                    }
                }
                h
            }
            Block::Table { rows, .. } => {
                let row_h = (line_height as u32) + 9;
                (1 + rows.len() as u32) * row_h + (2 + rows.len() as u32)
            }
            Block::Image { .. } => (font_size * 10.0) as u32, // rough estimate, corrected on render
            Block::ThematicBreak => font_size as u32,
        }
    }

    /// Measure exact block height via cosmic-text layout (no rasterization).
    fn measure_block_height(&mut self, block: &Block) -> u32 {
        let font_size = self.font_size;
        let width_px = self.width_px;
        let ch = self.cell_height;

        let raw_h = match block {
            Block::Heading { level, spans, .. } => {
                let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                let scale = level.font_scale();
                let pad_top = (font_size * scale * 0.8) as u32;
                let span = vec![StyledTextSpan {
                    text, color: [230, 230, 230], bold: true, italic: false,
                }];
                let opts = RenderOptions {
                    font_size: font_size * scale,
                    line_height_factor: 1.15,
                    padding_top: pad_top,
                    ..Default::default()
                };
                self.renderer.measure_text_height(&span, width_px, &opts)
            }
            Block::Paragraph { spans } => {
                let styled: Vec<StyledTextSpan> = spans.iter().map(|s| StyledTextSpan {
                    text: s.text.clone(), color: [0, 0, 0], bold: s.style.bold, italic: s.style.italic,
                }).collect();
                let opts = RenderOptions {
                    font_size,
                    line_height_factor: 1.4,
                    ..Default::default()
                };
                self.renderer.measure_text_height(&styled, width_px, &opts)
            }
            Block::CodeBlock { lang, code } => {
                // Build same spans as render_code_block (including syntax highlighting)
                let spans = match lang.as_deref() {
                    Some(lang_str) => {
                        let mut s = vec![StyledTextSpan {
                            text: format!("{}\n\n", lang_str),
                            color: [128, 128, 128], bold: false, italic: false,
                        }];
                        s.extend(self.highlighter.highlight_to_styled_spans(code, lang_str));
                        s
                    }
                    None => vec![StyledTextSpan {
                        text: code.clone(),
                        color: [255, 255, 255], bold: false, italic: false,
                    }],
                };
                let inset = (font_size * 2.0) as u32;
                let padding = (font_size * 0.8) as u32;
                let inner_width = width_px.saturating_sub(inset * 2);
                let opts = RenderOptions {
                    font_size,
                    line_height_factor: 1.3,
                    padding_left: padding,
                    padding_top: padding / 2,
                    padding_bottom: padding,
                    use_code_font: true,
                    ..Default::default()
                };
                self.renderer.measure_text_height(&spans, inner_width, &opts)
            }
            Block::List { items, .. } => {
                let bullet_indent = (font_size * 1.5) as u32;
                let mut h = 0u32;
                for (item_idx, item) in items.iter().enumerate() {
                    let mut text_spans: Vec<StyledTextSpan> = item.spans.iter().map(|s| StyledTextSpan {
                        text: s.text.clone(), color: [0, 0, 0], bold: s.style.bold, italic: s.style.italic,
                    }).collect();
                    if item_idx + 1 < items.len() {
                        text_spans.push(StyledTextSpan {
                            text: "\n".to_string(), color: [0, 0, 0], bold: false, italic: false,
                        });
                    }
                    let opts = RenderOptions {
                        font_size,
                        line_height_factor: 1.4,
                        padding_left: bullet_indent,
                        ..Default::default()
                    };
                    h += self.renderer.measure_text_height(&text_spans, width_px, &opts);
                    // Child blocks
                    for child in &item.children {
                        h += self.measure_block_height(child);
                    }
                }
                h
            }
            Block::BlockQuote { blocks: inner } => {
                let bar_width = 4u32;
                let bar_gap = (font_size * 0.5) as u32;
                let inset = (font_size * 2.0) as u32;
                let pad_v = (font_size * 0.4) as u32;
                let child_gap = (font_size * 0.3) as u32;
                let content_width = width_px.saturating_sub(inset * 2).saturating_sub(bar_width + bar_gap);
                let mut h = pad_v * 2;
                for (i, b) in inner.iter().enumerate() {
                    // Temporarily override width for child measurement
                    let saved_width = self.width_px;
                    self.width_px = content_width;
                    h += self.measure_block_height(b);
                    self.width_px = saved_width;
                    if i + 1 < inner.len() {
                        h += child_gap;
                    }
                }
                h
            }
            Block::Table { rows, .. } => {
                let line_height = (font_size * 1.4).ceil();
                let row_h = (line_height as u32) + 9;
                (1 + rows.len() as u32) * row_h + (2 + rows.len() as u32)
            }
            Block::Image { .. } => (font_size * 10.0) as u32,
            Block::ThematicBreak => self.cell_height,
        };

        // Pad to cell_height multiple (same as ensure_block_rendered)
        if ch > 0 { ((raw_h + ch - 1) / ch) * ch } else { raw_h }
    }

    /// Whether there are unmeasured blocks remaining.
    pub fn has_unmeasured(&self, total_blocks: usize) -> bool {
        self.next_unmeasured < total_blocks
    }

    /// Measure up to `count` blocks starting at next_unmeasured.
    /// Updates block_heights, block_offsets, and total_height.
    pub fn measure_batch(&mut self, blocks: &[Block], count: usize) {
        let end = (self.next_unmeasured + count).min(blocks.len());
        for i in self.next_unmeasured..end {
            self.block_heights[i] = self.measure_block_height(&blocks[i]);
        }
        self.next_unmeasured = end;
        self.recompute_offsets(blocks);
    }

    /// Measure blocks 0..n (first viewport), update heights and offsets.
    pub fn measure_initial(&mut self, blocks: &[Block], n: usize) {
        let n = n.min(blocks.len());
        for i in 0..n {
            self.block_heights[i] = self.measure_block_height(&blocks[i]);
        }
        self.next_unmeasured = n;
        self.recompute_offsets(blocks);
    }

    /// Recompute block_offsets and total_height from block_heights.
    fn recompute_offsets(&mut self, blocks: &[Block]) {
        self.block_offsets.clear();
        let mut cursor = 0u32;
        for (i, &h) in self.block_heights.iter().enumerate() {
            self.block_offsets.push(cursor);
            cursor += h;
            if i + 1 < self.block_heights.len() {
                cursor += Self::gap_between(&blocks[i], &blocks[i + 1], self.gap, self.heading_before, self.heading_after);
            }
        }
        self.total_height = cursor;
    }

    fn render_alt_text(alt: &str, url: &str, renderer: &mut TextRenderer, width_px: u32, font_size: f32, theme: &Theme) -> DynamicImage {
        let label = if alt.is_empty() { format!("[image: {}]", url) } else { format!("[{}]", alt) };
        let spans = vec![graphics::StyledTextSpan {
            text: label,
            color: crate::theme::color_to_rgb(theme.inline_code),
            bold: false, italic: true,
        }];
        let opts = graphics::RenderOptions { font_size, line_height_factor: 1.4, ..Default::default() };
        renderer.render_styled_text(&spans, width_px, &opts).image
    }

    /// Slice parent highlight ranges into a child's byte-relative ranges.
    fn slice_highlights(highlights: &[graphics::HighlightRange], offset: usize, len: usize) -> Vec<graphics::HighlightRange> {
        highlights.iter()
            .filter_map(|hl| {
                let end = offset + len;
                if hl.byte_end > offset && hl.byte_start < end {
                    Some(graphics::HighlightRange {
                        byte_start: hl.byte_start.saturating_sub(offset),
                        byte_end: hl.byte_end.saturating_sub(offset).min(len),
                        color: hl.color, alpha: hl.alpha,
                    })
                } else { None }
            })
            .collect()
    }

    fn link_ranges_from_spans(spans: &[crate::blocks::StyledSpan], _theme: &Theme) -> Vec<graphics::LinkRange> {
        let mut links = Vec::new();
        let mut cursor = 0usize;
        let link_icon_bytes = "\u{00A0}↗".len(); // NBSP + arrow appended to link text
        for span in spans {
            let len = span.text.len();
            if span.link_url.is_some() {
                links.push(graphics::LinkRange {
                    byte_start: cursor,
                    byte_end: cursor + len + link_icon_bytes,
                    url: span.link_url.clone(),
                });
                cursor += len + link_icon_bytes;
            } else {
                cursor += len;
            }
        }
        links
    }

    fn render_block_image(
        block: &Block,
        renderer: &mut TextRenderer,
        highlighter: &Highlighter,
        width_px: u32,
        font_size: f32,
        theme: &Theme,
        highlights: Vec<graphics::HighlightRange>,
        base_dir: &std::path::Path,
    ) -> DynamicImage {
        match block {
            Block::Heading { level, spans, .. } => {
                let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                let heading_spans = vec![StyledTextSpan {
                    text,
                    color: crate::theme::color_to_rgb(theme.heading_text),
                    bold: true,
                    italic: false,
                }];
                let links = Self::link_ranges_from_spans(spans, theme);
                // Absorb heading_before as padding_top inside the image
                let scale = level.font_scale();
                let pad_top = (font_size * scale * 0.8) as u32;
                let opts = RenderOptions {
                    font_size: font_size * scale,
                    line_height_factor: 1.15,
                    padding_top: pad_top,
                    highlights,
                    links,
                    ..Default::default()
                };
                renderer.render_styled_text(&heading_spans, width_px, &opts).image
            }
            Block::Paragraph { spans } => {
                let styled = graphics::styled_spans_to_text_spans(spans, theme);
                let links = Self::link_ranges_from_spans(spans, theme);
                let opts = RenderOptions {
                    font_size,
                    line_height_factor: 1.4,
                    highlights,
                    links,
                    ..Default::default()
                };
                renderer.render_styled_text(&styled, width_px, &opts).image
            }
            Block::CodeBlock { lang, code } => {
                // Offset highlights by the language label length (prepended in render_code_block)
                let label_len = lang.as_ref().map_or(0, |l| l.len() + 2); // "lang\n\n"
                let code_highlights: Vec<graphics::HighlightRange> = highlights.into_iter()
                    .map(|mut hl| {
                        hl.byte_start += label_len;
                        hl.byte_end += label_len;
                        hl
                    })
                    .collect();
                let (code_img, _code_buffer) = renderer.render_code_block(code, lang.as_deref(), width_px, font_size, highlighter, theme, code_highlights);
                code_img
            }
            Block::BlockQuote { blocks } => {
                let bar_width = 4u32;
                let bar_gap = (font_size * 0.5) as u32;
                let inset = (font_size * 2.0) as u32;
                let content_width = width_px.saturating_sub(inset * 2).saturating_sub(bar_width + bar_gap);
                let pad_v = (font_size * 0.4) as u32;

                // Render each child block recursively
                let mut child_images: Vec<image::DynamicImage> = Vec::new();
                let mut byte_cursor = 0usize;
                for (i, b) in blocks.iter().enumerate() {
                    let child_text_len = b.text_content().len();
                    let child_hl = Self::slice_highlights(&highlights, byte_cursor, child_text_len);

                    let child_img = Self::render_block_image(
                        b, renderer, highlighter,
                        content_width, font_size, theme, child_hl, base_dir,
                    );
                    child_images.push(child_img);
                    byte_cursor += child_text_len;
                    if i + 1 < blocks.len() { byte_cursor += 1; }
                }

                // Stack children vertically with gap
                let child_gap = (font_size * 0.3) as u32;
                let content_h: u32 = child_images.iter().map(|img| img.height()).sum::<u32>()
                    + child_gap * child_images.len().saturating_sub(1) as u32;
                let total_h = content_h + pad_v * 2;

                let bg = crate::theme::color_to_rgb(theme.blockquote_bg);
                let bar_color = crate::theme::color_to_rgb(theme.blockquote_bar);
                let content_x = inset + bar_width + bar_gap;
                let bg_right = (content_x + content_width).min(width_px);

                let mut pixels = vec![0u8; (width_px * total_h * 4) as usize];

                // Fill background
                for y in 0..total_h {
                    for x in inset..bg_right {
                        let idx = ((y * width_px + x) * 4) as usize;
                        if idx + 3 < pixels.len() {
                            pixels[idx] = bg[0];
                            pixels[idx + 1] = bg[1];
                            pixels[idx + 2] = bg[2];
                            pixels[idx + 3] = 255;
                        }
                    }
                }

                // Draw vertical bar
                for y in 0..total_h {
                    for x in inset..inset + bar_width {
                        let idx = ((y * width_px + x) * 4) as usize;
                        if idx + 3 < pixels.len() {
                            pixels[idx] = bar_color[0];
                            pixels[idx + 1] = bar_color[1];
                            pixels[idx + 2] = bar_color[2];
                            pixels[idx + 3] = 255;
                        }
                    }
                }

                // Composite child images
                let mut y_off = pad_v;
                for (_ci, child_img) in child_images.iter().enumerate() {
                    let ch = child_img.height();
                    let child_rgba = child_img.as_rgba8().expect("image is rgba8");
                    let child_data = child_rgba.as_raw();
                    let cw = child_img.width();
                    for y in 0..ch {
                        for x in 0..cw.min(content_width) {
                            let src_idx = ((y * cw + x) * 4) as usize;
                            let dst_x = content_x + x;
                            let dst_y = y_off + y;
                            if dst_y >= total_h || dst_x >= width_px { continue; }
                            let dst_idx = ((dst_y * width_px + dst_x) * 4) as usize;
                            if src_idx + 3 < child_data.len() && dst_idx + 3 < pixels.len() {
                                let alpha = child_data[src_idx + 3] as u32;
                                if alpha > 0 {
                                    graphics::blend_pixel(&mut pixels[dst_idx..dst_idx+4], &child_data[src_idx..src_idx+4], alpha);
                                }
                            }
                        }
                    }
                    y_off += ch + child_gap;
                }

                let img_buf = image::ImageBuffer::from_raw(width_px, total_h, pixels).expect("blockquote buffer");
                image::DynamicImage::ImageRgba8(img_buf)
            }
            Block::List { ordered, start, items } => {
                let bullet_indent = (font_size * 1.5) as u32;
                let mut item_images: Vec<image::DynamicImage> = Vec::new();
                let mut pending_link_boxes: Vec<(usize, graphics::LinkBox)> = Vec::new();
                let mut num = start.unwrap_or(1);
                let mut byte_cursor = 0usize;

                for (item_idx, item) in items.iter().enumerate() {
                    let bullet = if *ordered {
                        let b = format!("{}. ", num); num += 1; b
                    } else { "● ".to_string() };

                    let item_text_len: usize = item.spans.iter().map(|s| s.text.len()).sum();

                    let item_highlights = Self::slice_highlights(&highlights, byte_cursor, item_text_len);

                    // Render bullet (don't store buffer — bullets aren't selectable)
                    let bullet_spans = vec![StyledTextSpan {
                        text: bullet,
                        color: crate::theme::color_to_rgb(theme.list_bullet),
                        bold: false, italic: false,
                    }];
                    let bullet_opts = RenderOptions { font_size, line_height_factor: 1.4, ..Default::default() };
                    let bullet_img = renderer.render_styled_text(&bullet_spans, bullet_indent, &bullet_opts).image;

                    // Render item text with padding_left for wrapped lines
                    let mut text_spans = graphics::styled_spans_to_text_spans(&item.spans, theme);
                    // Add trailing newline between items (not after the last one)
                    if item_idx + 1 < items.len() {
                        text_spans.push(StyledTextSpan {
                            text: "\n".to_string(),
                            color: crate::theme::color_to_rgb(theme.heading_text),
                            bold: false, italic: false,
                        });
                    }
                    let item_links = Self::link_ranges_from_spans(&item.spans, theme);
                    let text_opts = RenderOptions {
                        font_size,
                        line_height_factor: 1.4,
                        padding_left: bullet_indent,
                        highlights: item_highlights,
                        links: item_links,
                        ..Default::default()
                    };
                    let text_result = renderer.render_styled_text(&text_spans, width_px, &text_opts);
                    let text_img = text_result.image;
                    // Capture link boxes before next render call clears them
                    let item_link_boxes = std::mem::take(&mut renderer.last_link_boxes);

                    let item_img_idx = item_images.len();

                    // Composite: overlay bullet at (0, 0) on top of text image
                    let item_h = text_img.height();
                    let mut pixels = vec![0u8; (width_px * item_h * 4) as usize];

                    let text_rgba = text_img.as_rgba8().expect("image is rgba8");
                    let text_data = text_rgba.as_raw();
                    let copy_len = (width_px * item_h * 4) as usize;
                    if text_data.len() >= copy_len && pixels.len() >= copy_len {
                        pixels[..copy_len].copy_from_slice(&text_data[..copy_len]);
                    }

                    let bullet_rgba = bullet_img.as_rgba8().expect("image is rgba8");
                    let bullet_data = bullet_rgba.as_raw();
                    for by in 0..bullet_img.height().min(item_h) {
                        for bx in 0..bullet_img.width().min(bullet_indent) {
                            let src_idx = ((by * bullet_img.width() + bx) * 4) as usize;
                            let dst_idx = ((by * width_px + bx) * 4) as usize;
                            if src_idx + 3 < bullet_data.len() && dst_idx + 3 < pixels.len() {
                                let alpha = bullet_data[src_idx + 3] as u32;
                                if alpha > 0 {
                                    graphics::blend_pixel(&mut pixels[dst_idx..dst_idx+4], &bullet_data[src_idx..src_idx+4], alpha);
                                }
                            }
                        }
                    }

                    let img_buf = image::ImageBuffer::from_raw(width_px, item_h, pixels).expect("list item buffer");
                    item_images.push(image::DynamicImage::ImageRgba8(img_buf));

                    for lb in item_link_boxes {
                        pending_link_boxes.push((item_img_idx, lb));
                    }

                    byte_cursor += item_text_len;

                    // Render child blocks (nested lists, paragraphs, code blocks, etc.)
                    for child in &item.children {
                        byte_cursor += 1; // \n separator before child in text_content()

                        let child_text_len = child.text_content().len();
                        let child_highlights = Self::slice_highlights(&highlights, byte_cursor, child_text_len);

                        let child_width = width_px.saturating_sub(bullet_indent);
                        let child_img = Self::render_block_image(
                            child, renderer, highlighter,
                            child_width, font_size, theme, child_highlights, base_dir,
                        );
                        // Capture child link boxes, shift x by bullet_indent
                        let child_img_idx = item_images.len();
                        for mut lb in std::mem::take(&mut renderer.last_link_boxes) {
                            lb.x0 += bullet_indent as f32;
                            lb.x1 += bullet_indent as f32;
                            pending_link_boxes.push((child_img_idx, lb));
                        }

                        // Shift child image right by bullet_indent
                        let ch = child_img.height();
                        let cw = child_img.width();
                        let mut child_pixels = vec![0u8; (width_px * ch * 4) as usize];
                        let child_rgba = child_img.as_rgba8().expect("image is rgba8");
                        let child_data = child_rgba.as_raw();
                        for y in 0..ch {
                            let copy_w = cw.min(child_width);
                            let src_start = (y * cw * 4) as usize;
                            let dst_start = ((y * width_px + bullet_indent) * 4) as usize;
                            let bytes = (copy_w * 4) as usize;
                            if src_start + bytes <= child_data.len() && dst_start + bytes <= child_pixels.len() {
                                child_pixels[dst_start..dst_start + bytes]
                                    .copy_from_slice(&child_data[src_start..src_start + bytes]);
                            }
                        }
                        let child_buf = image::ImageBuffer::from_raw(width_px, ch, child_pixels).expect("child buffer");
                        item_images.push(image::DynamicImage::ImageRgba8(child_buf));

                        byte_cursor += child_text_len;
                    }

                    if item_idx + 1 < items.len() {
                        byte_cursor += 1; // \n separator between items
                    }
                }

                // Stack all items vertically
                let total_h: u32 = item_images.iter().map(|img| img.height()).sum();
                let mut pixels = vec![0u8; (width_px * total_h * 4) as usize];
                let mut y_offset = 0u32;
                for item_img in &item_images {
                    let ih = item_img.height();
                    let item_rgba = item_img.as_rgba8().expect("image is rgba8");
                    let item_data = item_rgba.as_raw();
                    let row_bytes = (width_px * 4) as usize;
                    for row in 0..ih {
                        let src_start = (row * width_px * 4) as usize;
                        let dst_start = ((y_offset + row) * width_px * 4) as usize;
                        if src_start + row_bytes <= item_data.len() && dst_start + row_bytes <= pixels.len() {
                            pixels[dst_start..dst_start + row_bytes].copy_from_slice(&item_data[src_start..src_start + row_bytes]);
                        }
                    }
                    y_offset += ih;
                }

                // Compute per-image y offsets for link box adjustment
                let mut img_y_offsets: Vec<u32> = Vec::with_capacity(item_images.len());
                let mut cum_y = 0u32;
                for img in &item_images {
                    img_y_offsets.push(cum_y);
                    cum_y += img.height();
                }

                // Adjust link box y coordinates and store on renderer
                renderer.last_link_boxes.clear();
                for (img_idx, mut lb) in pending_link_boxes {
                    if let Some(&y_off) = img_y_offsets.get(img_idx) {
                        lb.y0 += y_off as f32;
                        lb.y1 += y_off as f32;
                        renderer.last_link_boxes.push(lb);
                    }
                }

                let img_buf = image::ImageBuffer::from_raw(width_px, total_h, pixels).expect("list buffer");
                image::DynamicImage::ImageRgba8(img_buf)
            }
            Block::Table { headers, rows } => {
                // Compute per-cell highlights
                // text_content format: "h1 h2 h3\nrow1c1 row1c2 row1c3\n..."
                let mut cell_highlights: Vec<Vec<Vec<graphics::HighlightRange>>> = Vec::new();
                let mut byte_cursor = 0usize;

                // Header row
                let mut header_hl: Vec<Vec<graphics::HighlightRange>> = Vec::new();
                for (ci, cell) in headers.iter().enumerate() {
                    let cell_text_len: usize = cell.iter().map(|s| s.text.len()).sum();
                    let cell_hl = Self::slice_highlights(&highlights, byte_cursor, cell_text_len);
                    header_hl.push(cell_hl);
                    byte_cursor += cell_text_len;
                    if ci + 1 < headers.len() { byte_cursor += 1; } // space separator
                }
                cell_highlights.push(header_hl);
                byte_cursor += 1; // \n

                for row in rows {
                    let mut row_hl: Vec<Vec<graphics::HighlightRange>> = Vec::new();
                    for (ci, cell) in row.iter().enumerate() {
                        let cell_text_len: usize = cell.iter().map(|s| s.text.len()).sum();
                        let cell_hl = Self::slice_highlights(&highlights, byte_cursor, cell_text_len);
                        row_hl.push(cell_hl);
                        byte_cursor += cell_text_len;
                        if ci + 1 < row.len() { byte_cursor += 1; }
                    }
                    cell_highlights.push(row_hl);
                    byte_cursor += 1; // \n
                }

                let table_img = renderer.render_table(headers, rows, width_px, font_size, theme, cell_highlights);
                table_img
            }
            Block::Image { url, alt } => {
                let path = if url.starts_with("http://") || url.starts_with("https://") {
                    // Remote images not supported yet — show alt text
                    return Self::render_alt_text(alt, url, renderer, width_px, font_size, theme);
                } else {
                    let p = std::path::Path::new(url);
                    if p.is_absolute() { p.to_path_buf() } else { base_dir.join(url) }
                };

                match image::ImageReader::open(&path).and_then(|r| r.with_guessed_format()).map_err(|e| e.into()).and_then(|r| r.decode()) {
                    Ok(img) => {
                        let iw = img.width();
                        let ih = img.height();
                        if iw == 0 || ih == 0 {
                            return Self::render_alt_text(alt, url, renderer, width_px, font_size, theme);
                        }
                        // Scale to fit width, keep aspect ratio, cap total pixels
                        let max_pixels = 800_000u32;
                        let (mut final_w, mut final_h) = if iw > width_px {
                            let scale = width_px as f32 / iw as f32;
                            (width_px, (ih as f32 * scale) as u32)
                        } else {
                            (iw, ih)
                        };
                        // Further reduce if too many pixels
                        if final_w * final_h > max_pixels {
                            let scale = (max_pixels as f32 / (final_w * final_h) as f32).sqrt();
                            final_w = (final_w as f32 * scale) as u32;
                            final_h = (final_h as f32 * scale) as u32;
                        }
                        let scaled = img.resize_exact(final_w, final_h, image::imageops::FilterType::Triangle);

                        // Place on full-width canvas for consistent placeholder rendering
                        let mut canvas = vec![0u8; (width_px * final_h * 4) as usize];
                        let src = scaled.to_rgba8();
                        let src_data = src.as_raw();
                        for y in 0..final_h {
                            let src_start = (y * final_w * 4) as usize;
                            let dst_start = (y * width_px * 4) as usize;
                            let bytes = (final_w * 4) as usize;
                            if src_start + bytes <= src_data.len() && dst_start + bytes <= canvas.len() {
                                canvas[dst_start..dst_start + bytes]
                                    .copy_from_slice(&src_data[src_start..src_start + bytes]);
                            }
                        }
                        DynamicImage::ImageRgba8(
                            image::ImageBuffer::from_raw(width_px, final_h, canvas)
                                .expect("image canvas"),
                        )
                    }
                    Err(_) => Self::render_alt_text(alt, url, renderer, width_px, font_size, theme),
                }
            }
            Block::ThematicBreak => renderer.render_hr(width_px, font_size as u16, theme),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::{Block, HeadingLevel, StyledSpan, InlineStyle};
    use crate::config::ResolvedSpacing;
    use crate::theme;

    fn test_theme() -> Theme {
        theme::builtin("default").unwrap()
    }

    fn test_spacing() -> ResolvedSpacing {
        ResolvedSpacing::with_overrides(&None)
    }

    #[test]
    fn viewport_new_empty() {
        let theme = test_theme();
        let vp = Viewport::new(&[], 800, 16.0, 24, &theme, None, &test_spacing(), std::path::PathBuf::from("."));
        assert_eq!(vp.total_height, 0);
    }

    #[test]
    fn viewport_single_heading() {
        let theme = test_theme();
        let blocks = vec![Block::Heading {
            level: HeadingLevel::H1,
            spans: vec![StyledSpan {
                text: "Hello".to_string(),
                style: InlineStyle::default(),
                link_url: None,
            }],
            id: None,
        }];
        let vp = Viewport::new(&blocks, 800, 16.0, 24, &theme, None, &test_spacing(), std::path::PathBuf::from("."));
        assert!(vp.total_height > 0);
    }

}
