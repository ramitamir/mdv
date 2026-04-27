use cosmic_text::{
    Align, Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, Style, SwashCache, Weight,
};
use image::{DynamicImage, ImageBuffer, Rgba};
use std::path::Path;

/// Alpha-blend a source pixel onto a destination pixel in-place.
#[inline]
pub fn blend_pixel(dst: &mut [u8], src: &[u8], alpha: u32) {
    let inv = 255 - alpha;
    dst[0] = ((dst[0] as u32 * inv + src[0] as u32 * alpha) / 255) as u8;
    dst[1] = ((dst[1] as u32 * inv + src[1] as u32 * alpha) / 255) as u8;
    dst[2] = ((dst[2] as u32 * inv + src[2] as u32 * alpha) / 255) as u8;
    dst[3] = 255;
}

/// Bounding box for a code block's copy button.
#[derive(Clone)]
pub struct CopyButton {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

pub struct TextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    font_family: String,
    code_font_family: String,
    pub last_link_boxes: Vec<LinkBox>,
    pub last_copy_button: Option<CopyButton>,
}

/// A highlight range to draw in rendered text.
/// byte_start/byte_end are relative to the concatenated span text.
#[derive(Clone)]
pub struct HighlightRange {
    pub byte_start: usize,
    pub byte_end: usize,
    pub color: [u8; 3],
    pub alpha: u8,
}

/// Options for rendering a styled text block as a pixel image.
/// A byte range to draw as an underline (for links).
pub struct LinkRange {
    pub byte_start: usize,
    pub byte_end: usize,
    pub url: Option<String>,
}

/// A clickable link region within a rendered block image.
#[derive(Clone)]
pub struct LinkBox {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub url: String,
}

pub struct RenderOptions {
    pub font_size: f32,
    pub line_height_factor: f32,
    pub background: Option<[u8; 3]>,
    pub padding_left: u32,
    pub padding_top: u32,
    pub padding_bottom: u32,
    pub highlights: Vec<HighlightRange>,
    pub links: Vec<LinkRange>,
    pub use_code_font: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            font_size: 0.0,
            line_height_factor: 1.4,
            background: None,
            padding_left: 0,
            padding_top: 0,
            padding_bottom: 0,
            highlights: Vec::new(),
            links: Vec::new(),
            use_code_font: false,
        }
    }
}

/// Result of render_styled_text — image + retained Buffer for hit-testing.
pub struct RenderResult {
    pub image: DynamicImage,
    pub buffer: Buffer,
}

/// A status bar pane specification.
pub struct StatusPane {
    pub text: String,
    pub bg: [u8; 3],
    pub fg: [u8; 3],
    pub bold: bool,
    /// If true, this pane expands to fill remaining width after fixed panes.
    pub fill: bool,
}

/// A text span with color, weight, and style for pixel rendering.
pub struct StyledTextSpan {
    pub text: String,
    pub color: [u8; 3],
    pub bold: bool,
    pub italic: bool,
}

impl TextRenderer {
    pub fn new(font_override: Option<&str>) -> Self {
        let font_system = FontSystem::new();
        let terminal_font = detect_terminal_font().unwrap_or_else(|| "monospace".to_string());
        let code_font_family = resolve_font_name(&font_system, &terminal_font);
        let font_name = match font_override {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => terminal_font,
        };
        let font_family = resolve_font_name(&font_system, &font_name);
        Self {
            font_system,
            swash_cache: SwashCache::new(),
            font_family,
            code_font_family,
            last_link_boxes: Vec::new(),
            last_copy_button: None,
        }
    }

    /// Render rich styled text into a pixel image + retained Buffer for hit-testing.
    /// Measure text height via cosmic-text layout without rasterization.
    pub fn measure_text_height(
        &mut self,
        spans: &[StyledTextSpan],
        width_px: u32,
        opts: &RenderOptions,
    ) -> u32 {
        let font_size = opts.font_size;
        let line_height = (font_size * opts.line_height_factor).ceil();
        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        let content_width = width_px.saturating_sub(opts.padding_left);
        let active_font = if opts.use_code_font { &self.code_font_family } else { &self.font_family };

        {
            let mut borrowed = buffer.borrow_with(&mut self.font_system);
            borrowed.set_size(Some(content_width as f32), None);

            let rich: Vec<(&str, Attrs)> = spans
                .iter()
                .map(|s| {
                    let mut attrs = Attrs::new()
                        .family(Family::Name(active_font))
                        .color(Color::rgb(s.color[0], s.color[1], s.color[2]));
                    if s.bold {
                        attrs = attrs.weight(Weight::BOLD);
                    }
                    if s.italic {
                        attrs = attrs.style(Style::Italic);
                    }
                    (s.text.as_str(), attrs)
                })
                .collect();

            let default_attrs = Attrs::new().family(Family::Name(active_font));
            borrowed.set_rich_text(rich, &default_attrs, Shaping::Advanced, Some(Align::Left));
            borrowed.shape_until_scroll(true);
        }

        let line_count = buffer.layout_runs().count().max(1) as u32;
        (line_height as u32) * line_count + opts.padding_top + opts.padding_bottom
    }

