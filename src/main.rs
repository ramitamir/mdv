use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

mod app_new;
mod blocks;
mod config;
mod graphics;
mod highlight;
mod keys;
mod parser;
mod terminal;
mod theme;
mod viewport;

// Kept on disk for reference but not compiled while ratatui rendering is replaced:
// mod app;
// mod render;

#[derive(Parser)]
#[command(
    name = "mdv",
    about = "Graphical Markdown viewer for modern terminals",
    long_about = "Graphical Markdown viewer for modern terminals (Ghostty, Kitty, WezTerm).\n\n\
        Renders headlines as pixel images via the Kitty graphics protocol,\n\
        with syntax-highlighted code blocks, tables, and styled text.\n\n\
        Supports configurable keybindings and color themes via\n\
        ~/.config/mdv/config.toml."
)]
struct Cli {
    /// Markdown file to view
    file: Option<PathBuf>,

    /// Color theme to use
    #[arg(long)]
    theme: Option<String>,

    /// List available themes and exit
    #[arg(long)]
    list_themes: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.list_themes {
        for name in theme::builtin_names() {
            println!("{}", name);
        }
        return Ok(());
    }

    // Check for graphics-capable terminal
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    let supported = matches!(term_program.to_lowercase().as_str(),
        "ghostty" | "kitty" | "wezterm" | "iterm.app"
    );
    if !supported {
        let name = if term_program.is_empty() { "unknown".to_string() } else { term_program };
        eprintln!("mdv requires a terminal with Kitty graphics protocol support.");
        eprintln!("Supported: Ghostty, Kitty, WezTerm, iTerm2");
        eprintln!("Detected:  {}", name);
        std::process::exit(1);
    }

    let file = cli.file
        .ok_or_else(|| anyhow::anyhow!("usage: mdv <file>"))?;

    let content = std::fs::read_to_string(&file)
        .with_context(|| format!("reading {:?}", file))?;

    let cfg = config::load();
    let (theme, keys, spacing) = config::resolve(&cfg, cli.theme.as_deref());

    let blocks = parser::parse(&content);
    let filename = file.to_string_lossy().to_string();
    let base_dir = file.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    app_new::run(blocks, &content, &filename, theme, keys, spacing, cfg.font, cfg.font_size, base_dir)
}
