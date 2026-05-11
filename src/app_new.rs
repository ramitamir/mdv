use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use std::collections::HashSet;
use std::io::Write;
use std::process::Command;
use std::time::Duration;
use crate::blocks::Block;
use crate::config::ResolvedKeys;
use crate::terminal::Terminal;
use crate::theme::Theme;
use crate::viewport::Viewport;

#[derive(PartialEq)]
enum Mode {
    Normal,
    Search,
}

struct SearchMatch {
    block_idx: usize,
    byte_start: usize,
    byte_end: usize,
}

pub fn run(mut blocks: Vec<Block>, raw_content: &str, filename: &str, theme: Theme, keys: ResolvedKeys, spacing: crate::config::ResolvedSpacing, font: Option<String>, font_size_override: Option<f32>, base_dir: std::path::PathBuf) -> Result<()> {
    let mut raw_content = raw_content.to_string();
    let mut term = Terminal::new()?;
    let font_size = font_size_override.unwrap_or(term.cell_height as f32 * 0.75);
    let cell_h = term.cell_height;
    let cell_w = term.cell_width;
    let margin_cols: u16 = 3;
    let margin_px = margin_cols as u32 * term.cell_width as u32;
    let width_px = term.content_width_px().saturating_sub(margin_px * 2);

    let mut viewport = Viewport::new(&blocks, width_px, font_size, cell_h, &theme, font.as_deref(), &spacing, base_dir.clone());

    // Measure first viewport of blocks synchronously for accurate layout
    let viewport_height_px = term.content_rows() as u32 * cell_h as u32;
    let first_viewport_blocks = viewport.block_offsets
        .iter()
        .position(|&off| off >= viewport_height_px)
        .unwrap_or(blocks.len())
        .min(blocks.len());
    viewport.measure_initial(&blocks, first_viewport_blocks + 1);

    let mut scroll_row: u32 = 0;
    let scroll_step: u32 = 1;
    let mut scroll_count: u32 = 0; // for periodic eviction

    // Track which block images are transmitted to the terminal
    let mut transmitted: HashSet<usize> = HashSet::new();
    let mut cursor_on_link = false;
    let mut hover_url: Option<String> = None;

    // Search state
    let mut mode = Mode::Normal;
    let mut search_input = String::new();
    let mut search_query = String::new();
    let mut matches: Vec<SearchMatch> = Vec::new();
    let mut current_match: usize = 0;

    set_term_bg(&mut term, &theme)?;

    // Initial render
    redraw(
        &mut term, &mut viewport, &blocks, &theme,
        &mut transmitted, scroll_row, cell_h, margin_cols,
    )?;
    draw_status(&mut term, &mut viewport, filename, scroll_row, &theme, hover_url.as_deref())?;

    let mut last_event_at = std::time::Instant::now();

    loop {
        // Background measurement during idle time
        if viewport.has_unmeasured(blocks.len()) {
            // Record which block is first visible before measurement
            let scroll_px = scroll_row as u64 * cell_h as u64;
            let first_visible = viewport.block_offsets
                .iter()
                .rposition(|&off| (off as u64) <= scroll_px)
                .unwrap_or(0);

            viewport.measure_batch(&blocks, 10);

            // Adjust scroll to keep the same content visible
            if first_visible < viewport.block_offsets.len() {
                scroll_row = viewport.block_offsets[first_visible] / cell_h as u32;
            }
        }

        let total_rows = pixel_to_rows(viewport.total_height, cell_h);
        let content_rows = term.content_rows() as u32;
        let max_scroll = total_rows.saturating_sub(content_rows);

        // Defensive: if a height correction (from any path) shrank the doc, the
        // user's scroll may now point past the end. Clamp and force a redraw so
        // the viewport refills instead of showing all-blank rows.
        if scroll_row > max_scroll {
            scroll_row = max_scroll;
            redraw(&mut term, &mut viewport, &blocks, &theme,
                   &mut transmitted, scroll_row, cell_h, margin_cols)?;
            draw_status(&mut term, &mut viewport, filename, scroll_row, &theme, hover_url.as_deref())?;
        }

        // Poll for input. If user has been idle, do background prefetch.
        let idle_ms = last_event_at.elapsed().as_millis();
        let scroll_px = scroll_row * cell_h as u32;
        let vp_px = term.content_height_px();
        let prefetch_ready = idle_ms >= 100
            && next_prefetch_block(&viewport, &transmitted, scroll_px, vp_px).is_some();

        let timeout = if prefetch_ready {
            Duration::from_millis(50)
        } else if viewport.has_unmeasured(blocks.len()) {
            Duration::from_millis(10)
        } else {
            Duration::from_secs(3600)
        };
        if !event::poll(timeout)? {
            // Timeout fired. Prefetch one adjacent block if idle long enough.
            if idle_ms >= 100 {
                if let Some(idx) = next_prefetch_block(&viewport, &transmitted, scroll_px, vp_px) {
                    let img = viewport.ensure_block_rendered(idx, &blocks, &theme);
                    let rgba = img.as_rgba8().expect("rgba8");
                    transmit_block_image(&mut term, idx, rgba.as_raw(), img.width(), img.height(), cell_h)?;
                    transmitted.insert(idx);

                    // If rendering revealed the estimated height was wrong, the safety
                    // net invalidates blocks below this one. Force a redraw so the screen
                    // reflects new positions (otherwise visible-but-invalidated blocks
                    // appear blank on next scroll).
                    if let Some(corrected_idx) = viewport.height_corrected_at.take() {
                        for j in (corrected_idx + 1)..blocks.len() {
                            if transmitted.remove(&j) {
                                for c in 0..20 { let _ = term.delete_image(block_image_id(j, c)); }
                            }
                        }
                        let new_max = pixel_to_rows(viewport.total_height, cell_h)
                            .saturating_sub(term.content_rows() as u32);
                        scroll_row = scroll_row.min(new_max);
                        redraw(&mut term, &mut viewport, &blocks, &theme,
                               &mut transmitted, scroll_row, cell_h, margin_cols)?;
                        draw_status(&mut term, &mut viewport, filename, scroll_row, &theme, hover_url.as_deref())?;
                    }
                }
            }
            continue;
        }
        let first = event::read()?;
        last_event_at = std::time::Instant::now();
        let mut quit = false;
        let mut changed = false;
        let mut status_msg: Option<String> = None;
        let old_scroll = scroll_row;

        // Handle search mode input
        if mode == Mode::Search {
            if let Event::Key(key) = &first {
                match key.code {
                    KeyCode::Esc => {
                        mode = Mode::Normal;
                        search_input.clear();
                        if !matches.is_empty() {
                            draw_status_search(&mut term, &mut viewport, &search_query, scroll_row, &theme, current_match, &matches)?;
                        } else {
                            draw_status(&mut term, &mut viewport, filename, scroll_row, &theme, hover_url.as_deref())?;
                        }
                    }
                    KeyCode::Enter => {
                        mode = Mode::Normal;
                        if !search_input.is_empty() {
                            search_query = search_input.clone();
                            matches.clear();
                            current_match = 0;
                            let query_lower = search_query.to_lowercase();
                            let query_len = query_lower.len();
                            for (i, block) in blocks.iter().enumerate() {
                                let text = block.text_content();
                                let text_lower = text.to_lowercase();
                                let mut start = 0;
                                while let Some(pos) = text_lower[start..].find(&query_lower) {
                                    // Map the lowercase byte offset back to the original text
                                    // by counting the same number of chars
                                    let char_start = text_lower[..start + pos].chars().count();
                                    let char_end = text_lower[..start + pos + query_len].chars().count();
                                    let orig_start: usize = text.char_indices().nth(char_start).map(|(i, _)| i).unwrap_or(text.len());
                                    let orig_end: usize = text.char_indices().nth(char_end).map(|(i, _)| i).unwrap_or(text.len());
                                    matches.push(SearchMatch {
                                        block_idx: i,
                                        byte_start: orig_start,
                                        byte_end: orig_end,
                                    });
                                    start += pos + query_len;
                                }
                            }
                            // Scroll to first match
                            if !matches.is_empty() {
                                let block_idx = matches[0].block_idx;
                                let block_top_px = viewport.block_offsets[block_idx];
                                scroll_row = pixel_to_rows(block_top_px, cell_h);
                            }
                            // Set highlights and evict affected blocks
                            let highlights: Vec<(usize, usize, usize, bool)> = matches.iter().enumerate()
                                .map(|(i, m)| (m.block_idx, m.byte_start, m.byte_end, i == current_match))
                                .collect();
                            let affected = viewport.set_search_highlights(highlights);
                            for idx in &affected {
                                for c in 0..20 { let _ = term.delete_image(block_image_id(*idx, c)); }
                                transmitted.remove(idx);
                            }
                            // Full redraw
                            redraw(
                                &mut term, &mut viewport, &blocks, &theme,
                                &mut transmitted, scroll_row, cell_h, margin_cols,
                            )?;
                            draw_status_search(&mut term, &mut viewport, &search_query, scroll_row, &theme, current_match, &matches)?;
                        } else {
                            draw_status(&mut term, &mut viewport, filename, scroll_row, &theme, hover_url.as_deref())?;
                        }
                    }
                    KeyCode::Backspace => {
                        search_input.pop();
                        draw_search_bar(&mut term, &search_input, &theme)?;
                    }
                    KeyCode::Char(c) => {
                        search_input.push(c);
                        draw_search_bar(&mut term, &search_input, &theme)?;
                    }
                    _ => {}
                }
            }
            continue;
        }

        // Handle search/select keys in normal mode
        if let Event::Key(key) = &first {
            if key.code == KeyCode::Char('r') || key.code == KeyCode::Char('v') {
                let selecting = key.code == KeyCode::Char('v');
                let start_line = scroll_to_source_line(scroll_row, cell_h, &viewport, &blocks, &raw_content);
                source_mode(&mut term, &raw_content, filename, &theme, start_line, selecting)?;
                // Return to rendered view
                set_term_bg(&mut term, &theme)?;
                term.delete_all_images()?;
                transmitted.clear();
                redraw(&mut term, &mut viewport, &blocks, &theme, &mut transmitted, scroll_row, cell_h, margin_cols)?;
                draw_status(&mut term, &mut viewport, filename, scroll_row, &theme, hover_url.as_deref())?;
                continue;
            } else if key.code == KeyCode::Char('e') {
                // Open file in $EDITOR
                let editor = std::env::var("VISUAL")
                    .or_else(|_| std::env::var("EDITOR"))
                    .unwrap_or_else(|_| "vi".to_string());
                // Drain any buffered crossterm events
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                // Tear down terminal
                term.cleanup()?;
                drop(term);
                // Let terminal process the "disable mouse tracking" escape,
                // then flush any remaining input bytes
                std::thread::sleep(Duration::from_millis(50));
                unsafe extern "C" { fn tcflush(fd: i32, action: i32) -> i32; }
                // TCIFLUSH: 1 on macOS, 0 on Linux
                let tciflush = if cfg!(target_os = "macos") { 1 } else { 0 };
                unsafe { tcflush(0, tciflush); }
                let editor_result = Command::new("sh")
                    .arg("-c")
                    .arg(format!("{} \"{}\"", editor, filename.replace('"', "\\\"")))
                    .status();
                // Re-init terminal
                term = Terminal::new()?;
                set_term_bg(&mut term, &theme)?;
                // Check editor result and show error or reload
                let editor_err = match &editor_result {
                    Err(e) => Some(format!("failed to launch '{}': {}", editor, e)),
                    Ok(status) if !status.success() => {
                        let code = status.code().map(|c| c.to_string()).unwrap_or("signal".into());
                        Some(format!("'{}' exited with {}", editor, code))
                    }
                    Ok(_) => {
                        // Reload file — content may have changed
                        if let Ok(new_content) = std::fs::read_to_string(filename) {
                            blocks = crate::parser::parse(&new_content);
                            raw_content = new_content;
                            viewport = Viewport::new(&blocks, width_px, font_size, cell_h, &theme, font.as_deref(), &spacing, base_dir.clone());
                            scroll_row = scroll_row.min(pixel_to_rows(viewport.total_height, cell_h).saturating_sub(1));
                            search_query.clear();
                            matches.clear();
                            current_match = 0;
                        }
                        None
                    }
                };
                transmitted.clear();
                term.delete_all_images()?;
                redraw(&mut term, &mut viewport, &blocks, &theme, &mut transmitted, scroll_row, cell_h, margin_cols)?;
                if let Some(err_msg) = editor_err {
                    let err_bg: [u8; 3] = [140, 30, 30];
                    let err_fg: [u8; 3] = [255, 200, 200];
                    draw_status_panes(&mut term,
                        &[
                            StatusPane { text: format!("{:^8}", "ERROR"), bg: err_bg, fg: [255, 255, 255], bold: true, fill: false },
                            StatusPane { text: err_msg, bg: err_bg, fg: err_fg, bold: false, fill: true },
                        ],
                        &[], err_bg, crate::theme::color_to_rgb(theme.background),
                    )?;
                } else {
                    draw_status(&mut term, &mut viewport, filename, scroll_row, &theme, hover_url.as_deref())?;
                }
                continue;
            } else if keys.normal.search.matches(key) {
                mode = Mode::Search;
                search_input.clear();
                draw_search_bar(&mut term, &search_input, &theme)?;
                continue;
            } else if keys.normal.next_match.matches(key) && !matches.is_empty() {
                navigate_match(
                    &matches, &mut current_match, 1,
                    &mut viewport, &mut term, &blocks, &theme,
                    &mut transmitted, &mut scroll_row, cell_h, margin_cols, &search_query,
                )?;
                continue;
            } else if keys.normal.prev_match.matches(key) && !matches.is_empty() {
                navigate_match(
                    &matches, &mut current_match, -1,
                    &mut viewport, &mut term, &blocks, &theme,
                    &mut transmitted, &mut scroll_row, cell_h, margin_cols, &search_query,
                )?;
                continue;
            } else if key.code == KeyCode::Char('?') {
                show_help(&mut term, &theme)?;
                set_term_bg(&mut term, &theme)?;
                term.delete_all_images()?;
                transmitted.clear();
                redraw(&mut term, &mut viewport, &blocks, &theme, &mut transmitted, scroll_row, cell_h, margin_cols)?;
                draw_status(&mut term, &mut viewport, filename, scroll_row, &theme, hover_url.as_deref())?;
                continue;
            } else if key.code == KeyCode::Esc && !search_query.is_empty() {
                let affected = viewport.clear_search_highlights();
                for idx in &affected {
                    for c in 0..20 { let _ = term.delete_image(block_image_id(*idx, c)); }
                    transmitted.remove(idx);
                }
                search_query.clear();
                matches.clear();
                current_match = 0;
                redraw(
                    &mut term, &mut viewport, &blocks, &theme,
                    &mut transmitted, scroll_row, cell_h, margin_cols,
                )?;
                draw_status(&mut term, &mut viewport, filename, scroll_row, &theme, hover_url.as_deref())?;
                continue;
            }
        }

        process_event(
            &first, &keys, &mut scroll_row, &mut quit, &mut changed,
            max_scroll, content_rows, scroll_step, &mut term, &mut viewport,
            &blocks, &mut transmitted, cell_h, margin_px, margin_cols, cell_w, &mut cursor_on_link, &mut status_msg, &mut hover_url,
        )?;

        if quit { break; }

        // Drain pending events — jump to final position (cap at 50 to prevent spin)
        let mut drain_count = 0u32;
        while drain_count < 50 && event::poll(Duration::from_millis(0))? {
            let ev = event::read()?;
            drain_count += 1;
            process_event(
                &ev, &keys, &mut scroll_row, &mut quit, &mut changed,
                max_scroll, content_rows, scroll_step, &mut term, &mut viewport,
                &blocks, &mut transmitted, cell_h, margin_px, margin_cols, cell_w, &mut cursor_on_link, &mut status_msg, &mut hover_url,
            )?;
            if quit { break; }
        }

        if quit { break; }

        if changed {
            if scroll_row != old_scroll {
                let delta = scroll_row as i64 - old_scroll as i64;
                let content_rows = term.content_rows() as i64;

                if delta.abs() >= content_rows {
                    redraw(
                        &mut term, &mut viewport, &blocks, &theme,
                        &mut transmitted, scroll_row, cell_h, margin_cols,
                    )?;
                } else {
                    incremental_scroll(
                        &mut term, &mut viewport, &blocks, &theme,
                        &mut transmitted, old_scroll, scroll_row, cell_h, margin_cols,
                    )?;
                }

                // Periodic eviction — every 20 scrolls
                scroll_count += 1;
                if scroll_count % 20 == 0 {
                    let sp = scroll_row * cell_h as u32;
                    let vp = term.content_height_px();
                    evict_distant(&mut term, &mut viewport, &mut transmitted, sp, vp);
                }
            } else {
                // No scroll change but something changed (e.g. resize) — full redraw
                redraw(
                    &mut term, &mut viewport, &blocks, &theme,
                    &mut transmitted, scroll_row, cell_h, margin_cols,
                )?;
            }

            if !matches.is_empty() {
                draw_status_search(&mut term, &mut viewport, &search_query, scroll_row, &theme, current_match, &matches)?;
            } else {
                draw_status(&mut term, &mut viewport, filename, scroll_row, &theme, hover_url.as_deref())?;
            }
        }

        // Show status message from link clicks (even if nothing else changed)
        if let Some(msg) = &status_msg {
            let mode_bg = crate::theme::color_to_rgb(theme.status_bar_view);
            let fill = crate::theme::color_to_rgb(theme.status_info_bg);
            draw_status_panes(&mut term,
                &[pane_mode("NORMAL", &theme, mode_bg), pane_info(msg, &theme)],
                &[], fill, crate::theme::color_to_rgb(theme.background),
            )?;
        }

    }

    term.cleanup()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn process_event(
    ev: &Event,
    keys: &ResolvedKeys,
    scroll_row: &mut u32,
    quit: &mut bool,
    changed: &mut bool,
    max_scroll: u32,
    content_rows: u32,
    scroll_step: u32,
    term: &mut Terminal,
    viewport: &mut Viewport,
    blocks: &[Block],
    transmitted: &mut HashSet<usize>,
    cell_h: u16,
    margin_px: u32,
    margin_cols: u16,
    cell_w: u16,
    cursor_on_link: &mut bool,
    status_msg: &mut Option<String>,
    hover_url: &mut Option<String>,
) -> Result<()> {
    match ev {
        Event::Key(key) => {
            if *key == crossterm::event::KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL) {
                *quit = true;
            } else if keys.normal.quit.matches(key) {
                *quit = true;
            } else if keys.normal.scroll_down.matches(key) {
                *scroll_row = (*scroll_row + scroll_step).min(max_scroll);
                *changed = true;
            } else if keys.normal.scroll_up.matches(key) {
                *scroll_row = scroll_row.saturating_sub(scroll_step);
                *changed = true;
            } else if keys.normal.page_down.matches(key) {
                *scroll_row = (*scroll_row + content_rows / 2).min(max_scroll);
                *changed = true;
            } else if keys.normal.page_up.matches(key) {
                *scroll_row = scroll_row.saturating_sub(content_rows / 2);
                *changed = true;
            } else if keys.normal.top.matches(key) {
                *scroll_row = 0;
                *changed = true;
            } else if keys.normal.bottom.matches(key) {
                *scroll_row = max_scroll;
                *changed = true;
            }
        }
        Event::Mouse(mouse) => {
            match mouse.kind {
                MouseEventKind::ScrollDown => {
                    *scroll_row = (*scroll_row + scroll_step * 3).min(max_scroll);
                    *changed = true;
                }
                MouseEventKind::ScrollUp => {
                    *scroll_row = scroll_row.saturating_sub(scroll_step * 3);
                    *changed = true;
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    let (block_idx, rel_x, rel_y) = mouse_to_block(mouse, *scroll_row, cell_h, margin_cols, cell_w, viewport);

                    if block_idx < blocks.len() {
                        // Click — check for copy button first, then links
                        if viewport.copy_button_at_pixel(block_idx, rel_x, rel_y) {
                            if let Block::CodeBlock { code, .. } = &blocks[block_idx] {
                                let _ = term.copy_to_clipboard(code);
                                *status_msg = Some("Copied to clipboard".to_string());
                            }
                        } else if let Some(url) = viewport.link_at_pixel(block_idx, rel_x, rel_y) {
                            if let Some(anchor) = url.strip_prefix('#') {
                                if let Some(target) = find_heading_by_slug(blocks, anchor) {
                                    let target_px = viewport.block_offsets[target];
                                    *scroll_row = pixel_to_rows(target_px, cell_h);
                                    *changed = true;
                                } else {
                                    *status_msg = Some(format!("Anchor not found: #{}", anchor));
                                }
                            } else if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:") {
                                let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
                                match std::process::Command::new(opener)
                                    .arg(&url)
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .spawn()
                                {
                                    Ok(_) => *status_msg = Some(format!("Opening: {}", url)),
                                    Err(e) => *status_msg = Some(format!("Failed to open: {}", e)),
                                }
                            } else {
                                *status_msg = Some(format!("Unsupported link: {}", url));
                            }
                        }
                    }
                }
                MouseEventKind::Moved => {
                    let (block_idx, rel_x, rel_y) = mouse_to_block(mouse, *scroll_row, cell_h, margin_cols, cell_w, viewport);
                    let link = if block_idx < blocks.len() {
                        viewport.link_at_pixel(block_idx, rel_x, rel_y)
                    } else {
                        None
                    };
                    let on_copy = block_idx < blocks.len() && viewport.copy_button_at_pixel(block_idx, rel_x, rel_y);
                    let on_clickable = link.is_some() || on_copy;
                    if on_clickable != *cursor_on_link {
                        *cursor_on_link = on_clickable;
                        if on_clickable {
                            let _ = term.set_pointer_cursor();
                        } else {
                            let _ = term.set_default_cursor();
                        }
                        let _ = term.flush();
                    }
                    if *hover_url != link {
                        *hover_url = link;
                        *changed = true;
                    }
                }
                _ => {}
            }
        }
        Event::Resize(_w, _h) => {
            term.update_size();
            let new_width = term.content_width_px().saturating_sub(margin_px * 2);
            if new_width != viewport.width_px {
                // Width changed — re-render all blocks at new width
                viewport.resize(blocks, new_width);
            }
            // Always clear terminal images and retransmit (row count may have changed)
            term.delete_all_images()?;
            transmitted.clear();
            *scroll_row = (*scroll_row).min(
                pixel_to_rows(viewport.total_height, cell_h)
                    .saturating_sub(term.content_rows() as u32)
            );
            *changed = true;
        }
        _ => {}
    }
    Ok(())
}

