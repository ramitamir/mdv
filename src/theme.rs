use ratatui::style::Color;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Theme {
    // Global
    pub background: Color,
    pub text: Color,

    // Status bar mode pills
    pub status_bar_view: Color,
    pub status_bar_source: Color,
    pub status_bar_select: Color,
    pub status_bar_fg: Color,
    // Status bar pane backgrounds
    pub status_filename_bg: Color,
    pub status_info_bg: Color,
    pub status_keys_bg: Color,
    pub status_mid_fg: Color,
    pub status_dim_fg: Color,

    // Search
    pub search_match: Color,
    pub search_match_fg: Color,
    pub search_current: Color,
    pub search_current_fg: Color,
    pub search_bar: Color,

    // Selection & cursor
    pub selection: Color,
    pub selection_fg: Color,
    pub cursor: Color,
    pub cursor_fg: Color,

    // Help popup
    pub help_bg: Color,
    pub help_border: Color,
    pub help_fg: Color,

    // Content
    pub heading_text: Color,
    pub inline_code: Color,
    pub inline_code_bg: Color,
    pub link: Color,
    pub list_bullet: Color,
    pub hr: Color,
    pub blockquote_bg: Color,
    pub blockquote_bar: Color,
    pub code_block_border: Color,
    pub code_block_text: Color,
    pub table_border: Color,

    // Image search highlight
    pub image_match: Color,
    pub image_match_current: Color,
}

#[derive(Debug, Default, Deserialize)]
pub struct ColorsConfig {
    pub background: Option<String>,
    pub text: Option<String>,
    pub status_bar_view: Option<String>,
    pub status_bar_source: Option<String>,
    pub status_bar_select: Option<String>,
    pub status_bar_fg: Option<String>,
    pub status_filename_bg: Option<String>,
    pub status_info_bg: Option<String>,
    pub status_keys_bg: Option<String>,
    pub status_mid_fg: Option<String>,
    pub status_dim_fg: Option<String>,
    pub search_match: Option<String>,
    pub search_match_fg: Option<String>,
    pub search_current: Option<String>,
    pub search_current_fg: Option<String>,
    pub search_bar: Option<String>,
    pub selection: Option<String>,
    pub selection_fg: Option<String>,
    pub cursor: Option<String>,
    pub cursor_fg: Option<String>,
    pub help_bg: Option<String>,
    pub help_border: Option<String>,
    pub help_fg: Option<String>,
    pub heading_text: Option<String>,
    pub inline_code: Option<String>,
    pub inline_code_bg: Option<String>,
    pub link: Option<String>,
    pub list_bullet: Option<String>,
    pub hr: Option<String>,
    pub blockquote_bg: Option<String>,
    pub blockquote_bar: Option<String>,
    pub code_block_border: Option<String>,
    pub code_block_text: Option<String>,
    pub table_border: Option<String>,
    pub image_match: Option<String>,
    pub image_match_current: Option<String>,
}