    pub fn render_styled_text(
        &mut self,
        spans: &[StyledTextSpan],
        width_px: u32,
        opts: &RenderOptions,
    ) -> RenderResult {
        let font_size = opts.font_size;
        let line_height = (font_size * opts.line_height_factor).ceil();
        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        let content_width = width_px.saturating_sub(opts.padding_left);
        let active_font = if opts.use_code_font { &self.code_font_family } else { &self.font_family };

        {
            let mut borrowed = buffer.borrow_with(&mut self.font_system);
            borrowed.set_size(Some(content_width as f32), None);

            let rich: Vec<(&str, Attrs)> = spans
                .iter()
                .map(|s| {
                    let mut attrs = Attrs::new()
                        .family(Family::Name(active_font))
                        .color(Color::rgb(s.color[0], s.color[1], s.color[2]));
                    if s.bold {
                        attrs = attrs.weight(Weight::BOLD);
                    }
                    if s.italic {
                        attrs = attrs.style(Style::Italic);
                    }
                    (s.text.as_str(), attrs)
                })
                .collect();

            let default_attrs = Attrs::new().family(Family::Name(active_font));
            borrowed.set_rich_text(rich, &default_attrs, Shaping::Advanced, Some(Align::Left));
            borrowed.shape_until_scroll(true);
        }

        let line_count = buffer.layout_runs().count().max(1) as u32;
        let height_px = (line_height as u32) * line_count + opts.padding_top + opts.padding_bottom;
        let mut pixels = vec![0u8; (width_px * height_px * 4) as usize];

        // Fill background if specified
        if let Some([r, g, b]) = opts.background {
            for pixel in pixels.chunks_exact_mut(4) {
                pixel[0] = r;
                pixel[1] = g;
                pixel[2] = b;
                pixel[3] = 255;
            }
        }

        // Precompute line byte offsets for highlight/link glyph matching
        let line_byte_offsets: Vec<usize> = if !opts.highlights.is_empty() || !opts.links.is_empty() {
            let full_text: String = spans.iter().map(|s| s.text.as_str()).collect();
            let mut offsets = Vec::new();
            let mut off = 0usize;
            for line in full_text.split('\n') {
                offsets.push(off);
                off += line.len() + 1;
            }
            offsets
        } else {
            Vec::new()
        };

        if !opts.highlights.is_empty() {
            let pad_px = 2u32;

            for run in buffer.layout_runs() {
                let line_base = line_byte_offsets.get(run.line_i).copied().unwrap_or(0);
                for hl in &opts.highlights {
                    let mut min_x = width_px as f32;
                    let mut max_x = 0.0f32;
                    let mut found = false;

                    for glyph in run.glyphs.iter() {
                        let abs_byte = line_base + glyph.start;
                        if abs_byte >= hl.byte_start && abs_byte < hl.byte_end {
                            min_x = min_x.min(glyph.x + opts.padding_left as f32);
                            max_x = max_x.max(glyph.x + glyph.w + opts.padding_left as f32);
                            found = true;
                        }
                    }
                    if !found { continue; }

                    let hl_y = run.line_y - font_size + opts.padding_top as f32;
                    let x0 = (min_x - pad_px as f32).max(0.0) as u32;
                    let y0 = (hl_y).max(0.0) as u32;
                    let x1 = ((max_x + pad_px as f32) as u32).min(width_px);
                    let y1 = ((hl_y + line_height) as u32).min(height_px);

                    let a = hl.alpha as u32;
                    let src = [hl.color[0], hl.color[1], hl.color[2], 255];
                    for py in y0..y1 {
                        for px in x0..x1 {
                            let idx = ((py * width_px + px) * 4) as usize;
                            if idx + 3 < pixels.len() {
                                blend_pixel(&mut pixels[idx..idx+4], &src, a);
                            }
                        }
                    }
                }
            }
        }

        let x_offset = opts.padding_left as i32;
        let y_offset = opts.padding_top as i32;
        buffer.draw(
            &mut self.font_system,
            &mut self.swash_cache,
            Color::rgb(255, 255, 255),
            |x, y, w, h, c| {
                for row in 0..h as i32 {
                    for col in 0..w as i32 {
                        let px = x + col + x_offset;
                        let py = y + row + y_offset;
                        if px >= 0
                            && py >= 0
                            && (px as u32) < width_px
                            && (py as u32) < height_px
                        {
                            let idx = ((py as u32 * width_px + px as u32) * 4) as usize;
                            if idx + 3 < pixels.len() {
                                let alpha = c.a() as u32;
                                blend_pixel(&mut pixels[idx..idx+4], &[c.r(), c.g(), c.b(), 255], alpha);
                            }
                        }
                    }
                }
            },
        );

        // Collect link bounding boxes (no underline drawing)
        self.last_link_boxes.clear();
        if !opts.links.is_empty() {
            for run in buffer.layout_runs() {
                let line_base = line_byte_offsets.get(run.line_i).copied().unwrap_or(0);
                for ul in &opts.links {
                    let mut min_x = width_px as f32;
                    let mut max_x = 0.0f32;
                    let mut found = false;

                    for glyph in run.glyphs.iter() {
                        let abs_byte = line_base + glyph.start;
                        if abs_byte >= ul.byte_start && abs_byte < ul.byte_end {
                            min_x = min_x.min(glyph.x + opts.padding_left as f32);
                            max_x = max_x.max(glyph.x + glyph.w + opts.padding_left as f32);
                            found = true;
                        }
                    }
                    if !found { continue; }

                    let run_top = run.line_y - font_size + opts.padding_top as f32;

                    if let Some(ref url) = ul.url {
                        self.last_link_boxes.push(LinkBox {
                            x0: min_x,
                            y0: run_top,
                            x1: max_x,
                            y1: run_top + line_height,
                            url: url.clone(),
                        });
                    }
                }
            }
        }

        let img_buf: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(width_px, height_px, pixels)
                .expect("failed to create image buffer");
        RenderResult {
            image: DynamicImage::ImageRgba8(img_buf),
            buffer,
        }
    }