/// Redraw the content area using Unicode placeholders.
/// Ensures all visible block images are transmitted, then prints
/// U+10EEEE placeholder rows.
fn redraw(
    term: &mut Terminal,
    viewport: &mut Viewport,
    blocks: &[Block],
    theme: &Theme,
    transmitted: &mut HashSet<usize>,
    scroll_row: u32,
    cell_h: u16,
    margin_cols: u16,
) -> Result<()> {
    let ch = cell_h as u32;
    if ch == 0 { return Ok(()); }
    let content_rows = term.content_rows();
    let cols = term.cols.saturating_sub(margin_cols * 2);

    let scroll_px = scroll_row * ch;
    let viewport_px = content_rows as u32 * ch;
    let scroll_end = scroll_px + viewport_px;

    // Render + transmit visible blocks. Restart on any height correction because
    // offsets shifted — blocks that were in range may now be out (or vice versa),
    // and the safety net may have invalidated blocks we'd already passed.
    loop {
        let first = viewport.block_offsets
            .partition_point(|&off| off < scroll_px)
            .saturating_sub(1);

        let mut corrected = false;
        for i in first..blocks.len() {
            if i >= viewport.block_offsets.len() { break; }
            let block_top = viewport.block_offsets[i];
            if block_top >= scroll_end { break; }
            let block_bottom = block_top + viewport.block_heights[i];
            if block_bottom <= scroll_px { continue; }

            if !transmitted.contains(&i) {
                let img = viewport.ensure_block_rendered(i, blocks, theme);
                let rgba = img.as_rgba8().expect("image is rgba8");
                transmit_block_image(term, i, rgba.as_raw(), img.width(), img.height(), cell_h)?;
                transmitted.insert(i);

                if let Some(corrected_idx) = viewport.height_corrected_at.take() {
                    for j in (corrected_idx + 1)..blocks.len() {
                        if transmitted.remove(&j) {
                            for c in 0..20 { let _ = term.delete_image(block_image_id(j, c)); }
                        }
                    }
                    corrected = true;
                    break;
                }
            }
        }
        if !corrected { break; }
    }

    // Clear content area
    term.clear_content()?;

    // Print placeholder rows with margin offset
    crossterm::queue!(term.stdout_mut(), crossterm::cursor::MoveTo(0, 0))?;

    for screen_row in 0..content_rows {
        let pixel_y = scroll_px + screen_row as u32 * ch;

        // Binary search for the block containing this pixel_y
        let idx = viewport.block_offsets
            .partition_point(|&off| off <= pixel_y)
            .saturating_sub(1);

        let mut printed = false;
        if idx < viewport.block_offsets.len() {
            let block_top = viewport.block_offsets[idx];
            let block_bottom = block_top + viewport.block_heights[idx];
            if pixel_y >= block_top && pixel_y < block_bottom && transmitted.contains(&idx) {
                let row_in_image = (pixel_y - block_top) / ch;
                let chunk = row_in_image / crate::terminal::MAX_IMAGE_ROWS;
                let row_in_chunk = row_in_image % crate::terminal::MAX_IMAGE_ROWS;
                let image_id = block_image_id(idx, chunk);
                crossterm::queue!(term.stdout_mut(), crossterm::cursor::MoveTo(margin_cols, screen_row))?;
                term.print_placeholder_row(image_id, row_in_chunk, cols)?;
                printed = true;
            }
        }

        if !printed {
            write!(term.stdout_mut(), "\x1b[2K")?; // clear line
        }

        if screen_row + 1 < content_rows {
            write!(term.stdout_mut(), "\r\n")?;
        }
    }

    term.flush()?;

    // Evict distant block caches and transmitted images
    evict_distant(term, viewport, transmitted, scroll_px, viewport_px);

    Ok(())
}

