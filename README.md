# mdv

A graphical Markdown viewer for modern terminals.

mdv renders Markdown files with pixel-level typography using the Kitty graphics protocol. Headlines are rendered as actual large-font graphics, code blocks are syntax-highlighted, and images are displayed inline. No browser, no electron, just your terminal.

## Requirements

A graphics-capable terminal:
- [Ghostty](https://ghostty.org)
- [Kitty](https://sw.kovidgoyal.net/kitty/)
- [WezTerm](https://wezfurlong.org/wezterm/)

## Install

### From source

```sh
git clone https://github.com/ramitamir/mdv.git
cd mdv
cargo install --path .
```

### Build and run directly

```sh
cargo build --release
./target/release/mdv README.md
```

## Usage

```sh
mdv file.md                  # View a markdown file
mdv --theme dracula file.md  # Use a specific theme
mdv --list-themes            # List available themes
```

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `j` / `k` / `Arrow keys` | Scroll up / down |
| `PgDn` / `PgUp` | Half page down / up |
| `g` / `G` | Top / bottom |
| `/` | Search |
| `n` / `N` | Next / previous match |
| `r` | Raw source view |
| `v` | Select mode (in source view) |
| `e` | Open in `$EDITOR` |
| `?` | Help |
| `q` | Quit |

Mouse scroll, link clicking, and click-drag selection in source view are also supported.

### Themes

Built-in themes: `default`, `catppuccin-mocha`, `catppuccin-latte`, `catppuccin-frappe`, `catppuccin-macchiato`, `dracula`, `tokyo-night`, `gruvbox-dark`, `nord`

## Configuration

Config file: `~/.config/mdv/config.toml`

### General

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `theme` | string | `"default"` | Color theme name |
| `font` | string | terminal font | Font family for body text |
| `font_size` | float | 75% of cell height | Font size in pixels |

### Spacing

All values are multipliers of `font_size`.

| Key | Default | Description |
|-----|---------|-------------|
| `block_gap` | `0.4` | Gap between blocks |
| `heading_before` | `1.5` | Space before headings |
| `heading_after` | `0.0` | Space after headings |

### Colors

All values are hex strings (`"#RRGGBB"`). Set under `[colors]`.

**Content**

| Key | Default | Description |
|-----|---------|-------------|
| `heading_text` | `#E6E6E6` | Heading text |
| `inline_code` | `#DCB464` | Inline code text |
| `inline_code_bg` | `#282832` | Inline code background |
| `link` | `#6496FF` | Link text |
| `list_bullet` | `#78788C` | List bullet/number |
| `hr` | `#50505F` | Horizontal rule |
| `blockquote_bg` | `#1E1E28` | Blockquote background |
| `blockquote_bar` | `#5050C8` | Blockquote left bar |
| `code_block_border` | `#505064` | Code block border |
| `code_block_text` | `#FFFFFF` | Code block text |
| `table_border` | `#464655` | Table border |

**Status bar**

| Key | Default | Description |
|-----|---------|-------------|
| `status_bar_view` | `#283C78` | Mode pill — NORMAL |
| `status_bar_source` | `#1E6432` | Mode pill — SOURCE |
| `status_bar_select` | `#8C5A14` | Mode pill — SEARCH/SELECT |
| `status_bar_fg` | `#FFFFFF` | Mode pill text |
| `status_filename_bg` | `#2D2D3C` | Filename pane background |
| `status_info_bg` | `#232330` | Info pane background |
| `status_keys_bg` | `#1E1E2A` | Keys pane background |
| `status_mid_fg` | `#B4B4C3` | Filename/info text |
| `status_dim_fg` | `#78788C` | Keys/hint text |

**Search**

| Key | Default | Description |
|-----|---------|-------------|
| `search_match` | `#C8B450` | Match highlight background |
| `search_match_fg` | `#000000` | Match highlight text |
| `search_current` | `#FFC832` | Current match background |
| `search_current_fg` | `#000000` | Current match text |
| `search_bar` | `#28283C` | Search input background |

**Selection & cursor** (source view)

| Key | Default | Description |
|-----|---------|-------------|
| `selection` | `#283C5A` | Selection background |
| `selection_fg` | `#FFFFFF` | Selection text |
| `cursor` | `#C8C8C8` | Cursor background |
| `cursor_fg` | `#000000` | Cursor text |

**Help**

| Key | Default | Description |
|-----|---------|-------------|
| `help_bg` | `#1E1E2D` | Help overlay background |
| `help_border` | `#505064` | Help overlay border |
| `help_fg` | `#C8C8D2` | Help overlay text |

### Keys

Keybindings are set under `[keys.normal]`, `[keys.source]`, and `[keys.search]`. Values can be a single key or an array.

```toml
[keys.normal]
quit = "q"
scroll_down = ["j", "Down"]
scroll_up = ["k", "Up"]
page_down = "PageDown"
page_up = "PageUp"
top = "g"
bottom = "G"
search = "/"
next_match = "n"
prev_match = "N"

[keys.source]
quit = "q"
select = "v"
yank = "y"

[keys.search]
cancel = "Escape"
confirm = "Enter"
```

### Example config

```toml
theme = "catppuccin-mocha"
font = "JetBrains Mono"
font_size = 16.0

[spacing]
block_gap = 0.4
heading_before = 1.5

[colors]
heading_text = "#CDD6F4"
link = "#89B4FA"
```

## License

Apache-2.0
