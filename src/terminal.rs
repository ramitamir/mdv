use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use crossterm::{
    cursor,
    execute, queue,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};

pub struct Terminal {
    stdout: io::BufWriter<io::Stdout>,
    pub cols: u16,
    pub rows: u16,
    pub cell_width: u16,
    pub cell_height: u16,
}

/// Max image rows before needing to split into chunks (limited by diacritic count).
pub const MAX_IMAGE_ROWS: u32 = 10;

fn diacritic(n: u32) -> char {
    const DIACRITICS: [u32; 57] = [
        0x0305, 0x030D, 0x030E, 0x0310, 0x0312, 0x033D, 0x033E, 0x033F,
        0x0346, 0x034A, 0x034B, 0x034C, 0x0350, 0x0351, 0x0352, 0x0357,
        0x035B, 0x0363, 0x0364, 0x0365, 0x0366, 0x0367, 0x0368, 0x0369,
        0x036A, 0x036B, 0x036C, 0x036D, 0x036E, 0x036F, 0x0483, 0x0484,
        0x0485, 0x0486, 0x0487, 0x0592, 0x0593, 0x0594, 0x0595, 0x0597,
        0x0598, 0x0599, 0x059A, 0x059B, 0x059C, 0x059D, 0x059E, 0x059F,
        0x05A0, 0x05A1, 0x05A2, 0x05A3, 0x05A4, 0x05A5, 0x05A6, 0x05A7,
        0x05A8,
    ];
    let idx = (n as usize) % DIACRITICS.len();
    char::from_u32(DIACRITICS[idx]).unwrap_or('\u{0305}')
}

impl Terminal {
    pub fn new() -> Result<Self> {
        terminal::enable_raw_mode().context("enable raw mode")?;
        let mut stdout = io::BufWriter::with_capacity(256 * 1024, io::stdout());
        execute!(
            stdout,
            EnterAlternateScreen,
            cursor::Hide
        )
        .context("terminal setup")?;
        stdout.write_all(b"\x1b[?1003h\x1b[?1006h")?;
        stdout.flush()?;

        let (cols, rows) = terminal::size().context("get terminal size")?;
        let (cell_width, cell_height) = detect_cell_size();

        Ok(Self {
            stdout,
            cols,
            rows,
            cell_width,
            cell_height,
        })
    }

    pub fn stdout_mut(&mut self) -> &mut io::BufWriter<io::Stdout> {
        &mut self.stdout
    }

    pub fn content_width_px(&self) -> u32 {
        self.cols as u32 * self.cell_width as u32
    }

    pub fn content_height_px(&self) -> u32 {
        self.content_rows() as u32 * self.cell_height as u32
    }

    pub fn content_rows(&self) -> u16 {
        self.rows.saturating_sub(2) // 2 rows for pixel-rendered status bar
    }

    /// Transmit an image with virtual placement (U=1).
    pub fn transmit_virtual(&mut self, image_id: u32, rgba: &[u8], width: u32, height: u32) -> Result<()> {
        let mut png_bytes = Vec::new();
        {
            use image::codecs::png::{PngEncoder, CompressionType, FilterType};
            use image::ImageEncoder;
            PngEncoder::new_with_quality(&mut png_bytes, CompressionType::Fast, FilterType::Sub)
                .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
                .context("PNG encode")?;
        }

        const CHUNK_SIZE: usize = 3072;
        let chunks: Vec<&[u8]> = png_bytes.chunks(CHUNK_SIZE).collect();
        let total = chunks.len();

        for (i, chunk) in chunks.iter().enumerate() {
            let encoded = STANDARD.encode(chunk);
            let more = if i + 1 < total { 1u8 } else { 0u8 };

            if i == 0 {
                write!(
                    self.stdout,
                    "\x1b_Gi={image_id},a=T,U=1,f=100,q=2,s={width},v={height},m={more};{encoded}\x1b\\"
                )?;
            } else {
                write!(
                    self.stdout,
                    "\x1b_Gm={more};{encoded}\x1b\\"
                )?;
            }
        }
        self.stdout.flush().context("transmit flush")
    }

    pub fn print_placeholder_row(&mut self, image_id: u32, row_in_image: u32, num_cols: u16) -> Result<()> {
        let id_bytes = image_id.to_be_bytes();
        let id_extra = id_bytes[0] as u32;
        let r = id_bytes[1];
        let g = id_bytes[2];
        let b = id_bytes[3];

        let row_diac = diacritic(row_in_image);
        let col0_diac = diacritic(0);
        let extra_diac = diacritic(id_extra);

        let mut row = String::with_capacity(num_cols as usize * 4 + 30);
        use std::fmt::Write as FmtWrite;
        write!(row, "\x1b[38;2;{r};{g};{b}m\u{10EEEE}{row_diac}{col0_diac}{extra_diac}").unwrap();
        for _ in 1..num_cols {
            row.push('\u{10EEEE}');
        }
        row.push_str("\x1b[39m");
        self.stdout.write_all(row.as_bytes())?;
        Ok(())
    }

    pub fn delete_image(&mut self, image_id: u32) -> Result<()> {
        write!(self.stdout, "\x1b_Ga=d,d=I,i={};\x1b\\", image_id)?;
        Ok(())
    }

    pub fn delete_all_images(&mut self) -> Result<()> {
        write!(self.stdout, "\x1b_Ga=d;\x1b\\")?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.stdout.flush().context("flush")
    }

    pub fn set_pointer_cursor(&mut self) -> Result<()> {
        write!(self.stdout, "\x1b]22;pointer\x07")?;
        Ok(())
    }

    pub fn set_default_cursor(&mut self) -> Result<()> {
        write!(self.stdout, "\x1b]22;default\x07")?;
        Ok(())
    }

    pub fn copy_to_clipboard(&mut self, text: &str) -> Result<()> {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(text);
        write!(self.stdout, "\x1b]52;c;{}\x07", encoded)?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Clear the content area (all rows except status bar).
    pub fn clear_content(&mut self) -> Result<()> {
        for row in 0..self.content_rows() {
            queue!(self.stdout, cursor::MoveTo(0, row))?;
            write!(self.stdout, "\x1b[2K")?; // clear entire line
        }
        Ok(())
    }


    pub fn update_size(&mut self) {
        if let Ok((cols, rows)) = terminal::size() {
            self.cols = cols;
            self.rows = rows;
        }
        let (cw, ch) = detect_cell_size();
        self.cell_width = cw;
        self.cell_height = ch;
    }

    pub fn cleanup(&mut self) -> Result<()> {
        self.delete_all_images()?;
        // Disable mouse button tracking
        self.stdout.write_all(b"\x1b[?1003l\x1b[?1006l")?;
        execute!(
            self.stdout,
            cursor::Show,
            LeaveAlternateScreen,
        )?;
        terminal::disable_raw_mode()?;
        Ok(())
    }
}

fn detect_cell_size() -> (u16, u16) {
    if let Ok(ws) = terminal::window_size() {
        if ws.width > 0 && ws.height > 0 && ws.columns > 0 && ws.rows > 0 {
            return (ws.width / ws.columns, ws.height / ws.rows);
        }
    }
    // Warn once: fallback dimensions will desync image alignment with text cells.
    use std::sync::atomic::{AtomicBool, Ordering};
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "mdv: warning — terminal did not report pixel dimensions; \
             falling back to 10x20 cell size. Image alignment may be off. \
             Try a graphics-capable terminal (Ghostty, Kitty, WezTerm)."
        );
    }
    (10, 20)
}