/// Evict block caches and terminal images far from viewport.
/// Find the next un-transmitted block in the prefetch window, prioritizing
/// blocks below the viewport (next scroll-down direction).
fn next_prefetch_block(
    viewport: &Viewport,
    transmitted: &HashSet<usize>,
    scroll_px: u32,
    viewport_px: u32,
) -> Option<usize> {
    let window_start = scroll_px.saturating_sub(viewport_px);
    let window_end = scroll_px + viewport_px * 2;
    let viewport_end = scroll_px + viewport_px;

    // First: blocks intersecting [viewport_end, window_end) — i.e., the next viewport down
    let first_below = viewport.block_offsets.partition_point(|&off| off < viewport_end)
        .saturating_sub(1);
    for i in first_below..viewport.block_offsets.len() {
        let top = viewport.block_offsets[i];
        if top >= window_end { break; }
        let bottom = top + viewport.block_heights[i];
        if bottom <= viewport_end { continue; }
        if !transmitted.contains(&i) { return Some(i); }
    }

    // Then: blocks intersecting [window_start, scroll_px) — the viewport above
    let first_above = viewport.block_offsets.partition_point(|&off| off < window_start)
        .saturating_sub(1);
    for i in first_above..viewport.block_offsets.len() {
        let top = viewport.block_offsets[i];
        if top >= scroll_px { break; }
        let bottom = top + viewport.block_heights[i];
        if bottom <= window_start { continue; }
        if !transmitted.contains(&i) { return Some(i); }
    }

    None
}