fn parse_hex_rgb(s: &str) -> Option<[u8; 3]> {
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

pub fn parse_hex_color(s: &str) -> Option<Color> {
    let [r, g, b] = parse_hex_rgb(s)?;
    Some(Color::Rgb(r, g, b))
}

pub fn color_to_rgb(c: Color) -> [u8; 3] {
    match c {
        Color::Rgb(r, g, b) => [r, g, b],
        _ => [255, 255, 255],
    }
}

fn default_theme() -> Theme {
    Theme {
        background: Color::Rgb(22, 22, 30),
        text: Color::Rgb(200, 200, 210),
        status_bar_view: Color::Rgb(40, 60, 120),
        status_bar_source: Color::Rgb(30, 100, 50),
        status_bar_select: Color::Rgb(140, 90, 20),
        status_bar_fg: Color::White,
        status_filename_bg: Color::Rgb(45, 45, 60),
        status_info_bg: Color::Rgb(35, 35, 48),
        status_keys_bg: Color::Rgb(30, 30, 42),
        status_mid_fg: Color::Rgb(180, 180, 195),
        status_dim_fg: Color::Rgb(120, 120, 140),
        search_match: Color::Rgb(200, 180, 80),
        search_match_fg: Color::Black,
        search_current: Color::Rgb(255, 200, 50),
        search_current_fg: Color::Black,
        search_bar: Color::Rgb(40, 40, 60),
        selection: Color::Rgb(40, 60, 90),
        selection_fg: Color::White,
        cursor: Color::Rgb(200, 200, 200),
        cursor_fg: Color::Black,
        help_bg: Color::Rgb(30, 30, 45),
        help_border: Color::Rgb(80, 80, 100),
        help_fg: Color::Rgb(200, 200, 210),
        heading_text: Color::Rgb(230, 230, 230),
        inline_code: Color::Rgb(220, 180, 100),
        inline_code_bg: Color::Rgb(40, 40, 50),
        link: Color::Rgb(100, 150, 255),
        list_bullet: Color::Rgb(120, 120, 140),
        hr: Color::Rgb(80, 80, 95),
        blockquote_bg: Color::Rgb(30, 30, 40),
        blockquote_bar: Color::Rgb(80, 80, 200),
        code_block_border: Color::Rgb(80, 80, 100),
        code_block_text: Color::White,
        table_border: Color::Rgb(70, 70, 85),
        image_match: Color::Rgb(60, 55, 20),
        image_match_current: Color::Rgb(40, 38, 15),
    }
}

fn catppuccin_mocha() -> Theme {
    Theme {
        background: Color::Rgb(0x1E, 0x1E, 0x2E),         // base
        text: Color::Rgb(0xCD, 0xD6, 0xF4),               // text
        status_bar_view: Color::Rgb(0x89, 0xB4, 0xFA),    // blue
        status_bar_source: Color::Rgb(0xA6, 0xE3, 0xA1),  // green
        status_bar_select: Color::Rgb(0xFA, 0xB3, 0x87),  // peach
        status_bar_fg: Color::Rgb(0x1E, 0x1E, 0x2E),      // base
        status_filename_bg: Color::Rgb(0x31, 0x32, 0x44),  // surface0
        status_info_bg: Color::Rgb(0x28, 0x28, 0x3A),
        status_keys_bg: Color::Rgb(0x24, 0x24, 0x36),
        status_mid_fg: Color::Rgb(0xBA, 0xC2, 0xDE),       // subtext1
        status_dim_fg: Color::Rgb(0x6C, 0x70, 0x86),        // overlay0
        search_match: Color::Rgb(0xF9, 0xE2, 0xAF),       // yellow
        search_match_fg: Color::Rgb(0x1E, 0x1E, 0x2E),    // base
        search_current: Color::Rgb(0xFA, 0xB3, 0x87),     // peach
        search_current_fg: Color::Rgb(0x1E, 0x1E, 0x2E),  // base
        search_bar: Color::Rgb(0x31, 0x32, 0x44),         // surface0
        selection: Color::Rgb(0x45, 0x47, 0x5A),          // surface1
        selection_fg: Color::Rgb(0xCD, 0xD6, 0xF4),       // text
        cursor: Color::Rgb(0xBA, 0xC2, 0xDE),             // subtext1
        cursor_fg: Color::Rgb(0x1E, 0x1E, 0x2E),          // base
        help_bg: Color::Rgb(0x18, 0x18, 0x25),            // mantle
        help_border: Color::Rgb(0x6C, 0x70, 0x86),        // overlay0
        help_fg: Color::Rgb(0xCD, 0xD6, 0xF4),            // text
        heading_text: Color::Rgb(0xCD, 0xD6, 0xF4),         // text
        inline_code: Color::Rgb(0xFA, 0xB3, 0x87),        // peach
        inline_code_bg: Color::Rgb(0x31, 0x32, 0x44),     // surface0
        link: Color::Rgb(0x89, 0xB4, 0xFA),               // blue
        list_bullet: Color::Rgb(0x6C, 0x70, 0x86),        // overlay0
        hr: Color::Rgb(0x45, 0x47, 0x5A),                 // surface1
        blockquote_bg: Color::Rgb(0x18, 0x18, 0x25),      // mantle
        blockquote_bar: Color::Rgb(0xB4, 0xBE, 0xFE),     // lavender
        code_block_border: Color::Rgb(0x45, 0x47, 0x5A),  // surface1
        code_block_text: Color::Rgb(0xCD, 0xD6, 0xF4),    // text
        table_border: Color::Rgb(0x31, 0x32, 0x44),       // surface0
        image_match: Color::Rgb(0x45, 0x47, 0x5A),        // surface1
        image_match_current: Color::Rgb(0x31, 0x32, 0x44), // surface0
    }
}

fn catppuccin_latte() -> Theme {
    Theme {
        background: Color::Rgb(0xEF, 0xF1, 0xF5),         // base
        text: Color::Rgb(0x4C, 0x4F, 0x69),               // text
        status_bar_view: Color::Rgb(0x1E, 0x66, 0xF5),     // blue
        status_bar_source: Color::Rgb(0x40, 0xA0, 0x2B),   // green
        status_bar_select: Color::Rgb(0xFE, 0x64, 0x0B),    // peach
        status_bar_fg: Color::Rgb(0xEF, 0xF1, 0xF5),       // base
        status_filename_bg: Color::Rgb(0xDC, 0xE0, 0xE8),  // surface0-ish
        status_info_bg: Color::Rgb(0xE2, 0xE5, 0xEC),
        status_keys_bg: Color::Rgb(0xE6, 0xE9, 0xEF),      // mantle
        status_mid_fg: Color::Rgb(0x5C, 0x5F, 0x77),        // subtext1
        status_dim_fg: Color::Rgb(0x9C, 0xA0, 0xB0),        // overlay0
        search_match: Color::Rgb(0xDF, 0x8E, 0x1D),        // yellow
        search_match_fg: Color::Rgb(0xEF, 0xF1, 0xF5),     // base
        search_current: Color::Rgb(0xFE, 0x64, 0x0B),      // peach
        search_current_fg: Color::Rgb(0xEF, 0xF1, 0xF5),   // base
        search_bar: Color::Rgb(0xCC, 0xD0, 0xDA),          // surface0
        selection: Color::Rgb(0xBC, 0xC0, 0xCC),           // surface1
        selection_fg: Color::Rgb(0x4C, 0x4F, 0x69),        // text
        cursor: Color::Rgb(0x5C, 0x5F, 0x77),              // subtext1
        cursor_fg: Color::Rgb(0xEF, 0xF1, 0xF5),           // base
        help_bg: Color::Rgb(0xE6, 0xE9, 0xEF),             // mantle
        help_border: Color::Rgb(0x9C, 0xA0, 0xB0),         // overlay0
        help_fg: Color::Rgb(0x4C, 0x4F, 0x69),             // text
        heading_text: Color::Rgb(0x4C, 0x4F, 0x69),          // text
        inline_code: Color::Rgb(0xFE, 0x64, 0x0B),         // peach
        inline_code_bg: Color::Rgb(0xCC, 0xD0, 0xDA),      // surface0
        link: Color::Rgb(0x1E, 0x66, 0xF5),                // blue
        list_bullet: Color::Rgb(0x9C, 0xA0, 0xB0),         // overlay0
        hr: Color::Rgb(0xBC, 0xC0, 0xCC),                  // surface1
        blockquote_bg: Color::Rgb(0xE6, 0xE9, 0xEF),       // mantle
        blockquote_bar: Color::Rgb(0x72, 0x87, 0xFD),      // lavender
        code_block_border: Color::Rgb(0xBC, 0xC0, 0xCC),   // surface1
        code_block_text: Color::Rgb(0x4C, 0x4F, 0x69),     // text
        table_border: Color::Rgb(0xBC, 0xC0, 0xCC),       // surface1
        image_match: Color::Rgb(0xBC, 0xC0, 0xCC),
        image_match_current: Color::Rgb(0xCC, 0xD0, 0xDA),
    }
}

fn catppuccin_frappe() -> Theme {
    Theme {
        background: Color::Rgb(0x30, 0x34, 0x46),         // base
        text: Color::Rgb(0xC6, 0xD0, 0xF5),               // text
        status_bar_view: Color::Rgb(0x8C, 0xAA, 0xEE),
        status_bar_source: Color::Rgb(0xA6, 0xD1, 0x89),
        status_bar_select: Color::Rgb(0xEF, 0x9F, 0x76),
        status_bar_fg: Color::Rgb(0x30, 0x34, 0x46),
        status_filename_bg: Color::Rgb(0x41, 0x45, 0x59),  // surface0
        status_info_bg: Color::Rgb(0x38, 0x3C, 0x50),
        status_keys_bg: Color::Rgb(0x33, 0x37, 0x4A),
        status_mid_fg: Color::Rgb(0xB5, 0xBF, 0xE2),       // subtext1
        status_dim_fg: Color::Rgb(0x73, 0x78, 0x99),        // overlay0
        search_match: Color::Rgb(0xE5, 0xC8, 0x90),
        search_match_fg: Color::Rgb(0x30, 0x34, 0x46),
        search_current: Color::Rgb(0xEF, 0x9F, 0x76),
        search_current_fg: Color::Rgb(0x30, 0x34, 0x46),
        search_bar: Color::Rgb(0x41, 0x45, 0x59),
        selection: Color::Rgb(0x51, 0x57, 0x6D),
        selection_fg: Color::Rgb(0xC6, 0xD0, 0xF5),
        cursor: Color::Rgb(0xB5, 0xBF, 0xE2),
        cursor_fg: Color::Rgb(0x30, 0x34, 0x46),
        help_bg: Color::Rgb(0x29, 0x2C, 0x3C),
        help_border: Color::Rgb(0x73, 0x78, 0x99),
        help_fg: Color::Rgb(0xC6, 0xD0, 0xF5),
        heading_text: Color::Rgb(0xC6, 0xD0, 0xF5),
        inline_code: Color::Rgb(0xEF, 0x9F, 0x76),
        inline_code_bg: Color::Rgb(0x41, 0x45, 0x59),
        link: Color::Rgb(0x8C, 0xAA, 0xEE),
        list_bullet: Color::Rgb(0x73, 0x78, 0x99),
        hr: Color::Rgb(0x51, 0x57, 0x6D),
        blockquote_bg: Color::Rgb(0x29, 0x2C, 0x3C),
        blockquote_bar: Color::Rgb(0xBA, 0xBB, 0xF1),
        code_block_border: Color::Rgb(0x51, 0x57, 0x6D),
        code_block_text: Color::Rgb(0xC6, 0xD0, 0xF5),
        table_border: Color::Rgb(0x41, 0x45, 0x59),       // surface0
        image_match: Color::Rgb(0x51, 0x57, 0x6D),
        image_match_current: Color::Rgb(0x41, 0x45, 0x59),
    }
}

fn catppuccin_macchiato() -> Theme {
    Theme {
        background: Color::Rgb(0x24, 0x27, 0x3A),         // base
        text: Color::Rgb(0xCA, 0xD3, 0xF5),               // text
        status_bar_view: Color::Rgb(0x8A, 0xAD, 0xF4),
        status_bar_source: Color::Rgb(0xA6, 0xDA, 0x95),
        status_bar_select: Color::Rgb(0xF5, 0xA9, 0x7F),
        status_bar_fg: Color::Rgb(0x24, 0x27, 0x3A),
        status_filename_bg: Color::Rgb(0x36, 0x3A, 0x4F),  // surface0
        status_info_bg: Color::Rgb(0x2E, 0x31, 0x44),
        status_keys_bg: Color::Rgb(0x29, 0x2C, 0x3E),
        status_mid_fg: Color::Rgb(0xB8, 0xC0, 0xE0),       // subtext1
        status_dim_fg: Color::Rgb(0x6E, 0x73, 0x8D),        // overlay0
        search_match: Color::Rgb(0xEE, 0xD4, 0x9F),
        search_match_fg: Color::Rgb(0x24, 0x27, 0x3A),
        search_current: Color::Rgb(0xF5, 0xA9, 0x7F),
        search_current_fg: Color::Rgb(0x24, 0x27, 0x3A),
        search_bar: Color::Rgb(0x36, 0x3A, 0x4F),
        selection: Color::Rgb(0x49, 0x4D, 0x64),
        selection_fg: Color::Rgb(0xCA, 0xD3, 0xF5),
        cursor: Color::Rgb(0xB8, 0xC0, 0xE0),
        cursor_fg: Color::Rgb(0x24, 0x27, 0x3A),
        help_bg: Color::Rgb(0x1E, 0x20, 0x30),
        help_border: Color::Rgb(0x6E, 0x73, 0x8D),
        help_fg: Color::Rgb(0xCA, 0xD3, 0xF5),
        heading_text: Color::Rgb(0xCA, 0xD3, 0xF5),
        inline_code: Color::Rgb(0xF5, 0xA9, 0x7F),
        inline_code_bg: Color::Rgb(0x36, 0x3A, 0x4F),
        link: Color::Rgb(0x8A, 0xAD, 0xF4),
        list_bullet: Color::Rgb(0x6E, 0x73, 0x8D),
        hr: Color::Rgb(0x49, 0x4D, 0x64),
        blockquote_bg: Color::Rgb(0x1E, 0x20, 0x30),
        blockquote_bar: Color::Rgb(0xB7, 0xBD, 0xF8),
        code_block_border: Color::Rgb(0x49, 0x4D, 0x64),
        code_block_text: Color::Rgb(0xCA, 0xD3, 0xF5),
        table_border: Color::Rgb(0x36, 0x3A, 0x4F),       // surface0
        image_match: Color::Rgb(0x49, 0x4D, 0x64),
        image_match_current: Color::Rgb(0x36, 0x3A, 0x4F),
    }
}

fn dracula() -> Theme {
    Theme {
        background: Color::Rgb(0x28, 0x2A, 0x36),         // background
        text: Color::Rgb(0xF8, 0xF8, 0xF2),               // foreground
        status_bar_view: Color::Rgb(0xBD, 0x93, 0xF9),
        status_bar_source: Color::Rgb(0x50, 0xFA, 0x7B),
        status_bar_select: Color::Rgb(0xFF, 0xB8, 0x6C),
        status_bar_fg: Color::Rgb(0x28, 0x2A, 0x36),
        status_filename_bg: Color::Rgb(0x44, 0x47, 0x5A),  // current line
        status_info_bg: Color::Rgb(0x38, 0x3A, 0x4A),
        status_keys_bg: Color::Rgb(0x31, 0x33, 0x42),
        status_mid_fg: Color::Rgb(0xF8, 0xF8, 0xF2),       // foreground
        status_dim_fg: Color::Rgb(0x62, 0x72, 0xA4),        // comment
        search_match: Color::Rgb(0xF1, 0xFA, 0x8C),
        search_match_fg: Color::Rgb(0x28, 0x2A, 0x36),
        search_current: Color::Rgb(0xFF, 0xB8, 0x6C),
        search_current_fg: Color::Rgb(0x28, 0x2A, 0x36),
        search_bar: Color::Rgb(0x44, 0x47, 0x5A),
        selection: Color::Rgb(0x44, 0x47, 0x5A),
        selection_fg: Color::Rgb(0xF8, 0xF8, 0xF2),
        cursor: Color::Rgb(0xF8, 0xF8, 0xF2),
        cursor_fg: Color::Rgb(0x28, 0x2A, 0x36),
        help_bg: Color::Rgb(0x21, 0x22, 0x2C),
        help_border: Color::Rgb(0x62, 0x72, 0xA4),
        help_fg: Color::Rgb(0xF8, 0xF8, 0xF2),
        heading_text: Color::Rgb(0xF8, 0xF8, 0xF2),
        inline_code: Color::Rgb(0xFF, 0xB8, 0x6C),
        inline_code_bg: Color::Rgb(0x44, 0x47, 0x5A),
        link: Color::Rgb(0x8B, 0xE9, 0xFD),
        list_bullet: Color::Rgb(0x62, 0x72, 0xA4),
        hr: Color::Rgb(0x62, 0x72, 0xA4),
        blockquote_bg: Color::Rgb(0x21, 0x22, 0x2C),
        blockquote_bar: Color::Rgb(0xBD, 0x93, 0xF9),
        code_block_border: Color::Rgb(0x62, 0x72, 0xA4),
        code_block_text: Color::Rgb(0xF8, 0xF8, 0xF2),
        table_border: Color::Rgb(0x44, 0x47, 0x5A),       // current line
        image_match: Color::Rgb(0x44, 0x47, 0x5A),
        image_match_current: Color::Rgb(0x38, 0x3A, 0x4A),
    }
}

fn tokyo_night() -> Theme {
    Theme {
        background: Color::Rgb(0x1A, 0x1B, 0x26),         // bg_dark
        text: Color::Rgb(0xA9, 0xB1, 0xD6),               // fg
        status_bar_view: Color::Rgb(0x7A, 0xA2, 0xF7),
        status_bar_source: Color::Rgb(0x9E, 0xCE, 0x6A),
        status_bar_select: Color::Rgb(0xFF, 0x9E, 0x64),
        status_bar_fg: Color::Rgb(0x1A, 0x1B, 0x26),
        status_filename_bg: Color::Rgb(0x29, 0x2E, 0x42),  // bg_highlight
        status_info_bg: Color::Rgb(0x22, 0x26, 0x38),
        status_keys_bg: Color::Rgb(0x1E, 0x21, 0x32),
        status_mid_fg: Color::Rgb(0xA9, 0xB1, 0xD6),       // fg_dark
        status_dim_fg: Color::Rgb(0x56, 0x5F, 0x89),        // comment
        search_match: Color::Rgb(0xE0, 0xAF, 0x68),
        search_match_fg: Color::Rgb(0x1A, 0x1B, 0x26),
        search_current: Color::Rgb(0xFF, 0x9E, 0x64),
        search_current_fg: Color::Rgb(0x1A, 0x1B, 0x26),
        search_bar: Color::Rgb(0x29, 0x2E, 0x42),
        selection: Color::Rgb(0x29, 0x2E, 0x42),
        selection_fg: Color::Rgb(0xC0, 0xCA, 0xF5),
        cursor: Color::Rgb(0xC0, 0xCA, 0xF5),
        cursor_fg: Color::Rgb(0x1A, 0x1B, 0x26),
        help_bg: Color::Rgb(0x16, 0x16, 0x1E),
        help_border: Color::Rgb(0x56, 0x5F, 0x89),
        help_fg: Color::Rgb(0xC0, 0xCA, 0xF5),
        heading_text: Color::Rgb(0xC0, 0xCA, 0xF5),
        inline_code: Color::Rgb(0xFF, 0x9E, 0x64),
        inline_code_bg: Color::Rgb(0x29, 0x2E, 0x42),
        link: Color::Rgb(0x7A, 0xA2, 0xF7),
        list_bullet: Color::Rgb(0x56, 0x5F, 0x89),
        hr: Color::Rgb(0x3B, 0x40, 0x61),
        blockquote_bg: Color::Rgb(0x16, 0x16, 0x1E),
        blockquote_bar: Color::Rgb(0xBB, 0x9A, 0xF7),
        code_block_border: Color::Rgb(0x3B, 0x40, 0x61),
        code_block_text: Color::Rgb(0xC0, 0xCA, 0xF5),
        table_border: Color::Rgb(0x29, 0x2E, 0x42),       // bg_highlight
        image_match: Color::Rgb(0x29, 0x2E, 0x42),
        image_match_current: Color::Rgb(0x1F, 0x22, 0x35),
    }
}

fn gruvbox_dark() -> Theme {
    Theme {
        background: Color::Rgb(0x28, 0x28, 0x28),         // bg0
        text: Color::Rgb(0xEB, 0xDB, 0xB2),               // fg1
        status_bar_view: Color::Rgb(0x45, 0x85, 0x88),
        status_bar_source: Color::Rgb(0x98, 0x97, 0x1A),
        status_bar_select: Color::Rgb(0xD6, 0x5D, 0x0E),
        status_bar_fg: Color::Rgb(0xEB, 0xDB, 0xB2),
        status_filename_bg: Color::Rgb(0x3C, 0x38, 0x36),  // bg1
        status_info_bg: Color::Rgb(0x34, 0x30, 0x2E),
        status_keys_bg: Color::Rgb(0x2E, 0x2A, 0x28),
        status_mid_fg: Color::Rgb(0xD5, 0xC4, 0xA1),       // fg2
        status_dim_fg: Color::Rgb(0x66, 0x5C, 0x54),        // bg3
        search_match: Color::Rgb(0xD7, 0x99, 0x21),
        search_match_fg: Color::Rgb(0x28, 0x28, 0x28),
        search_current: Color::Rgb(0xFE, 0x80, 0x19),
        search_current_fg: Color::Rgb(0x28, 0x28, 0x28),
        search_bar: Color::Rgb(0x3C, 0x38, 0x36),
        selection: Color::Rgb(0x50, 0x49, 0x45),
        selection_fg: Color::Rgb(0xEB, 0xDB, 0xB2),
        cursor: Color::Rgb(0xEB, 0xDB, 0xB2),
        cursor_fg: Color::Rgb(0x28, 0x28, 0x28),
        help_bg: Color::Rgb(0x1D, 0x20, 0x21),
        help_border: Color::Rgb(0x66, 0x5C, 0x54),
        help_fg: Color::Rgb(0xEB, 0xDB, 0xB2),
        heading_text: Color::Rgb(0xEB, 0xDB, 0xB2),
        inline_code: Color::Rgb(0xFE, 0x80, 0x19),
        inline_code_bg: Color::Rgb(0x3C, 0x38, 0x36),
        link: Color::Rgb(0x83, 0xA5, 0x98),
        list_bullet: Color::Rgb(0x66, 0x5C, 0x54),
        hr: Color::Rgb(0x50, 0x49, 0x45),
        blockquote_bg: Color::Rgb(0x1D, 0x20, 0x21),
        blockquote_bar: Color::Rgb(0xB1, 0x62, 0x86),
        code_block_border: Color::Rgb(0x50, 0x49, 0x45),
        code_block_text: Color::Rgb(0xEB, 0xDB, 0xB2),
        table_border: Color::Rgb(0x3C, 0x38, 0x36),       // bg1
        image_match: Color::Rgb(0x50, 0x49, 0x45),
        image_match_current: Color::Rgb(0x3C, 0x38, 0x36),
    }
}

fn nord() -> Theme {
    Theme {
        background: Color::Rgb(0x2E, 0x34, 0x40),         // nord0
        text: Color::Rgb(0xEC, 0xEF, 0xF4),               // nord6
        status_bar_view: Color::Rgb(0x81, 0xA1, 0xC1),
        status_bar_source: Color::Rgb(0xA3, 0xBE, 0x8C),
        status_bar_select: Color::Rgb(0xD0, 0x87, 0x70),
        status_bar_fg: Color::Rgb(0x2E, 0x34, 0x40),
        status_filename_bg: Color::Rgb(0x3B, 0x42, 0x52),  // nord1
        status_info_bg: Color::Rgb(0x35, 0x3C, 0x4A),
        status_keys_bg: Color::Rgb(0x31, 0x38, 0x46),
        status_mid_fg: Color::Rgb(0xD8, 0xDE, 0xE9),       // nord4
        status_dim_fg: Color::Rgb(0x4C, 0x56, 0x6A),        // nord3
        search_match: Color::Rgb(0xEB, 0xCB, 0x8B),
        search_match_fg: Color::Rgb(0x2E, 0x34, 0x40),
        search_current: Color::Rgb(0xD0, 0x87, 0x70),
        search_current_fg: Color::Rgb(0x2E, 0x34, 0x40),
        search_bar: Color::Rgb(0x3B, 0x42, 0x52),
        selection: Color::Rgb(0x43, 0x4C, 0x5E),
        selection_fg: Color::Rgb(0xEC, 0xEF, 0xF4),
        cursor: Color::Rgb(0xEC, 0xEF, 0xF4),
        cursor_fg: Color::Rgb(0x2E, 0x34, 0x40),
        help_bg: Color::Rgb(0x2E, 0x34, 0x40),
        help_border: Color::Rgb(0x4C, 0x56, 0x6A),
        help_fg: Color::Rgb(0xEC, 0xEF, 0xF4),
        heading_text: Color::Rgb(0xEC, 0xEF, 0xF4),
        inline_code: Color::Rgb(0xD0, 0x87, 0x70),
        inline_code_bg: Color::Rgb(0x3B, 0x42, 0x52),
        link: Color::Rgb(0x88, 0xC0, 0xD0),
        list_bullet: Color::Rgb(0x4C, 0x56, 0x6A),
        hr: Color::Rgb(0x43, 0x4C, 0x5E),
        blockquote_bg: Color::Rgb(0x2E, 0x34, 0x40),
        blockquote_bar: Color::Rgb(0xB4, 0x8E, 0xAD),
        code_block_border: Color::Rgb(0x43, 0x4C, 0x5E),
        code_block_text: Color::Rgb(0xEC, 0xEF, 0xF4),
        table_border: Color::Rgb(0x3B, 0x42, 0x52),       // nord1
        image_match: Color::Rgb(0x43, 0x4C, 0x5E),
        image_match_current: Color::Rgb(0x3B, 0x42, 0x52),
    }
}

impl Theme {
    pub fn syntax_theme_name(&self) -> &str {
        let [r, g, b] = color_to_rgb(self.heading_text);
        let luminance = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
        if luminance < 128.0 {
            "InspiredGitHub"
        } else {
            "base16-ocean.dark"
        }
    }
}

/// Return a built-in theme by name. Returns None for unknown names.
const BUILTIN_THEMES: &[(&str, fn() -> Theme)] = &[
    ("default", default_theme),
    ("catppuccin-mocha", catppuccin_mocha),
    ("catppuccin-latte", catppuccin_latte),
    ("catppuccin-frappe", catppuccin_frappe),
    ("catppuccin-macchiato", catppuccin_macchiato),
    ("dracula", dracula),
    ("tokyo-night", tokyo_night),
    ("gruvbox-dark", gruvbox_dark),
    ("nord", nord),
];

pub fn builtin(name: &str) -> Option<Theme> {
    BUILTIN_THEMES.iter()
        .find(|(n, _)| *n == name)
        .map(|(_, f)| f())
}

pub fn builtin_names() -> Vec<&'static str> {
    BUILTIN_THEMES.iter().map(|(n, _)| *n).collect()
}

