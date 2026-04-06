# Exact Block Height Measurement

## Problem

Block heights are estimated with a character-width heuristic (`chars * 0.65 * font_size`). When the actual rendered height differs (common for long wrapped lines in code blocks, paragraphs, and lists), blocks below shift and overlap because already-transmitted images remain at stale positions.

The root cause: estimation is fundamentally unreliable for variable-width text with wrapping. Industry standard (browsers, xi-editor, VS Code) is to measure all heights upfront using the actual text layout engine.

## Solution

Replace `estimate_block_height` with `measure_block_height` that uses cosmic-text's shaping/layout pass to get exact pixel heights without rasterization. Measure the first viewport synchronously at startup, then measure remaining blocks during idle time.

## Architecture

### measure_text_height (TextRenderer)

New method on `TextRenderer`:

```rust
fn measure_text_height(
    &mut self,
    spans: &[StyledTextSpan],
    width_px: u32,
    opts: &RenderOptions,
) -> u32
```

Duplicates lines 131-164 of `render_styled_text`: creates a cosmic-text Buffer, calls set_rich_text + shape_until_scroll, counts layout_runs, returns `(line_height * line_count) + padding_top + padding_bottom`. No pixel buffer allocated, no rasterization, no highlight/link processing.

### measure_block_height (Viewport)

Replaces `estimate_block_height`. Takes `&mut self` (needs renderer's FontSystem). Match on block type, replicating the exact width/padding/font parameters used by `render_block_image`:

- **Heading**: `measure_text_height` with `font_size * level.font_scale()`, `line_height_factor: 1.15`
- **Paragraph**: `measure_text_height` with body `font_size`, `line_height_factor: 1.4`
- **CodeBlock**: Build the same spans as render (prepend `"{lang}\n\n"` if language present, then code text). Call `measure_text_height` with `use_code_font: true`, `inner_width = width_px - inset*2` where `inset = font_size * 2.0`, `line_height_factor: 1.3`, `padding_left: padding`, `padding_top: padding/2`, `padding_bottom: padding` where `padding = font_size * 0.8`.
- **List**: For each item: `measure_text_height` for item spans + trailing `\n` span, with `padding_left: bullet_indent` where `bullet_indent = font_size * 1.5`, `line_height_factor: 1.4`. Then for each child block: recursively `measure_block_height` at `width_px - bullet_indent`. Sum all heights (no gaps between items — they stack directly).
- **BlockQuote**: `content_width = width_px - inset*2 - bar_width - bar_gap` where `bar_width=4`, `bar_gap = font_size*0.5`, `inset = font_size*2.0`, `pad_v = font_size*0.4`. Recursively measure children at content_width, sum with `child_gap = font_size*0.3` between children, add `2 * pad_v` for vertical padding.
- **Table**: Exact arithmetic (unchanged): `(1 + rows.len()) * row_h + (2 + rows.len())` where `row_h = line_height + 9`.
- **Image**: `(font_size * 10.0) as u32` — rough estimate, corrected on render when actual image dimensions are known.
- **ThematicBreak**: `cell_height as u32`.

After computing the raw height, pad to a multiple of `cell_height`: `((h + cell_height - 1) / cell_height) * cell_height`. This matches the padding applied in `ensure_block_rendered` (lines 122-137) so measurement and render agree.

### Measurement tracking

Add to `Viewport`:

```rust
next_unmeasured: usize,  // index of next block to measure in background
```

Starts at 0. After startup measurement of the first viewport, set to first-block-past-viewport. Background measurement increments this until it reaches `blocks.len()`.

### Startup flow

1. Create Viewport with all `block_heights` initialized via `estimate_block_height` (instant — pure arithmetic, gives approximate `total_height` for scrollbar)
2. Compute initial `block_offsets` from estimates
3. Determine first viewport: blocks 0..N where cumulative height fills the screen
4. Measure blocks 0..N via `measure_block_height` — replace estimates with exact heights
5. Recompute `block_offsets` from block 0
6. Render and display blocks 0..N
7. Set `next_unmeasured = N`
8. Background measurement progressively replaces remaining estimates

### Background measurement (idle loop)

The event loop changes from:

```rust
loop {
    let ev = event::read()?;  // blocks forever
    // handle...
}
```

To:

```rust
loop {
    let timeout = if viewport.has_unmeasured() {
        Duration::from_millis(10)
    } else {
        Duration::MAX  // block forever
    };

    if event::poll(timeout)? {
        let ev = event::read()?;
        // handle event...
    } else if viewport.has_unmeasured() {
        // Idle — measure a batch of blocks
        viewport.measure_batch(&blocks, 10);
        // Adjust scroll if measured blocks were above viewport
        let measured_end = viewport.next_unmeasured;
        let first_visible = viewport.block_offsets
            .partition_point(|&off| off <= scroll_row as u32 * cell_h as u32)
            .saturating_sub(1);
        // If any measured block is above the viewport, the offset shift
        // is already applied to block_offsets — scroll_row needs to track
        // the same content, so recompute from the first visible block's offset
        scroll_row = viewport.block_offsets[first_visible] / cell_h as u32;
    }
}
```

`measure_batch(blocks, count)` measures up to `count` blocks starting at `next_unmeasured`, updates `block_heights` and recomputes `block_offsets` for all blocks from `next_unmeasured` onward.

### Scroll position stability

After background measurement, `scroll_row` is recomputed from the first visible block's new offset. This ensures the user sees the same content regardless of offset shifts from measurement corrections.

### Resize

On terminal resize:
- Re-measure ALL blocks at new width (text needs reshaping at new wrap width)
- This is synchronous — resize is infrequent and users expect a brief pause
- Rebuild offsets, clear `block_cache`, retransmit visible blocks
- Reset `next_unmeasured = blocks.len()` (all measured)

### Safety net

Keep the height correction logic in `ensure_block_rendered` (lines 139-147) as a safety net for:
- Image blocks (height unknown until loaded)
- Any edge case where measurement and render disagree
- The window between startup and background measurement completion when the user scrolls past measured blocks

When a correction fires and shifts offsets, invalidate all transmitted blocks below (remove from `transmitted` set) so they retransmit at correct positions on next redraw.

### Cleanup

- Keep `estimate_block_height` for initial approximate heights before measurement completes
- Keep padded-to-cell-height logic in `ensure_block_rendered` (lines 122-137) — Kitty protocol alignment

## Files to modify

- `src/graphics.rs` — add `measure_text_height` method to TextRenderer
- `src/viewport.rs` — add `measure_block_height`, `next_unmeasured` field, `measure_batch`, `has_unmeasured` methods
- `src/app_new.rs` — change event loop to poll-based with idle measurement, scroll position adjustment after measurement

## Verification

1. `cargo build` — no errors
2. `cargo test` — all tests pass
3. Open the tenaya CLAUDE.md (long code blocks with wrapped lines) — no overlapping blocks
4. Scroll through the entire document — no visual glitches or jumps
5. Resize terminal — content reflows correctly
6. Open a small file — instant startup, no perceptible delay
7. Open a very large file (1000+ blocks) — first page appears instantly, scrollbar updates smoothly as background measurement completes