fn evict_distant(
    term: &mut Terminal,
    viewport: &mut Viewport,
    transmitted: &mut HashSet<usize>,
    scroll_px: u32,
    viewport_px: u32,
) {
    let margin = viewport_px;
    let keep_start = scroll_px.saturating_sub(margin);
    let keep_end = scroll_px + viewport_px + margin;

    let to_evict: Vec<usize> = transmitted.iter()
        .filter(|&&i| {
            if i >= viewport.block_offsets.len() { return true; }
            let top = viewport.block_offsets[i];
            let bottom = top + viewport.block_heights[i];
            bottom < keep_start || top > keep_end
        })
        .copied()
        .collect();

    for i in to_evict {
        for c in 0..20 { let _ = term.delete_image(block_image_id(i, c)); }
        transmitted.remove(&i);
    }

    viewport.evict_distant_blocks(scroll_px, viewport_px);
}

/// Image ID for a block chunk. Block i, chunk c → unique ID.
/// Set the terminal default background color from the theme.
fn set_term_bg(term: &mut Terminal, theme: &Theme) -> Result<()> {
    let [r, g, b] = crate::theme::color_to_rgb(theme.background);
    write!(term.stdout_mut(), "\x1b[48;2;{};{};{}m", r, g, b)?;
    Ok(())
}

/// Chunk 0 covers rows 0..MAX_IMAGE_ROWS, chunk 1 covers MAX_IMAGE_ROWS..2*MAX_IMAGE_ROWS, etc.
fn block_image_id(block_idx: usize, chunk: u32) -> u32 {
    (block_idx as u32 + 1) * 1000 + chunk
}

/// Number of chunks needed for an image of given height.
fn block_chunk_count(image_height: u32, cell_h: u16) -> u32 {
    let image_rows = pixel_to_rows(image_height, cell_h);
    (image_rows + crate::terminal::MAX_IMAGE_ROWS - 1) / crate::terminal::MAX_IMAGE_ROWS
}

/// Transmit a block image, splitting into chunks if taller than MAX_IMAGE_ROWS.
fn transmit_block_image(
    term: &mut Terminal,
    block_idx: usize,
    rgba: &[u8],
    width: u32,
    height: u32,
    cell_h: u16,
) -> Result<()> {
    let max_chunk_px = crate::terminal::MAX_IMAGE_ROWS * cell_h as u32;
    let chunks = block_chunk_count(height, cell_h);

    for c in 0..chunks {
        let y_start = c * max_chunk_px;
        let y_end = ((c + 1) * max_chunk_px).min(height);
        let chunk_h = y_end - y_start;
        let row_bytes = (width * 4) as usize;
        let start = y_start as usize * row_bytes;
        let end = y_end as usize * row_bytes;
        let chunk_data = &rgba[start..end];
        let id = block_image_id(block_idx, c);
        term.transmit_virtual(id, chunk_data, width, chunk_h)?;
    }
    Ok(())
}

fn pixel_to_rows(pixels: u32, cell_h: u16) -> u32 {
    if cell_h == 0 { return 0; }
    (pixels + cell_h as u32 - 1) / cell_h as u32
}

/// Map mouse event position to (block_idx, rel_x, rel_y).
fn mouse_to_block(
    mouse: &crossterm::event::MouseEvent,
    scroll_row: u32,
    cell_h: u16,
    margin_cols: u16,
    cell_w: u16,
    viewport: &Viewport,
) -> (usize, f32, f32) {
    let ch = cell_h as u32;
    let scroll_px = scroll_row * ch;
    let pixel_y = scroll_px + mouse.row as u32 * ch + ch / 2;
    let pixel_x = mouse.column.saturating_sub(margin_cols) as u32 * cell_w as u32 + cell_w as u32 / 2;

    let block_idx = viewport.block_offsets
        .partition_point(|&off| off <= pixel_y)
        .saturating_sub(1);

    let block_top = if block_idx < viewport.block_offsets.len() {
        viewport.block_offsets[block_idx]
    } else {
        0
    };
    (block_idx, pixel_x as f32, pixel_y.saturating_sub(block_top) as f32)
}

/// Find a heading block matching an anchor. Checks explicit id first, then text slug.
fn find_heading_by_slug(blocks: &[Block], anchor: &str) -> Option<usize> {
    // First pass: check explicit HTML id attributes
    for (i, block) in blocks.iter().enumerate() {
        if let Block::Heading { id: Some(id), .. } = block {
            if id == anchor {
                return Some(i);
            }
        }
    }
    // Second pass: check generated slugs (GitHub-style)
    for (i, block) in blocks.iter().enumerate() {
        if let Block::Heading { spans, .. } = block {
            let text: String = spans.iter().map(|s| s.text.as_str()).collect();
            let slug: String = text
                .to_lowercase()
                .chars()
                .map(|c| if c == ' ' { '-' } else { c })
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect();
            if slug == anchor {
                return Some(i);
            }
        }
    }
    None
}