    /// Render a status bar as a pixel image with colored panes.
    pub fn render_status_bar(
        &mut self,
        left_panes: &[StatusPane],
        right_panes: &[StatusPane],
        fill_bg: [u8; 3],
        width_px: u32,
        height_px: u32,
        font_size: f32,
    ) -> DynamicImage {
        let mut pixels = vec![0u8; (width_px * height_px * 4) as usize];

        // Fill with base color
        for p in pixels.chunks_exact_mut(4) {
            p[0] = fill_bg[0]; p[1] = fill_bg[1]; p[2] = fill_bg[2]; p[3] = 255;
        }

        let char_w = (font_size * 0.65) as u32;
        let pad_chars = 1u32; // 1 char padding each side of text

        let all_panes: Vec<&StatusPane> = left_panes.iter().chain(right_panes.iter()).collect();

        // Sum fixed pane widths (non-fill panes)
        let fixed_total: u32 = all_panes.iter()
            .filter(|p| !p.fill)
            .map(|p| (p.text.chars().count() as u32 + pad_chars * 2) * char_w)
            .sum();
        let fill_width = width_px.saturating_sub(fixed_total);

        // Compute pane pixel positions — left panes from x=0, right panes from end
        let mut left_rects: Vec<(u32, u32, &StatusPane)> = Vec::new();
        let mut x = 0u32;
        for pane in left_panes {
            let pane_w = if pane.fill {
                fill_width
            } else {
                (pane.text.chars().count() as u32 + pad_chars * 2) * char_w
            };
            left_rects.push((x, x + pane_w, pane));
            x += pane_w;
        }

        let mut right_rects: Vec<(u32, u32, &StatusPane)> = Vec::new();
        let total_right: u32 = right_panes.iter()
            .map(|p| if p.fill { fill_width } else { (p.text.chars().count() as u32 + pad_chars * 2) * char_w })
            .sum();
        let mut rx = width_px.saturating_sub(total_right);
        for pane in right_panes {
            let pane_w = if pane.fill {
                fill_width
            } else {
                (pane.text.chars().count() as u32 + pad_chars * 2) * char_w
            };
            right_rects.push((rx, rx + pane_w, pane));
            rx += pane_w;
        }

        // Fill pane backgrounds
        for rects in [&left_rects, &right_rects] {
            for &(x0, x1, pane) in rects {
                let bg = pane.bg;
                for y in 0..height_px {
                    for x in x0..x1.min(width_px) {
                        let idx = ((y * width_px + x) * 4) as usize;
                        if idx + 3 < pixels.len() {
                            pixels[idx] = bg[0]; pixels[idx+1] = bg[1]; pixels[idx+2] = bg[2]; pixels[idx+3] = 255;
                        }
                    }
                }
            }
        }

        // Render text for each pane, clipped to pane bounds
        let text_y = (height_px as f32 - font_size) / 2.0; // vertically center
        for rects in [&left_rects, &right_rects] {
            for &(x0, x1, pane) in rects {
                let text_x = x0 + pad_chars * char_w;
                let spans = vec![StyledTextSpan {
                    text: pane.text.clone(),
                    color: pane.fg,
                    bold: pane.bold,
                    italic: false,
                }];
                // Render text with pane background so anti-aliasing blends correctly
                let opts = RenderOptions {
                    font_size,
                    line_height_factor: 1.0,
                    use_code_font: true,
                    background: Some(pane.bg),
                    ..Default::default()
                };
                let result = self.render_styled_text(&spans, width_px, &opts);
                let text_rgba = result.image.as_rgba8().expect("image is rgba8");
                let text_data = text_rgba.as_raw();
                let tw = text_rgba.width();
                let th = text_rgba.height();

                let clip_x = x1;

                // Copy (not blend) since text was rendered on the correct background
                for ty in 0..th.min(height_px) {
                    for tx in 0..tw {
                        let src_idx = ((ty * tw + tx) * 4) as usize;
                        let dx = text_x + tx;
                        let dy = text_y as u32 + ty;
                        if dx >= clip_x || dx >= width_px || dy >= height_px { continue; }
                        let dst_idx = ((dy * width_px + dx) * 4) as usize;
                        if src_idx + 3 < text_data.len() && dst_idx + 3 < pixels.len() {
                            pixels[dst_idx..dst_idx+4].copy_from_slice(&text_data[src_idx..src_idx+4]);
                        }
                    }
                }
            }
        }

        let img_buf = image::ImageBuffer::from_raw(width_px, height_px, pixels)
            .expect("status bar buffer");
        DynamicImage::ImageRgba8(img_buf)
    }

