# mdv — Graphical Markdown Viewer for Modern Terminals

## Project Description

A terminal-based Markdown pager that renders headlines as actual large-font graphics using the Kitty graphics protocol. Targets graphics-capable terminals only: Ghostty, Kitty, WezTerm.

## Tech Stack

- **Language:** Rust (2024 edition)
- **Markdown parsing:** `pulldown-cmark` — CommonMark event-driven parser
- **Text rendering:** `cosmic-text` — pure Rust font layout, shaping, and rasterization
- **Terminal graphics:** Kitty graphics protocol via `base64` encoding + escape sequences. Will use `ratatui-image` once the pager is built
- **TUI framework:** `ratatui` + `crossterm` (pager mode)
- **Syntax highlighting:** `syntect` (code blocks)
- **CLI:** `clap` (derive)

## Commands

```sh
cargo build                          # Build
cargo run -- <file.md>               # Run on a markdown file
cargo run --example text_sizes       # Run the graphics proof-of-concept
cargo test                           # Run tests
cargo clippy                         # Lint
cargo fmt                            # Format
```

## Architecture

```
src/
  main.rs        — CLI args, terminal setup, launch app
  app.rs         — App state, event loop, key handling, scrolling
  parser.rs      — MD file → Vec<Block> using pulldown-cmark
  blocks.rs      — Block enum definition and layout calculations
  render.rs      — Convert blocks → ratatui widgets for the viewport
  graphics.rs    — Render text → pixel images using cosmic-text
  highlight.rs   — Syntax highlighting for code blocks using syntect

examples/
  text_sizes.rs  — Graphics PoC: renders text at different sizes via Kitty protocol
```

### Rendering Strategy

- **H1–H6**: Rendered as images via cosmic-text → Kitty graphics protocol
- **Code blocks**: Syntax-highlighted ANSI text via syntect
- **Tables, paragraphs, lists, quotes**: ANSI-styled terminal text
- **Images**: Decoded and displayed via Kitty graphics protocol

## Conventions

- Use `anyhow` for error handling in application code, `thiserror` for library-style errors
- Prefer early returns over deep nesting
- Keep modules focused — one responsibility per file
- All terminal escape sequences go through dedicated functions, never inline
- Test with `test-samples/full.md` for visual verification in Ghostty
- No fallback rendering — assume graphics-capable terminal