/// Incremental scroll: shift content with terminal scroll, draw only new rows.
fn incremental_scroll(
    term: &mut Terminal,
    viewport: &mut Viewport,
    blocks: &[Block],
    theme: &Theme,
    transmitted: &mut HashSet<usize>,
    old_scroll: u32,
    new_scroll: u32,
    cell_h: u16,
    margin_cols: u16,
) -> Result<()> {
    let ch = cell_h as u32;
    if ch == 0 { return Ok(()); }
    let content_rows = term.content_rows();
    let cols = term.cols.saturating_sub(margin_cols * 2);
    let delta = new_scroll as i64 - old_scroll as i64;
    let abs_delta = delta.unsigned_abs() as u16;

    // Set scroll region to content area (exclude status bar)
    write!(term.stdout_mut(), "\x1b[1;{}r", content_rows)?;

    let new_scroll_px = new_scroll * ch;
    let vp_px = content_rows as u32 * ch;

    // Only check for new blocks to transmit — just for the newly exposed rows.
    // If a height correction happens here, offsets shifted underneath us and the
    // safety net invalidated visible blocks elsewhere; the cheap scroll-region
    // approach can't repair the screen, so fall back to a full redraw.
    let corrected = if delta > 0 {
        let exposed_start = new_scroll_px + vp_px - abs_delta as u32 * ch;
        let exposed_end = new_scroll_px + vp_px;
        ensure_blocks_for_range(term, viewport, blocks, theme, transmitted, exposed_start, exposed_end, cell_h)?
    } else {
        let exposed_start = new_scroll_px;
        let exposed_end = new_scroll_px + abs_delta as u32 * ch;
        ensure_blocks_for_range(term, viewport, blocks, theme, transmitted, exposed_start, exposed_end, cell_h)?
    };
    if corrected {
        write!(term.stdout_mut(), "\x1b[r")?; // reset scroll region before redraw
        return redraw(term, viewport, blocks, theme, transmitted, new_scroll, cell_h, margin_cols);
    }

    if delta > 0 {
        write!(term.stdout_mut(), "\x1b[{}S", abs_delta)?;
        for i in 0..abs_delta {
            let screen_row = content_rows - abs_delta + i;
            let pixel_y = new_scroll_px + screen_row as u32 * ch;
            crossterm::queue!(term.stdout_mut(), crossterm::cursor::MoveTo(margin_cols, screen_row))?;
            print_row(term, viewport, blocks, transmitted, pixel_y, cell_h, cols)?;
        }
    } else {
        write!(term.stdout_mut(), "\x1b[{}T", abs_delta)?;
        for i in 0..abs_delta {
            let screen_row = i;
            let pixel_y = new_scroll_px + screen_row as u32 * ch;
            crossterm::queue!(term.stdout_mut(), crossterm::cursor::MoveTo(margin_cols, screen_row))?;
            print_row(term, viewport, blocks, transmitted, pixel_y, cell_h, cols)?;
        }
    }

    // Reset scroll region to full terminal
    write!(term.stdout_mut(), "\x1b[r")?;
    term.flush()?;
    Ok(())
}

/// Ensure blocks overlapping a specific pixel range are rendered and transmitted.
/// Used for incremental scroll — only checks the newly exposed rows.
/// Returns true if any block render triggered a height correction (offsets
/// shifted). When true, the caller must do a full redraw rather than rely on
/// incremental painting, because blocks elsewhere in the viewport were
/// invalidated by the safety net.
fn ensure_blocks_for_range(
    term: &mut Terminal,
    viewport: &mut Viewport,
    blocks: &[Block],
    theme: &Theme,
    transmitted: &mut HashSet<usize>,
    range_start: u32,
    range_end: u32,
    cell_h: u16,
) -> Result<bool> {
    let mut corrected = false;
    let first = viewport.block_offsets
        .partition_point(|&off| off < range_start)
        .saturating_sub(1);

    for i in first..blocks.len() {
        if i >= viewport.block_offsets.len() { break; }
        let block_top = viewport.block_offsets[i];
        if block_top >= range_end { break; }
        let block_bottom = block_top + viewport.block_heights[i];
        if block_bottom <= range_start { continue; }

        if !transmitted.contains(&i) {
            let img = viewport.ensure_block_rendered(i, blocks, theme);
            let rgba = img.as_rgba8().expect("image is rgba8");
            transmit_block_image(term, i, rgba.as_raw(), img.width(), img.height(), cell_h)?;
            transmitted.insert(i);

            if let Some(corrected_idx) = viewport.height_corrected_at.take() {
                for j in (corrected_idx + 1)..blocks.len() {
                    if transmitted.remove(&j) {
                        for c in 0..20 { let _ = term.delete_image(block_image_id(j, c)); }
                    }
                }
                corrected = true;
            }
        }
    }
    Ok(corrected)
}