    /// Render a horizontal rule as a pixel image.
    pub fn render_hr(
        &mut self,
        width_px: u32,
        cell_height: u16,
        theme: &crate::theme::Theme,
    ) -> DynamicImage {
        let height_px = cell_height as u32;
        let mut pixels = vec![0u8; (width_px * height_px * 4) as usize];
        let color = crate::theme::color_to_rgb(theme.hr);
        let y = height_px / 2;
        for x in 0..width_px {
            let idx = ((y * width_px + x) * 4) as usize;
            if idx + 3 < pixels.len() {
                pixels[idx] = color[0];
                pixels[idx + 1] = color[1];
                pixels[idx + 2] = color[2];
                pixels[idx + 3] = 255;
            }
        }
        let img_buf: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(width_px, height_px, pixels).unwrap();
        DynamicImage::ImageRgba8(img_buf)
    }

    /// Render a syntax-highlighted code block into a DynamicImage.
    pub fn render_code_block(
        &mut self,
        code: &str,
        lang: Option<&str>,
        width_px: u32,
        base_font_size: f32,
        highlighter: &crate::highlight::Highlighter,
        theme: &crate::theme::Theme,
        highlights: Vec<HighlightRange>,
    ) -> (DynamicImage, Buffer) {
        let border_color_rgb = crate::theme::color_to_rgb(theme.code_block_border);
        let spans = match lang {
            Some(lang_str) => {
                let mut s = vec![StyledTextSpan {
                    text: format!("{}\n\n", lang_str),
                    color: border_color_rgb,
                    bold: false,
                    italic: false,
                }];
                s.extend(highlighter.highlight_to_styled_spans(code, lang_str));
                s
            }
            None => {
                vec![StyledTextSpan {
                    text: code.to_string(),
                    color: crate::theme::color_to_rgb(theme.code_block_text),
                    bold: false,
                    italic: false,
                }]
            }
        };

        let border_color = border_color_rgb;
        let radius: u32 = 20;
        let border_w: u32 = 2;
        let inset = (base_font_size * 2.0) as u32;
        let padding = (base_font_size * 0.8) as u32;

        let inner_width = width_px.saturating_sub(inset * 2);

        // Render code text
        let code_opts = RenderOptions {
            font_size: base_font_size,
            line_height_factor: 1.3,
            padding_left: padding,
            padding_top: padding / 2,
            padding_bottom: padding,
            highlights,
            use_code_font: true,
            ..Default::default()
        };
        let code_result = self.render_styled_text(&spans, inner_width, &code_opts);
        let code_img = code_result.image;
        let code_buffer = code_result.buffer;
        let code_h = code_img.height();
        let rect_w = inner_width;
        let rect_h = code_h;
        let bg = crate::theme::color_to_rgb(theme.background);
        let mut pixels = vec![0u8; (width_px * code_h * 4) as usize];

        // Fill entire image with theme background
        for p in pixels.chunks_exact_mut(4) {
            p[0] = bg[0]; p[1] = bg[1]; p[2] = bg[2]; p[3] = 255;
        }

        // Draw rounded rectangle with border
        let inner_w = rect_w.saturating_sub(border_w * 2);
        let inner_h = rect_h.saturating_sub(border_w * 2);
        let inner_r = radius.saturating_sub(border_w);

        for y in 0..rect_h {
            for x in 0..rect_w {
                if !is_inside_rounded_rect(x, y, rect_w, rect_h, radius) {
                    continue;
                }

                let in_inner = x >= border_w && y >= border_w
                    && x < border_w + inner_w && y < border_w + inner_h
                    && is_inside_rounded_rect(x - border_w, y - border_w, inner_w, inner_h, inner_r);

                let px = inset + x;
                let idx = ((y * width_px + px) * 4) as usize;
                if idx + 3 < pixels.len() {
                    if !in_inner {
                        pixels[idx] = border_color[0];
                        pixels[idx + 1] = border_color[1];
                        pixels[idx + 2] = border_color[2];
                        pixels[idx + 3] = 255;
                    }
                }
            }
        }

        // Composite code text
        let code_rgba = code_img.as_rgba8().expect("image is rgba8");
        let code_data = code_rgba.as_raw();
        for cy in 0..code_h {
            for cx in 0..code_img.width().min(inner_width) {
                let src_idx = ((cy * code_img.width() + cx) * 4) as usize;
                let dst_x = inset + cx;
                let dst_y = cy;
                let dst_idx = ((dst_y * width_px + dst_x) * 4) as usize;
                if src_idx + 3 < code_data.len() && dst_idx + 3 < pixels.len() {
                    let alpha = code_data[src_idx + 3] as u32;
                    if alpha > 0 {
                        blend_pixel(&mut pixels[dst_idx..dst_idx+4], &code_data[src_idx..src_idx+4], alpha);
                    }
                }
            }
        }

        // Draw copy icon (two overlapping rectangles) in the bottom-right corner
        let icon_size = (base_font_size * 0.7) as u32;
        let stroke = 2u32;
        let offset = icon_size / 3; // overlap offset
        let total_w = icon_size + offset;
        let total_h = icon_size + offset;
        let icon_margin = (base_font_size * 0.6) as u32;
        let icon_x = inset + rect_w - total_w - icon_margin;
        let icon_y = code_h.saturating_sub(total_h + icon_margin);
        let ic = border_color_rgb;

        let icon_r = icon_size / 4; // corner radius
        let inner_r = icon_r.saturating_sub(stroke);
        let inner_sz = icon_size.saturating_sub(stroke * 2);

        // Back rounded rectangle (bottom-right)
        for y in offset..total_h {
            for x in offset..total_w {
                let lx = x - offset;
                let ly = y - offset;
                let in_outer = is_inside_rounded_rect(lx, ly, icon_size, icon_size, icon_r);
                let in_inner = lx >= stroke && ly >= stroke
                    && lx < stroke + inner_sz && ly < stroke + inner_sz
                    && is_inside_rounded_rect(lx - stroke, ly - stroke, inner_sz, inner_sz, inner_r);
                if in_outer && !in_inner {
                    let dx = icon_x + x;
                    let dy = icon_y + y;
                    if dx < width_px && dy < code_h {
                        let idx = ((dy * width_px + dx) * 4) as usize;
                        if idx + 3 < pixels.len() {
                            pixels[idx] = ic[0]; pixels[idx+1] = ic[1]; pixels[idx+2] = ic[2]; pixels[idx+3] = 255;
                        }
                    }
                }
            }
        }
        // Front rounded rectangle (top-left)
        for y in 0..icon_size {
            for x in 0..icon_size {
                let in_outer = is_inside_rounded_rect(x, y, icon_size, icon_size, icon_r);
                let in_inner = x >= stroke && y >= stroke
                    && x < stroke + inner_sz && y < stroke + inner_sz
                    && is_inside_rounded_rect(x - stroke, y - stroke, inner_sz, inner_sz, inner_r);
                if in_outer && !in_inner {
                    let dx = icon_x + x;
                    let dy = icon_y + y;
                    if dx < width_px && dy < code_h {
                        let idx = ((dy * width_px + dx) * 4) as usize;
                        if idx + 3 < pixels.len() {
                            pixels[idx] = ic[0]; pixels[idx+1] = ic[1]; pixels[idx+2] = ic[2]; pixels[idx+3] = 255;
                        }
                    }
                }
            }
        }

        // Store copy button bounding box with generous click target
        let click_pad = base_font_size as u32;
        self.last_copy_button = Some(CopyButton {
            x0: icon_x.saturating_sub(click_pad) as f32,
            y0: icon_y.saturating_sub(click_pad) as f32,
            x1: (icon_x + total_w + click_pad) as f32,
            y1: (icon_y + total_h + click_pad) as f32,
        });

        let img_buf: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(width_px, code_h, pixels)
                .expect("failed to create code block image buffer");
        (DynamicImage::ImageRgba8(img_buf), code_buffer)
    }