macro_rules! override_color {
    ($self:expr, $colors:expr, $field:ident) => {
        if let Some(ref s) = $colors.$field {
            if let Some(c) = parse_hex_color(s) {
                $self.$field = c;
            } else {
                eprintln!("mdv: invalid color for {}: {:?}", stringify!($field), s);
            }
        }
    };
}

impl Theme {
    /// Apply optional color overrides from config onto this theme.
    pub fn apply_overrides(&mut self, colors: &ColorsConfig) {
        override_color!(self, colors, background);
        override_color!(self, colors, text);
        override_color!(self, colors, status_bar_view);
        override_color!(self, colors, status_bar_source);
        override_color!(self, colors, status_bar_select);
        override_color!(self, colors, status_bar_fg);
        override_color!(self, colors, status_filename_bg);
        override_color!(self, colors, status_info_bg);
        override_color!(self, colors, status_keys_bg);
        override_color!(self, colors, status_mid_fg);
        override_color!(self, colors, status_dim_fg);
        override_color!(self, colors, search_match);
        override_color!(self, colors, search_match_fg);
        override_color!(self, colors, search_current);
        override_color!(self, colors, search_current_fg);
        override_color!(self, colors, search_bar);
        override_color!(self, colors, selection);
        override_color!(self, colors, selection_fg);
        override_color!(self, colors, cursor);
        override_color!(self, colors, cursor_fg);
        override_color!(self, colors, help_bg);
        override_color!(self, colors, help_border);
        override_color!(self, colors, help_fg);
        override_color!(self, colors, inline_code);
        override_color!(self, colors, inline_code_bg);
        override_color!(self, colors, link);
        override_color!(self, colors, list_bullet);
        override_color!(self, colors, hr);
        override_color!(self, colors, blockquote_bg);
        override_color!(self, colors, blockquote_bar);
        override_color!(self, colors, code_block_border);
        override_color!(self, colors, code_block_text);
        override_color!(self, colors, table_border);
        override_color!(self, colors, image_match);
        override_color!(self, colors, image_match_current);
        override_color!(self, colors, heading_text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_6digit() {
        assert_eq!(parse_hex_color("#FF0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_hex_color("#00ff00"), Some(Color::Rgb(0, 255, 0)));
    }

    #[test]
    fn parse_hex_invalid() {
        assert_eq!(parse_hex_color("red"), None);
        assert_eq!(parse_hex_color("#GG0000"), None);
        assert_eq!(parse_hex_color("#FF00"), None);
    }

    #[test]
    fn builtin_default_exists() {
        assert!(builtin("default").is_some());
    }

    #[test]
    fn builtin_catppuccin_mocha_exists() {
        assert!(builtin("catppuccin-mocha").is_some());
    }

    #[test]
    fn builtin_unknown_returns_none() {
        assert!(builtin("nonexistent").is_none());
    }

    #[test]
    fn builtin_names_includes_default() {
        assert!(builtin_names().contains(&"default"));
    }

    #[test]
    fn apply_overrides_changes_color() {
        let mut theme = builtin("default").unwrap();
        let original = theme.status_bar_view;
        let overrides = ColorsConfig {
            status_bar_view: Some("#FF0000".to_string()),
            ..Default::default()
        };
        theme.apply_overrides(&overrides);
        assert_eq!(theme.status_bar_view, Color::Rgb(255, 0, 0));
        assert_ne!(theme.status_bar_view, original);
    }

    #[test]
    fn apply_overrides_heading_text() {
        let mut theme = builtin("default").unwrap();
        let overrides = ColorsConfig {
            heading_text: Some("#AABBCC".to_string()),
            ..Default::default()
        };
        theme.apply_overrides(&overrides);
        assert_eq!(theme.heading_text, Color::Rgb(0xAA, 0xBB, 0xCC));
    }
}