/// Print a single placeholder row at the cursor position (binary search).
fn print_row(
    term: &mut Terminal,
    viewport: &Viewport,
    blocks: &[Block],
    transmitted: &HashSet<usize>,
    pixel_y: u32,
    ch: u16,
    cols: u16,
) -> Result<()> {
    let ch32 = ch as u32;

    // Binary search for the block containing pixel_y
    let idx = viewport.block_offsets
        .partition_point(|&off| off <= pixel_y)
        .saturating_sub(1);

    if idx < blocks.len() && idx < viewport.block_offsets.len() {
        let block_top = viewport.block_offsets[idx];
        let block_bottom = block_top + viewport.block_heights[idx];

        if pixel_y >= block_top && pixel_y < block_bottom && transmitted.contains(&idx) {
            let row_in_image = (pixel_y - block_top) / ch32;
            let chunk = row_in_image / crate::terminal::MAX_IMAGE_ROWS;
            let row_in_chunk = row_in_image % crate::terminal::MAX_IMAGE_ROWS;
            let image_id = block_image_id(idx, chunk);
            term.print_placeholder_row(image_id, row_in_chunk, cols)?;
            return Ok(());
        }
    }

    // Gap or untransmitted — blank line
    write!(term.stdout_mut(), "\x1b[2K")?; // clear line (faster than writing spaces)
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn navigate_match(
    matches: &[SearchMatch],
    current_match: &mut usize,
    direction: i32,
    viewport: &mut Viewport,
    term: &mut Terminal,
    blocks: &[Block],
    theme: &Theme,
    transmitted: &mut HashSet<usize>,
    scroll_row: &mut u32,
    cell_h: u16,
    margin_cols: u16,
    search_query: &str,
) -> Result<()> {
    *current_match = if direction > 0 {
        (*current_match + 1) % matches.len()
    } else if *current_match == 0 {
        matches.len() - 1
    } else {
        *current_match - 1
    };
    let block_idx = matches[*current_match].block_idx;
    let block_top_px = viewport.block_offsets[block_idx];
    let block_bottom_px = block_top_px + viewport.block_heights[block_idx];
    let scroll_px = *scroll_row * cell_h as u32;
    let viewport_bottom = scroll_px + term.content_rows() as u32 * cell_h as u32;
    if block_top_px < scroll_px || block_bottom_px > viewport_bottom {
        *scroll_row = pixel_to_rows(block_top_px, cell_h);
    }
    let highlights: Vec<(usize, usize, usize, bool)> = matches.iter().enumerate()
        .map(|(i, m)| (m.block_idx, m.byte_start, m.byte_end, i == *current_match))
        .collect();
    let affected = viewport.set_search_highlights(highlights);
    for idx in &affected {
        for c in 0..20 { let _ = term.delete_image(block_image_id(*idx, c)); }
        transmitted.remove(idx);
    }
    redraw(
        term, viewport, blocks, theme,
        transmitted, *scroll_row, cell_h, margin_cols,
    )?;
    draw_status_search(term, viewport, search_query, *scroll_row, theme, *current_match, matches)?;
    Ok(())
}

use crate::graphics::StatusPane;

fn pane_mode(text: &str, theme: &Theme, mode_bg: [u8; 3]) -> StatusPane {
    StatusPane { text: format!("{:^8}", text), bg: mode_bg, fg: crate::theme::color_to_rgb(theme.status_bar_fg), bold: true, fill: false }
}
fn pane_progress(text: &str, mode_bg: [u8; 3]) -> StatusPane {
    StatusPane { text: format!("{:>10}", text), bg: mode_bg, fg: [255, 255, 255], bold: false, fill: false }
}
fn pane_file(text: &str, theme: &Theme) -> StatusPane {
    StatusPane { text: text.to_string(), bg: crate::theme::color_to_rgb(theme.status_filename_bg), fg: crate::theme::color_to_rgb(theme.status_mid_fg), bold: false, fill: true }
}
fn pane_info(text: &str, theme: &Theme) -> StatusPane {
    StatusPane { text: format!("{:^12}", text), bg: crate::theme::color_to_rgb(theme.status_info_bg), fg: crate::theme::color_to_rgb(theme.status_mid_fg), bold: false, fill: false }
}
fn pane_keys(text: &str, theme: &Theme) -> StatusPane {
    StatusPane { text: format!("{:^26}", text), bg: crate::theme::color_to_rgb(theme.status_keys_bg), fg: crate::theme::color_to_rgb(theme.status_dim_fg), bold: false, fill: false }
}

fn draw_status_panes(
    term: &mut Terminal,
    left: &[StatusPane],
    right: &[StatusPane],
    fill_bg: [u8; 3],
    restore_bg: [u8; 3],
) -> Result<()> {
    let cols = term.cols as usize;
    let status_row = term.content_rows();

    let pane_width = |p: &StatusPane| p.text.chars().count() + 2; // 1 space pad each side
    let fixed_left: usize = left.iter().filter(|p| !p.fill).map(pane_width).sum();
    let fixed_right: usize = right.iter().filter(|p| !p.fill).map(pane_width).sum();
    let fill_width = cols.saturating_sub(fixed_left + fixed_right);
    let has_any_fill = left.iter().any(|p| p.fill) || right.iter().any(|p| p.fill);

    crossterm::queue!(term.stdout_mut(), crossterm::cursor::MoveTo(0, status_row))?;

    for p in left {
        emit_pane(term, p, fill_width)?;
    }
    if !has_any_fill {
        let stdout = term.stdout_mut();
        write!(stdout, "\x1b[48;2;{};{};{}m", fill_bg[0], fill_bg[1], fill_bg[2])?;
        for _ in 0..fill_width {
            stdout.write_all(b" ")?;
        }
    }
    for p in right {
        emit_pane(term, p, fill_width)?;
    }

    // Restore the terminal bg to theme background so subsequent `\x1b[2K`
    // line-clears (in redraw / clear_content) paint theme bg, not default black.
    write!(
        term.stdout_mut(),
        "\x1b[39m\x1b[22m\x1b[48;2;{};{};{}m",
        restore_bg[0], restore_bg[1], restore_bg[2]
    )?;
    term.flush()?;
    Ok(())
}

fn emit_pane(term: &mut Terminal, p: &StatusPane, fill_width: usize) -> Result<()> {
    let stdout = term.stdout_mut();
    write!(
        stdout,
        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m",
        p.fg[0], p.fg[1], p.fg[2], p.bg[0], p.bg[1], p.bg[2]
    )?;
    if p.bold {
        write!(stdout, "\x1b[1m")?;
    }
    if p.fill {
        // Fill panes get padded/truncated to fill_width chars (no extra padding)
        let n = p.text.chars().count();
        if n >= fill_width {
            let truncated: String = p.text.chars().take(fill_width).collect();
            write!(stdout, "{}", truncated)?;
        } else {
            write!(stdout, "{}", p.text)?;
            for _ in 0..(fill_width - n) {
                stdout.write_all(b" ")?;
            }
        }
    } else {
        // Fixed panes get 1 space padding on each side for breathing room
        write!(stdout, " {} ", p.text)?;
    }
    if p.bold {
        write!(stdout, "\x1b[22m")?;
    }
    Ok(())
}

/// Map the rendered viewport scroll position to a source line number.
/// Finds which block is at the top of the screen, then locates that
/// block's text in the raw content to get the corresponding line.
fn scroll_to_source_line(scroll_row: u32, cell_h: u16, viewport: &Viewport, blocks: &[Block], raw_content: &str) -> usize {
    let scroll_px = scroll_row as u64 * cell_h as u64;
    // Find the block at the top of the viewport
    let top_block = viewport.block_offsets.iter()
        .rposition(|&off| (off as u64) <= scroll_px)
        .unwrap_or(0);
    // Find source line for this block by searching for its text
    let block_text = blocks.get(top_block).map(|b| b.text_content()).unwrap_or_default();
    let first_word = block_text.split_whitespace().next().unwrap_or("");
    if first_word.is_empty() {
        return 0;
    }
    let lines: Vec<&str> = raw_content.lines().collect();
    // Walk blocks before top_block to get a minimum line to search from
    let mut min_line = 0usize;
    for i in 0..top_block {
        let text = blocks[i].text_content();
        let word = text.split_whitespace().next().unwrap_or("");
        if word.is_empty() { continue; }
        for j in min_line..lines.len() {
            if lines[j].contains(word) {
                min_line = j + 1;
                break;
            }
        }
    }
    // Find the target block's line
    for j in min_line..lines.len() {
        if lines[j].contains(first_word) {
            return j;
        }
    }
    min_line
}

fn scroll_pct(scroll_row: u32, viewport: &Viewport, term: &Terminal) -> String {
    let total_rows = pixel_to_rows(viewport.total_height, term.cell_height);
    let visible_rows = term.content_rows() as u32;
    let bottom_row = (scroll_row + visible_rows).min(total_rows);
    let pct = if total_rows > 0 {
        (bottom_row * 100 / total_rows) as u16
    } else { 100 };
    format!("{}%", pct)
}

fn draw_status(
    term: &mut Terminal,
    viewport: &mut Viewport,
    filename: &str,
    scroll_row: u32,
    theme: &Theme,
    message: Option<&str>,
) -> Result<()> {
    let mode_bg = crate::theme::color_to_rgb(theme.status_bar_view);
    let fill = crate::theme::color_to_rgb(theme.status_info_bg);
    let pct = scroll_pct(scroll_row, viewport, term);
    let display = message.unwrap_or(filename);
    draw_status_panes(term,
        &[pane_mode("NORMAL", theme, mode_bg), pane_file(display, theme)],
        &[pane_keys("?=help", theme), pane_progress(&pct, mode_bg)],
        fill, crate::theme::color_to_rgb(theme.background),
    )
}

fn draw_search_bar(
    term: &mut Terminal,
    input: &str,
    theme: &Theme,
) -> Result<()> {
    let mode_bg = crate::theme::color_to_rgb(theme.status_bar_select);
    let fill = crate::theme::color_to_rgb(theme.status_info_bg);
    draw_status_panes(term,
        &[pane_mode("SEARCH", theme, mode_bg), pane_file(input, theme)],
        &[pane_keys("enter=search esc=cancel", theme)],
        fill, crate::theme::color_to_rgb(theme.background),
    )
}

fn draw_status_search(
    term: &mut Terminal,
    viewport: &mut Viewport,
    query: &str,
    scroll_row: u32,
    theme: &Theme,
    current: usize,
    matches: &[SearchMatch],
) -> Result<()> {
    let mode_bg = crate::theme::color_to_rgb(theme.status_bar_select);
    let fill = crate::theme::color_to_rgb(theme.status_info_bg);
    let total = matches.len();
    let match_info = if total > 0 { format!("{}/{}", current + 1, total) } else { "no matches".to_string() };
    let pct = scroll_pct(scroll_row, viewport, term);
    let pos = if total > 0 {
        let block_idx = matches[current].block_idx;
        let match_row = pixel_to_rows(viewport.block_offsets[block_idx], term.cell_height);
        format!("L{} {}", match_row + 1, pct)
    } else {
        pct
    };
    draw_status_panes(term,
        &[pane_mode("SEARCH", theme, mode_bg), pane_file(query, theme)],
        &[pane_keys("n/N=next/prev esc=cancel", theme), pane_info(&match_info, theme), pane_progress(&pos, mode_bg)],
        fill, crate::theme::color_to_rgb(theme.background),
    )
}

// ── Help overlay ─────────────────────────────────────────────────────────────

fn theme_to_ct(c: ratatui::style::Color) -> crossterm::style::Color {
    let [r, g, b] = crate::theme::color_to_rgb(c);
    crossterm::style::Color::Rgb { r, g, b }
}

fn show_help(term: &mut Terminal, theme: &Theme) -> Result<()> {
    use crossterm::style::{SetBackgroundColor, SetForegroundColor, ResetColor};

    let bg = theme_to_ct(theme.help_bg);
    let fg = theme_to_ct(theme.help_fg);
    let key_fg = theme_to_ct(theme.link);
    let heading_fg = theme_to_ct(theme.inline_code);
    let dim_fg = theme_to_ct(theme.status_dim_fg);

    let config_path = "~/.config/mdv/config.toml";

    let sections: &[(&str, &[(&str, &str)])] = &[
        ("Navigation", &[
            ("j / k / ↑ / ↓", "Scroll up / down"),
            ("PgDn / PgUp", "Half page down / up"),
            ("g / G", "Go to top / bottom"),
            ("Mouse scroll", "Scroll up / down"),
        ]),
        ("Modes", &[
            ("/", "Enter search mode"),
            ("r", "Raw source view"),
            ("v", "Select mode (in source view)"),
            ("e", "Open in $EDITOR"),
            ("?", "This help screen"),
        ]),
        ("Search", &[
            ("Enter", "Confirm search"),
            ("n / N", "Next / previous match"),
            ("Esc", "Clear search"),
        ]),
        ("Source View", &[
            ("h/j/k/l / Arrows", "Move cursor"),
            ("w / b", "Word forward / back"),
            ("0 / $", "Line start / end"),
            ("v", "Start selection"),
            ("y", "Copy selection"),
            ("Esc / q", "Back to viewer"),
        ]),
        ("Mouse", &[
            ("Scroll", "Scroll up / down"),
            ("Click link", "Open link in browser"),
            ("Hover link", "Show URL in status bar"),
            ("Click + drag", "Select text (source view)"),
        ]),
        ("General", &[
            ("q / Ctrl-c", "Quit"),
        ]),
    ];

    let rows = term.rows as usize;
    let cols = term.cols as usize;

    // Clear screen
    crossterm::queue!(term.stdout_mut(), SetBackgroundColor(bg))?;
    for row in 0..rows {
        crossterm::queue!(term.stdout_mut(), crossterm::cursor::MoveTo(0, row as u16))?;
        write!(term.stdout_mut(), "{:width$}", "", width = cols)?;
    }

    let mut y = 1u16;
    // Title
    crossterm::queue!(term.stdout_mut(),
        crossterm::cursor::MoveTo(4, y),
        SetForegroundColor(heading_fg), SetBackgroundColor(bg),
    )?;
    write!(term.stdout_mut(), "mdv v{} — Keyboard Shortcuts", env!("CARGO_PKG_VERSION"))?;
    y += 2;

    for (section_name, bindings) in sections {
        crossterm::queue!(term.stdout_mut(),
            crossterm::cursor::MoveTo(4, y),
            SetForegroundColor(heading_fg), SetBackgroundColor(bg),
        )?;
        write!(term.stdout_mut(), "{}", section_name)?;
        y += 1;

        for (key, desc) in *bindings {
            if y as usize >= rows - 3 { break; }
            crossterm::queue!(term.stdout_mut(),
                crossterm::cursor::MoveTo(6, y),
                SetForegroundColor(key_fg), SetBackgroundColor(bg),
            )?;
            write!(term.stdout_mut(), "{:<22}", key)?;
            crossterm::queue!(term.stdout_mut(), SetForegroundColor(fg))?;
            write!(term.stdout_mut(), "{}", desc)?;
            y += 1;
        }
        y += 1;
    }

    // Config + license
    if (y as usize) < rows - 4 {
        crossterm::queue!(term.stdout_mut(),
            crossterm::cursor::MoveTo(4, y),
            SetForegroundColor(dim_fg), SetBackgroundColor(bg),
        )?;
        write!(term.stdout_mut(), "Config:  {}", config_path)?;
        y += 1;
        crossterm::queue!(term.stdout_mut(),
            crossterm::cursor::MoveTo(4, y),
            SetForegroundColor(dim_fg), SetBackgroundColor(bg),
        )?;
        write!(term.stdout_mut(), "License: Apache-2.0")?;
    }

    // Footer
    crossterm::queue!(term.stdout_mut(),
        crossterm::cursor::MoveTo(4, rows.saturating_sub(1) as u16),
        SetForegroundColor(dim_fg), SetBackgroundColor(bg),
    )?;
    write!(term.stdout_mut(), "Press any key to close")?;

    crossterm::queue!(term.stdout_mut(), ResetColor)?;
    term.flush()?;

    // Wait for any key
    loop {
        if let Event::Key(_) = event::read()? { break; }
    }
    Ok(())
}

// ── Source mode: raw markdown viewer with cursor and selection ─────────────

fn source_mode(
    term: &mut Terminal,
    content: &str,
    filename: &str,
    theme: &Theme,
    start_line: usize,
    start_selecting: bool,
) -> Result<()> {
    use crossterm::style::{SetBackgroundColor, SetForegroundColor, ResetColor};

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let start = start_line.min(total_lines.saturating_sub(1));
    let mut scroll: usize = start;
    let mut cur_r: usize = start;
    let mut cur_c: usize = 0;
    let mut sel_active = start_selecting;
    let mut anch_r: usize = start;
    let mut anch_c: usize = 0;
    let mut copied_msg: Option<String> = None;

    let sel_bg = theme_to_ct(theme.selection);
    let text_fg = theme_to_ct(theme.help_fg);
    let cursor_bg = theme_to_ct(theme.cursor);
    let cursor_fg = theme_to_ct(theme.cursor_fg);
    let src_bg = crate::theme::color_to_rgb(theme.status_bar_source);

    loop {
        // Draw
        let content_rows = term.content_rows() as usize;
        let cols = term.cols as usize;
        let (sr, sc, er, ec) = if sel_active {
            if anch_r < cur_r || (anch_r == cur_r && anch_c <= cur_c) {
                (anch_r, anch_c, cur_r, cur_c)
            } else {
                (cur_r, cur_c, anch_r, anch_c)
            }
        } else {
            (usize::MAX, 0, 0, 0)
        };

        // Keep cursor in view: if cur_r is above scroll, snap to it; otherwise
        // step scroll forward until the cursor's wrapped segment is visible.
        if cur_r < scroll { scroll = cur_r; }
        let mut screen_map = build_screen_map(&lines, scroll, cols, content_rows);
        loop {
            let visible = screen_map.iter().any(|&(li, s, e)| li == cur_r && cur_c >= s && cur_c <= e);
            if visible || scroll >= total_lines.saturating_sub(1) { break; }
            scroll += 1;
            screen_map = build_screen_map(&lines, scroll, cols, content_rows);
        }

        crossterm::queue!(term.stdout_mut(), crossterm::cursor::MoveTo(0, 0))?;
        for screen_row in 0..content_rows {
            crossterm::queue!(term.stdout_mut(), crossterm::cursor::MoveTo(0, screen_row as u16))?;
            write!(term.stdout_mut(), "\x1b[2K")?;
            let Some(&(li, seg_start, seg_end)) = screen_map.get(screen_row) else { continue; };
            let chars: Vec<char> = lines[li].chars().collect();
            for ci in seg_start..seg_end {
                let ch = chars.get(ci).copied().unwrap_or(' ');
                let is_cursor = li == cur_r && ci == cur_c;
                let is_sel = sel_active && li >= sr && li <= er && {
                    if sr == er { ci >= sc && ci < ec }
                    else if li == sr { ci >= sc }
                    else if li == er { ci < ec }
                    else { true }
                };
                if is_cursor {
                    crossterm::queue!(term.stdout_mut(), SetBackgroundColor(cursor_bg), SetForegroundColor(cursor_fg))?;
                } else if is_sel {
                    crossterm::queue!(term.stdout_mut(), SetBackgroundColor(sel_bg), SetForegroundColor(text_fg))?;
                } else {
                    crossterm::queue!(term.stdout_mut(), ResetColor)?;
                }
                write!(term.stdout_mut(), "{}", ch)?;
            }
            // Cursor at end-of-line position falls just past seg_end on the
            // last segment of its source line — paint it in the next column.
            if li == cur_r && cur_c == seg_end && cur_c == chars.len() {
                crossterm::queue!(term.stdout_mut(), SetBackgroundColor(cursor_bg), SetForegroundColor(cursor_fg))?;
                write!(term.stdout_mut(), " ")?;
                crossterm::queue!(term.stdout_mut(), ResetColor)?;
            }
        }
        crossterm::queue!(term.stdout_mut(), ResetColor)?;

        let content_rows = term.content_rows() as usize;
        let pct = if total_lines > 0 { (scroll + content_rows).min(total_lines) * 100 / total_lines } else { 100 };
        let line_info = format!("L{} {}%", cur_r + 1, pct);

        if sel_active {
            let sel_bg = crate::theme::color_to_rgb(theme.status_bar_select);
            let fill_bg = crate::theme::color_to_rgb(theme.status_info_bg);
            let file_text = if let Some(ref msg) = copied_msg {
                format!("{} — {}", filename, msg)
            } else {
                filename.to_string()
            };
            draw_status_panes(term,
                &[pane_mode("SELECT", theme, sel_bg), pane_file(&file_text, theme)],
                &[pane_keys("y=copy esc=cancel", theme), pane_progress(&line_info, sel_bg)],
                fill_bg, crate::theme::color_to_rgb(theme.background),
            )?;
        } else {
            let fill_bg = crate::theme::color_to_rgb(theme.status_info_bg);
            let file_text = if let Some(ref msg) = copied_msg {
                format!("{} — {}", filename, msg)
            } else {
                filename.to_string()
            };
            draw_status_panes(term,
                &[pane_mode("SOURCE", theme, src_bg), pane_file(&file_text, theme)],
                &[pane_keys("v=select esc=back", theme), pane_progress(&line_info, src_bg)],
                fill_bg, crate::theme::color_to_rgb(theme.background),
            )?;
        }
        copied_msg = None;
        term.flush()?;

        // Input
        let ev = event::read()?;
        if let Event::Mouse(mouse) = &ev {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    let screen_r = mouse.row as usize;
                    if let Some(&(src_li, seg_start, _seg_end)) = screen_map.get(screen_r) {
                        let line_len = lines[src_li].chars().count();
                        let cr = src_li;
                        let cc = (seg_start + mouse.column as usize).min(line_len);

                        if sel_active {
                            // Click within or near selection — adjust nearest endpoint
                            let (sr, sc, er, ec) = if anch_r < cur_r || (anch_r == cur_r && anch_c <= cur_c) {
                                (anch_r, anch_c, cur_r, cur_c)
                            } else {
                                (cur_r, cur_c, anch_r, anch_c)
                            };
                            let in_sel = (cr > sr || (cr == sr && cc >= sc)) && (cr < er || (cr == er && cc <= ec));
                            if in_sel {
                                // Keep anchor, move cursor to click position
                                cur_r = cr;
                                cur_c = cc;
                            } else {
                                // Outside selection — start new
                                cur_r = cr;
                                cur_c = cc;
                                anch_r = cr;
                                anch_c = cc;
                            }
                        } else {
                            cur_r = cr;
                            cur_c = cc;
                            sel_active = true;
                            anch_r = cr;
                            anch_c = cc;
                        }
                    }
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    let cr = term.content_rows() as usize;
                    let screen_r = mouse.row as usize;
                    if mouse.row == 0 && scroll > 0 {
                        scroll -= 1;
                        if cur_r > 0 { cur_r -= 1; }
                    } else if screen_r >= cr.saturating_sub(1) && scroll + 1 < total_lines {
                        scroll += 1;
                        if cur_r + 1 < total_lines { cur_r += 1; }
                    } else if let Some(&(src_li, seg_start, _)) = screen_map.get(screen_r) {
                        cur_r = src_li;
                        cur_c = seg_start + mouse.column as usize;
                    }
                    let line_len = lines.get(cur_r).map_or(0, |l| l.chars().count());
                    cur_c = cur_c.min(line_len);
                }
                MouseEventKind::Up(MouseButton::Left) => {}
                MouseEventKind::ScrollDown => {
                    let cr = term.content_rows() as usize;
                    scroll = (scroll + 3).min(total_lines.saturating_sub(cr));
                    // Don't move cursor — keep selection stable
                }
                MouseEventKind::ScrollUp => {
                    scroll = scroll.saturating_sub(3);
                }
                _ => {}
            }
            continue;
        }
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Esc => {
                    if sel_active { sel_active = false; } else { break; }
                }
                KeyCode::Char('q') if !sel_active => break,
                KeyCode::Char('v') => {
                    if sel_active { sel_active = false; }
                    else { sel_active = true; anch_r = cur_r; anch_c = cur_c; }
                }
                KeyCode::Char('y') if sel_active => {
                    let (sr2, sc2, er2, ec2) = if anch_r < cur_r || (anch_r == cur_r && anch_c <= cur_c) {
                        (anch_r, anch_c, cur_r, cur_c)
                    } else { (cur_r, cur_c, anch_r, anch_c) };
                    let mut text = String::new();
                    for r in sr2..=er2 {
                        if r >= total_lines { break; }
                        let lc: Vec<char> = lines[r].chars().collect();
                        let s = if r == sr2 { sc2 } else { 0 };
                        let e = if r == er2 { ec2 + 1 } else { lc.len() };
                        if r > sr2 { text.push('\n'); }
                        text.extend(&lc[s..e.min(lc.len())]);
                    }
                    if !text.is_empty() {
                        let _ = term.copy_to_clipboard(&text);
                        copied_msg = Some(format!("Copied {} chars", text.len()));
                    }
                    sel_active = false;
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    if cur_c > 0 {
                        cur_c -= 1;
                    } else if cur_r > 0 {
                        // Wrap to end of previous line
                        cur_r -= 1;
                        cur_c = lines.get(cur_r).map_or(0, |l| l.chars().count());
                    }
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    let len = lines.get(cur_r).map_or(0, |l| l.chars().count());
                    if cur_c < len { cur_c += 1; }
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if cur_r + 1 < total_lines {
                        cur_r += 1;
                        cur_c = cur_c.min(lines.get(cur_r).map_or(0, |l| l.chars().count()));
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if cur_r > 0 {
                        cur_r -= 1;
                        cur_c = cur_c.min(lines.get(cur_r).map_or(0, |l| l.chars().count()));
                    }
                }
                KeyCode::Char('w') => {
                    let chars: Vec<char> = lines.get(cur_r).unwrap_or(&"").chars().collect();
                    let mut i = cur_c;
                    while i < chars.len() && !chars[i].is_whitespace() { i += 1; }
                    while i < chars.len() && chars[i].is_whitespace() { i += 1; }
                    if i < chars.len() { cur_c = i; }
                    else if cur_r + 1 < total_lines { cur_r += 1; cur_c = 0; }
                }
                KeyCode::Char('b') => {
                    if cur_c == 0 && cur_r > 0 {
                        cur_r -= 1;
                        cur_c = lines.get(cur_r).map_or(0, |l| l.chars().count()).saturating_sub(1);
                    } else {
                        let chars: Vec<char> = lines.get(cur_r).unwrap_or(&"").chars().collect();
                        let mut i = cur_c.saturating_sub(1);
                        while i > 0 && chars[i].is_whitespace() { i -= 1; }
                        while i > 0 && !chars.get(i - 1).map_or(true, |c| c.is_whitespace()) { i -= 1; }
                        cur_c = i;
                    }
                }
                KeyCode::Char('0') | KeyCode::Home => cur_c = 0,
                KeyCode::Char('$') | KeyCode::End => {
                    cur_c = lines.get(cur_r).map_or(0, |l| l.chars().count());
                }
                KeyCode::Char('g') if !sel_active => { cur_r = 0; cur_c = 0; }
                KeyCode::Char('G') if !sel_active => { cur_r = total_lines.saturating_sub(1); cur_c = 0; }
                KeyCode::PageDown => {
                    cur_r = (cur_r + content_rows).min(total_lines.saturating_sub(1));
                }
                KeyCode::PageUp => { cur_r = cur_r.saturating_sub(content_rows); }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                _ => {}
            }
            // Top edge: snap. Bottom edge handled by visibility loop in next draw.
            if cur_r < scroll { scroll = cur_r; }
        }
    }
    Ok(())
}

/// Build a per-screen-row mapping of (source_line, segment_start_char, segment_end_char)
/// for source-mode rendering with line wrapping.
fn build_screen_map(lines: &[&str], scroll: usize, cols: usize, content_rows: usize) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::with_capacity(content_rows);
    if cols == 0 { return out; }
    let mut li = scroll;
    while out.len() < content_rows && li < lines.len() {
        let chars_count = lines[li].chars().count();
        if chars_count == 0 {
            out.push((li, 0, 0));
        } else {
            let mut i = 0;
            while i < chars_count && out.len() < content_rows {
                let end = (i + cols).min(chars_count);
                out.push((li, i, end));
                i = end;
            }
        }
        li += 1;
    }
    out
}