    /// Render a table with grid lines and cell text into a DynamicImage.
    /// Compute per-row heights for a table by measuring each cell's wrapped text
    /// at its column width. Row height is the max of its cells.
    /// Returns (row_heights, total_height_px).
    pub fn measure_table_layout(
        &mut self,
        headers: &[Vec<crate::blocks::StyledSpan>],
        rows: &[Vec<Vec<crate::blocks::StyledSpan>>],
        width_px: u32,
        base_font_size: f32,
    ) -> (Vec<u32>, u32) {
        let num_cols = headers.len().max(1);
        let col_width_px = width_px / num_cols as u32;
        let cell_pad = (base_font_size * 0.5) as u32;
        let line_height_min = (base_font_size * 1.4).ceil() as u32;
        let min_row_h = line_height_min + cell_pad * 2;
        let cell_width = col_width_px.saturating_sub(cell_pad * 2);

        let all_rows: Vec<&[Vec<crate::blocks::StyledSpan>]> = std::iter::once(headers)
            .chain(rows.iter().map(|r| r.as_slice()))
            .collect();

        let mut row_heights = Vec::with_capacity(all_rows.len());
        for (row_idx, row_cells) in all_rows.iter().enumerate() {
            let is_header = row_idx == 0;
            let mut max_h = min_row_h;
            for cell in row_cells.iter() {
                let cell_text: String = cell.iter().map(|s| s.text.as_str()).collect();
                if cell_text.is_empty() { continue; }
                let cell_spans = vec![StyledTextSpan {
                    text: cell_text,
                    color: [0, 0, 0],
                    bold: is_header,
                    italic: false,
                }];
                let opts = RenderOptions {
                    font_size: base_font_size,
                    line_height_factor: 1.3,
                    padding_left: cell_pad,
                    padding_top: cell_pad,
                    padding_bottom: cell_pad,
                    ..Default::default()
                };
                let h = self.measure_text_height(&cell_spans, cell_width, &opts);
                if h > max_h { max_h = h; }
            }
            row_heights.push(max_h);
        }

        let grid_line = TABLE_GRID_LINE;
        let total_h: u32 = row_heights.iter().sum::<u32>()
            + grid_line * (row_heights.len() as u32 + 1);
        (row_heights, total_h)
    }

