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

Does exactly what `render_styled_text` does through line 164 (create Buffer, set_rich_text, shape_until_scroll, count layout_runs), then returns the height. No pixel buffer allocated, no rasterization.

### measure_block_height (Viewport)

Replaces `estimate_block_height`. Match on block type:

- **Heading**: `measure_text_height` with heading font_size and line_height 1.15
- **Paragraph**: `measure_text_height` with body font_size and line_height 1.4
- **CodeBlock**: `measure_text_height` with code font, `inner_width = width_px - inset*2`, line_height 1.3, padding. Prepend language label if present (same as render).
- **List**: For each item: `measure_text_height` for item spans + recursive `measure_block_height` for children. Sum with item gaps.
- **BlockQuote**: Recursive `measure_block_height` for children at reduced width + padding.
- **Table**: Exact arithmetic (fixed row height * count + borders). No change needed.
- **Image**: Rough height (corrected on render — images need to be loaded to know dimensions).
- **ThematicBreak**: `font_size as u32`. No change needed.

Note: `measure_block_height` takes `&mut self` because it needs the renderer's FontSystem.

### Measurement tracking

Add to `Viewport`:

```rust
next_unmeasured: usize,  // index of next block to measure in background
```

Starts at 0. After startup measurement of the first viewport, set to first-block-past-viewport. Background measurement increments this until it reaches `blocks.len()`.

### Startup flow

1. Create Viewport with all block_heights initialized via `estimate_block_height` (instant — pure arithmetic, gives approximate total_height for scrollbar)
2. Compute initial block_offsets from estimates
3. Determine first viewport: blocks 0..N where N fills the screen
4. Measure blocks 0..N via `measure_block_height` — replace estimates with exact heights
5. Recompute block_offsets
6. Render and display blocks 0..N
7. Set `next_unmeasured = N`
8. Background measurement progressively replaces remaining estimates

### Background measurement (idle loop)

The event loop changes from:

```rust
loop {
    let ev = event::read()?;
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
    } else {
        // Idle — measure a batch of blocks
        let scroll_delta = viewport.measure_batch(&blocks, 10);
        if scroll_delta != 0 && measured_block_was_above_viewport {
            scroll_row = (scroll_row as i64 + scroll_delta) as u32;
        }
    }
}
```

`measure_batch(blocks, count)` measures up to `count` blocks starting at `next_unmeasured`, updates `block_heights` and `block_offsets`, returns the cumulative offset delta for blocks above the current viewport.

### Scroll position stability

When a background measurement changes a block's height above the viewport:
- `scroll_row` shifts by the same delta
- Visible content stays the same — the user sees no jump

When a block below the viewport changes height:
- Only `total_height` and downstream offsets change
- No visible effect until the user scrolls there

### Resize

On terminal resize:
- Re-measure ALL blocks at new width (reshaping needed anyway since wrap width changed)
- This is synchronous — resize is infrequent and the user expects a brief pause
- Rebuild offsets, clear block_cache, retransmit visible blocks

### Cleanup

- Keep `estimate_block_height` for initial approximate heights (used before measurement completes)
- Remove the height correction logic in `ensure_block_rendered` (lines 139-147) once all blocks are measured before rendering — but keep it as a safety net during the window between startup and background measurement completion
- Keep the padded-to-cell-height logic (lines 122-137) — that's for Kitty protocol alignment, not height estimation

## Files to modify

- `src/graphics.rs` — add `measure_text_height` method to TextRenderer
- `src/viewport.rs` — replace `estimate_block_height` with `measure_block_height`, add `next_unmeasured` field, add `measure_batch` and `has_unmeasured` methods
- `src/app_new.rs` — change event loop to poll-based with idle measurement, adjust scroll_row on background measurement

## Verification

1. `cargo build` — no errors
2. `cargo test` — all tests pass
3. Open a large markdown file with long code blocks — no overlapping blocks
4. Scroll through the entire document — no visual glitches
5. Resize terminal — content reflows correctly
6. Open a small file — instant startup, no perceptible delay
