use base64::{Engine as _, engine::general_purpose::STANDARD};
use cosmic_text::{
    Align, Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache, Weight,
};
use crossterm::terminal;
use std::io::{self, Write};
use std::path::Path;

/// Detect the terminal's font family by reading its config file.
/// Currently supports Ghostty and Kitty.
fn detect_terminal_font() -> Option<String> {
    let term_program = std::env::var("TERM_PROGRAM").ok()?;
    match term_program.as_str() {
        "ghostty" => {
            let config = dirs_path("ghostty/config")?;
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
    let home = std::env::var("HOME").ok()?;
    let path = Path::new(&home).join(".config").join(relative);
    if path.exists() { Some(path) } else { None }
}

/// Resolve a terminal font name (e.g. "JetBrainsMono Nerd Font") to a fontdb family name
/// (e.g. "JetBrains Mono"). Falls back to the original name if no match found.
fn resolve_font_name(font_system: &FontSystem, terminal_name: &str) -> String {
    let db = font_system.db();
    let lower = terminal_name.to_lowercase();

    // Exact match first
    for face in db.faces() {
        for family in &face.families {
            if family.0.eq_ignore_ascii_case(terminal_name) {
                return family.0.clone();
            }
        }
    }

    // Strip common suffixes like "Nerd Font", "Nerd Font Mono", "NF" and try base name
    let stripped = lower
        .replace(" nerd font mono", "")
        .replace(" nerd font", "")
        .replace(" nf", "")
        .replace(" mono", "");

    // Also try removing spaces within the name (e.g. "JetBrainsMono" → "jetbrainsmono")
    let no_spaces = stripped.replace(' ', "");

    let mut best_match: Option<String> = None;
    for face in db.faces() {
        for family in &face.families {
            let fam_lower = family.0.to_lowercase();
            let fam_no_spaces = fam_lower.replace(' ', "");

            if fam_lower == stripped || fam_no_spaces == no_spaces {
                return family.0.clone();
            }

            // Partial: if the fontdb name starts with our stripped name
            if fam_no_spaces.starts_with(&no_spaces) && best_match.is_none() {
                best_match = Some(family.0.clone());
            }
        }
    }

    best_match.unwrap_or_else(|| terminal_name.to_string())
}

/// Render text into an RGBA pixel buffer using cosmic-text
fn render_text(
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    text: &str,
    font_size: f32,
    width: u32,
    text_color: [u8; 3],
    font_family: &str,
) -> (Vec<u8>, u32, u32) {
    let line_height = (font_size * 1.5).ceil();
    let metrics = Metrics::new(font_size, line_height);
    let mut buffer = Buffer::new(font_system, metrics);

    {
        let mut borrowed = buffer.borrow_with(font_system);
        borrowed.set_size(Some(width as f32), None);
        let attrs = Attrs::new()
            .family(Family::Name(font_family))
            .weight(Weight::BOLD);
        borrowed.set_text(text, &attrs, Shaping::Advanced, Some(Align::Left));
        borrowed.shape_until_scroll(true);
    }

    let height = line_height as u32 + 8; // small padding
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    let color = Color::rgb(text_color[0], text_color[1], text_color[2]);

    buffer.draw(font_system, swash_cache, color, |x, y, w, h, c| {
        // Draw each glyph region into our pixel buffer
        for row in 0..h as i32 {
            for col in 0..w as i32 {
                let px = x + col;
                let py = y + row;
                if px >= 0 && py >= 0 && (px as u32) < width && (py as u32) < height {
                    let idx = ((py as u32 * width + px as u32) * 4) as usize;
                    if idx + 3 < pixels.len() {
                        let alpha = c.a() as u32;
                        let inv_alpha = 255 - alpha;
                        // Alpha blend over existing pixel
                        pixels[idx] =
                            ((pixels[idx] as u32 * inv_alpha + c.r() as u32 * alpha) / 255) as u8;
                        pixels[idx + 1] = ((pixels[idx + 1] as u32 * inv_alpha
                            + c.g() as u32 * alpha)
                            / 255) as u8;
                        pixels[idx + 2] = ((pixels[idx + 2] as u32 * inv_alpha
                            + c.b() as u32 * alpha)
                            / 255) as u8;
                        pixels[idx + 3] =
                            (pixels[idx + 3] as u32 + alpha).min(255) as u8;
                    }
                }
            }
        }
    });

    (pixels, width, height)
}

/// Encode RGBA pixels as PNG bytes
fn encode_png(pixels: &[u8], width: u32, height: u32) -> Vec<u8> {
    let img: image::ImageBuffer<image::Rgba<u8>, _> =
        image::ImageBuffer::from_raw(width, height, pixels.to_vec())
            .expect("failed to create image buffer");
    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    image::ImageEncoder::write_image(
        encoder,
        img.as_raw(),
        width,
        height,
        image::ExtendedColorType::Rgba8,
    )
    .expect("failed to encode PNG");
    png_bytes
}

/// Send a PNG image to the terminal via the Kitty graphics protocol
fn kitty_display_png(png_data: &[u8]) {
    let encoded = STANDARD.encode(png_data);
    let stdout = io::stdout();
    let mut out = stdout.lock();

    // Chunk into 4096-byte pieces
    let chunks: Vec<&str> = encoded
        .as_bytes()
        .chunks(4096)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == chunks.len() - 1;
        if i == 0 && is_last {
            // Single chunk
            write!(out, "\x1b_Ga=T,f=100,t=d;{}\x1b\\", chunk).unwrap();
        } else if i == 0 {
            // First chunk of multi
            write!(out, "\x1b_Ga=T,f=100,t=d,m=1;{}\x1b\\", chunk).unwrap();
        } else if is_last {
            // Last chunk
            write!(out, "\x1b_Gm=0;{}\x1b\\", chunk).unwrap();
        } else {
            // Middle chunk
            write!(out, "\x1b_Gm=1;{}\x1b\\", chunk).unwrap();
        }
    }
    out.flush().unwrap();
}

fn main() {
    // Get terminal width in columns, estimate pixel width
    let (cols, _rows) = terminal::size().expect("failed to get terminal size");
    // Assume ~8px per column as a reasonable default for most terminal fonts
    let pixel_width = (cols as u32) * 8;

    let mut font_system = FontSystem::new();

    let terminal_font = detect_terminal_font().unwrap_or_else(|| "monospace".to_string());
    let font_family = resolve_font_name(&font_system, &terminal_font);
    println!(
        "Terminal: {} columns, estimated {}px wide\nFont: {} (config: {})\n",
        cols, pixel_width, font_family, terminal_font
    );
    let mut swash_cache = SwashCache::new();

    let headings = [
        (1, 48.0, "Heading 1 — The Main Title", [230, 230, 250]),
        (2, 40.0, "Heading 2 — Section Header", [200, 200, 240]),
        (3, 32.0, "Heading 3 — Subsection", [180, 180, 230]),
        (4, 26.0, "Heading 4 — Minor Section", [160, 170, 220]),
        (5, 22.0, "Heading 5 — Detail Level", [140, 160, 210]),
        (6, 18.0, "Heading 6 — Fine Print", [130, 150, 200]),
    ];

    for (level, size, text, color) in &headings {
        let label = format!("H{} ({}px)", level, *size as u32);
        print!("{:<14}", label);

        let (pixels, w, h) = render_text(
            &mut font_system,
            &mut swash_cache,
            text,
            *size,
            pixel_width,
            *color,
            &font_family,
        );

        let png = encode_png(&pixels, w, h);
        kitty_display_png(&png);
        println!(); // newline after image
    }

    println!("\nDone! If you see 6 lines of graphical text at different sizes, the pipeline works.");
}