    pub fn render_table(
        &mut self,
        headers: &[Vec<crate::blocks::StyledSpan>],
        rows: &[Vec<Vec<crate::blocks::StyledSpan>>],
        width_px: u32,
        base_font_size: f32,
        theme: &crate::theme::Theme,
        cell_highlights: Vec<Vec<Vec<HighlightRange>>>,
    ) -> DynamicImage {
        let num_cols = headers.len().max(1);
        let col_width_px = width_px / num_cols as u32;
        let cell_pad = (base_font_size * 0.5) as u32;
        let grid_line = TABLE_GRID_LINE;

        let (row_heights, height_px) =
            self.measure_table_layout(headers, rows, width_px, base_font_size);

        // Cumulative y-offset for the start of each row (top grid line).
        let mut row_y_starts: Vec<u32> = Vec::with_capacity(row_heights.len() + 1);
        let mut acc = 0u32;
        for &rh in &row_heights {
            row_y_starts.push(acc);
            acc += rh + grid_line;
        }
        row_y_starts.push(acc); // bottom grid line position

        let mut pixels = vec![0u8; (width_px * height_px * 4) as usize];
        let grid_color = crate::theme::color_to_rgb(theme.table_border);
        let body_color = crate::theme::color_to_rgb(theme.text);

        // Draw horizontal grid lines between rows (and at top/bottom).
        for &y_start in &row_y_starts {
            for dy in 0..grid_line {
                let y = y_start + dy;
                if y < height_px {
                    for x in 0..width_px {
                        let idx = ((y * width_px + x) * 4) as usize;
                        if idx + 3 < pixels.len() {
                            pixels[idx] = grid_color[0];
                            pixels[idx + 1] = grid_color[1];
                            pixels[idx + 2] = grid_color[2];
                            pixels[idx + 3] = 255;
                        }
                    }
                }
            }
        }

        // Draw vertical grid lines
        for col_idx in 0..=num_cols {
            let x_start = (col_idx as u32 * col_width_px).min(width_px.saturating_sub(grid_line));
            for dx in 0..grid_line {
                let x = x_start + dx;
                if x < width_px {
                    for y in 0..height_px {
                        let idx = ((y * width_px + x) * 4) as usize;
                        if idx + 3 < pixels.len() {
                            pixels[idx] = grid_color[0];
                            pixels[idx + 1] = grid_color[1];
                            pixels[idx + 2] = grid_color[2];
                            pixels[idx + 3] = 255;
                        }
                    }
                }
            }
        }

        let img_buf: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(width_px, height_px, pixels).unwrap();
        let mut img = DynamicImage::ImageRgba8(img_buf);

        // Render cell text
        let all_rows: Vec<&[Vec<crate::blocks::StyledSpan>]> =
            std::iter::once(headers.as_ref())
                .chain(rows.iter().map(|r| r.as_slice()))
                .collect();

        for (row_idx, row_cells) in all_rows.iter().enumerate() {
            for (col_idx, cell) in row_cells.iter().enumerate() {
                let cell_text: String = cell.iter().map(|s| s.text.as_str()).collect();
                if cell_text.is_empty() {
                    continue;
                }

                let is_header = row_idx == 0;
                let cell_spans = vec![StyledTextSpan {
                    text: cell_text,
                    color: body_color,
                    bold: is_header,
                    italic: false,
                }];

                let cell_width = col_width_px.saturating_sub(cell_pad * 2);
                let cell_hl = cell_highlights.get(row_idx)
                    .and_then(|r| r.get(col_idx))
                    .cloned()
                    .unwrap_or_default();
                let cell_opts = RenderOptions {
                    font_size: base_font_size,
                    line_height_factor: 1.3,
                    padding_left: cell_pad,
                    padding_top: cell_pad,
                    padding_bottom: cell_pad,
                    highlights: cell_hl,
                    ..Default::default()
                };
                let cell_img = self.render_styled_text(&cell_spans, cell_width, &cell_opts).image;

                let dest_x = col_idx as u32 * col_width_px + grid_line;
                let dest_y = row_y_starts[row_idx] + grid_line;
                image::imageops::overlay(&mut img, &cell_img, dest_x as i64, dest_y as i64);
            }
        }

        img
    }

}

