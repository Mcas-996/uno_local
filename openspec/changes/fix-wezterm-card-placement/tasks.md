## 1. Placement-aware graphics runtime

- [x] 1.1 Add a non-encoding fitted-size API and pass UI-centered final rectangles into protocol generation
- [x] 1.2 Extend selected/discard protocol cache keys from card plus size to card plus final rectangle
- [x] 1.3 Wrap verified local non-tmux WezTerm iTerm2 data with absolute anchor and next-cell cursor positions
- [x] 1.4 Fail closed to `Text(Encoding)` and clear both preview caches when required WezTerm wrapping is unsafe

## 2. Regression coverage

- [x] 2.1 Test fitted and centered selected/discard rectangles remain inside their panels
- [x] 2.2 Test WezTerm anchor/restore bytes, independent slot positions, and rectangle-aware cache reuse/invalidation
- [x] 2.3 Test ordinary iTerm2, Sixel, Kitty, tmux, text mode, and malformed WezTerm fallback behavior

## 3. Dependency fallback and documentation

- [x] 3.1 Copy the complete upstream `ratatui-image` v11.0.6 source snapshot to `external/ratatui-image/` without repository/build artifacts and leave the crates.io dependency active
- [x] 3.2 Document snapshot provenance, maintenance and emergency patch rules, updated image architecture, and the manual terminal matrix without changing `docs/extra_info.md`

## 4. Verification

- [x] 4.1 Run `cargo fmt --check`, `cargo check`, `cargo test`, and strict Clippy with all warnings denied
- [x] 4.2 Verify the working tree keeps `Cargo.toml` on crates.io and records no change to `docs/extra_info.md`

## 5. Application-controlled WezTerm lifecycle

- [x] 5.1 Build `PreviewPlan` before `Terminal::draw` and share its final rectangles between UI reservation and output
- [x] 5.2 Atomically encode both WezTerm slots, clear changed old rectangles before Ratatui, and emit changed images afterward
- [x] 5.3 Preserve unchanged slots, independently handle replacement/movement/deletion, and full-clear on resize
- [x] 5.4 Clear on overlays, text mode, undersized terminals, fallback, and exit without partial output
- [x] 5.5 Cover lifecycle diffs, absolute CUP/ECH ordering, atomic failure, and 70x26/159x41 centering with automated tests
