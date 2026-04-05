# mdv Configuration Reference

Config file location: `~/.config/mdv/config.toml`

## General

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `theme` | string | `"default"` | Color theme name |
| `font` | string | terminal font | Font family for body text |
| `font_size` | float | 75% of cell height | Font size in pixels |

### Available themes

`default`, `catppuccin-mocha`, `catppuccin-latte`, `catppuccin-frappe`, `catppuccin-macchiato`, `dracula`, `tokyo-night`, `gruvbox-dark`, `nord`

List with `mdv --list-themes`.

## Spacing

All values are multipliers of `font_size`.

```toml
[spacing]
block_gap = 0.4
heading_before = 1.5
heading_after = 0.0
```

| Key | Default | Description |
|-----|---------|-------------|
| `block_gap` | `0.4` | Gap between blocks (paragraphs, code blocks, lists, etc.) |
| `heading_before` | `1.5` | Space before a heading |
| `heading_after` | `0.0` | Space after a heading |

## Colors

All values are hex color strings (`"#RRGGBB"`).

```toml
[colors]
heading_text = "#E6E6E6"
```

| Key | Default | Description |
|-----|---------|-------------|
| `heading_text` | `#E6E6E6` | Heading text color |
| `inline_code` | `#DCB464` | Inline code text color |
| `inline_code_bg` | `#282832` | Inline code background |
| `link` | `#6496FF` | Link text color |
| `list_bullet` | `#78788C` | List bullet color |
| `hr` | `#50505F` | Horizontal rule color |
| `blockquote_bg` | `#1E1E28` | Blockquote background |
| `blockquote_bar` | `#5050C8` | Blockquote left bar |
| `code_block_border` | `#505064` | Code block border color |
| `code_block_text` | `#FFFFFF` | Code block text color |
| `table_border` | `#464655` | Table grid line color |
| `status_bar_view` | `#283C78` | Status bar background (view mode) |
| `status_bar_source` | `#1E6432` | Status bar background (source mode) |
| `status_bar_select` | `#8C5A14` | Status bar background (select mode) |
| `status_bar_fg` | `#FFFFFF` | Status bar text color |
| `search_match` | `#C8B450` | Search match highlight |
| `search_match_fg` | `#000000` | Search match text |
| `search_current` | `#FFC832` | Current search match highlight |
| `search_current_fg` | `#000000` | Current search match text |
| `search_bar` | `#28283C` | Search input bar background |
| `selection` | `#283C5A` | Selection highlight |
| `selection_fg` | `#FFFFFF` | Selection text |
| `cursor` | `#C8C8C8` | Cursor color |
| `cursor_fg` | `#000000` | Cursor text color |
| `help_bg` | `#1E1E2D` | Help popup background |
| `help_border` | `#505064` | Help popup border |
| `help_fg` | `#C8C8D2` | Help popup text |

## Key Bindings

Keys can be a single string or an array. Use lowercase for letters, `ctrl-x` for ctrl combos.

Special keys: `esc`, `enter`, `up`, `down`, `left`, `right`, `home`, `end`, `pageup`, `pagedown`.

### Normal mode

```toml
[keys.normal]
quit = ["q", "esc"]
scroll_down = ["down", "j"]
```

| Key | Default | Description |
|-----|---------|-------------|
| `quit` | `["q", "esc"]` | Quit |
| `scroll_down` | `["down", "j"]` | Scroll down one line |
| `scroll_up` | `["up", "k"]` | Scroll up one line |
| `half_page_down` | none | Half page down |
| `half_page_up` | none | Half page up |
| `page_down` | `["pagedown"]` | Page down |
| `page_up` | `["pageup"]` | Page up |
| `top` | `["g", "home"]` | Go to top |
| `bottom` | `["G", "end"]` | Go to bottom |
| `search` | `["/"]` | Open search |
| `help` | `["?"]` | Show help |
| `next_match` | `["n"]` | Next search match |
| `prev_match` | `["N"]` | Previous search match |
| `source_mode` | `["r"]` | Toggle source/raw view |

### Source mode

```toml
[keys.source]
quit = ["q", "esc"]
```

| Key | Default | Description |
|-----|---------|-------------|
| `quit` | `["q", "esc"]` | Exit source mode |
| `cursor_down` | `["down", "j"]` | Move cursor down |
| `cursor_up` | `["up", "k"]` | Move cursor up |
| `cursor_left` | `["left", "h"]` | Move cursor left |
| `cursor_right` | `["right", "l"]` | Move cursor right |
| `word_forward` | `["w"]` | Next word |
| `word_back` | `["b"]` | Previous word |
| `line_start` | `["0", "home"]` | Go to line start |
| `line_end` | `["$", "end"]` | Go to line end |
| `half_page_down` | `["pagedown"]` | Half page down |
| `half_page_up` | `["pageup"]` | Half page up |
| `top` | `["g"]` | Go to top |
| `bottom` | `["G"]` | Go to bottom |
| `select` | `["v"]` | Start selection |
| `yank` | `["y"]` | Yank (copy) selection |
| `view_mode` | `["r"]` | Back to view mode |
| `search` | `["/"]` | Open search |
| `help` | `["?"]` | Show help |
| `next_match` | `["n"]` | Next search match |
| `prev_match` | `["N"]` | Previous search match |

### Search mode

```toml
[keys.search]
cancel = ["esc"]
confirm = ["enter"]
```

| Key | Default | Description |
|-----|---------|-------------|
| `cancel` | `["esc"]` | Cancel search |
| `confirm` | `["enter"]` | Confirm search |

## CLI Arguments

```
mdv [OPTIONS] <FILE>
```

| Flag | Description |
|------|-------------|
| `--theme <NAME>` | Override color theme |
| `--list-themes` | List available themes and exit |

## Example config

```toml
theme = "catppuccin-mocha"
font = "JetBrains Mono"
font_size = 18.0

[spacing]
block_gap = 0.4
heading_before = 1.5
heading_after = 0.0

[colors]
heading_text = "#CDD6F4"
link = "#89B4FA"

[keys.normal]
quit = ["q", "esc"]
scroll_down = ["down", "j", "ctrl-d"]
```