const TABLE_GRID_LINE: u32 = 2;

/// Check if a point (x, y) is inside a rounded rectangle of size (w, h) with corner radius r.
fn is_inside_rounded_rect(x: u32, y: u32, w: u32, h: u32, r: u32) -> bool {
    if w == 0 || h == 0 { return false; }
    let r = r.min(w / 2).min(h / 2);
    // Check each corner
    if x < r && y < r {
        // Top-left corner
        let dx = r - x;
        let dy = r - y;
        return dx * dx + dy * dy <= r * r;
    }
    if x >= w - r && y < r {
        // Top-right corner
        let dx = x - (w - r - 1);
        let dy = r - y;
        return dx * dx + dy * dy <= r * r;
    }
    if x < r && y >= h - r {
        // Bottom-left corner
        let dx = r - x;
        let dy = y - (h - r - 1);
        return dx * dx + dy * dy <= r * r;
    }
    if x >= w - r && y >= h - r {
        // Bottom-right corner
        let dx = x - (w - r - 1);
        let dy = y - (h - r - 1);
        return dx * dx + dy * dy <= r * r;
    }
    true
}

/// Convert a slice of `blocks::StyledSpan` to `StyledTextSpan` using theme colors.
/// Shared by `render_paragraph`, `render_blockquote`, and list rendering.
pub fn styled_spans_to_text_spans(
    spans: &[crate::blocks::StyledSpan],
    theme: &crate::theme::Theme,
) -> Vec<StyledTextSpan> {
    let mut result = Vec::new();
    for s in spans {
        let color = if s.style.code {
            crate::theme::color_to_rgb(theme.inline_code)
        } else if s.link_url.is_some() {
            crate::theme::color_to_rgb(theme.link)
        } else {
            crate::theme::color_to_rgb(theme.text)
        };
        let text = if s.link_url.is_some() {
            format!("{}\u{00A0}↗", s.text)
        } else {
            s.text.clone()
        };
        result.push(StyledTextSpan { text, color, bold: s.style.bold, italic: s.style.italic });
    }
    result
}

fn detect_terminal_font() -> Option<String> {
    let term_program = std::env::var("TERM_PROGRAM").ok()?;
    match term_program.as_str() {
        "ghostty" => {
            let config = dirs_path("ghostty/config")
                .or_else(|| dirs_path("com.mitchellh.ghostty/config"))?;
            let contents = std::fs::read_to_string(config).ok()?;
            for line in contents.lines() {
                let line = line.trim();
                if line.starts_with("font-family")
                    && let Some(val) = line.split('=').nth(1)
                {
                    let font = val.trim().to_string();
                    if !font.is_empty() {
                        return Some(font);
                    }
                }
            }
            None
        }
        "kitty" => {
            let config = dirs_path("kitty/kitty.conf")?;
            let contents = std::fs::read_to_string(config).ok()?;
            for line in contents.lines() {
                let line = line.trim();
                if line.starts_with("font_family")
                    && let Some((_, val)) = line.split_once(char::is_whitespace)
                {
                    let font = val.trim().to_string();
                    if !font.is_empty() {
                        return Some(font);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn dirs_path(relative: &str) -> Option<std::path::PathBuf> {
    // Platform-specific config dir (~/Library/Application Support on macOS)
    if let Some(config_dir) = dirs::config_dir() {
        let path = config_dir.join(relative);
        if path.exists() {
            return Some(path);
        }
    }
    // Fall back to ~/.config (common on Linux, also used by some macOS apps)
    if let Ok(home) = std::env::var("HOME") {
        let path = Path::new(&home).join(".config").join(relative);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn resolve_font_name(font_system: &FontSystem, terminal_name: &str) -> String {
    let db = font_system.db();

    for face in db.faces() {
        for family in &face.families {
            if family.0.eq_ignore_ascii_case(terminal_name) {
                return family.0.clone();
            }
        }
    }

    let stripped = terminal_name
        .to_lowercase()
        .replace(" nerd font mono", "")
        .replace(" nerd font", "")
        .replace(" nf", "")
        .replace(" mono", "");
    let no_spaces = stripped.replace(' ', "");

    let mut best_match: Option<String> = None;
    for face in db.faces() {
        for family in &face.families {
            let fam_lower = family.0.to_lowercase();
            let fam_no_spaces = fam_lower.replace(' ', "");

            if fam_lower == stripped || fam_no_spaces == no_spaces {
                return family.0.clone();
            }

            if fam_no_spaces.starts_with(&no_spaces) && best_match.is_none() {
                best_match = Some(family.0.clone());
            }
        }
    }

    best_match.unwrap_or_else(|| terminal_name.to_string())
}
