## 1. Backend Resolution and Runtime

- [x] 1.1 Reduce `GraphicsBackend` and localized UI labels to Sixel or reasoned Text outcomes
- [x] 1.2 Implement SSH short-circuit and detected-Sixel-only resolution for every local terminal
- [x] 1.3 Remove WezTerm IIP positioning, escape-data validation, and Kitty/IIP compatibility paths
- [x] 1.4 Key selected and discard protocol caches by card and fitted size while preserving stable encoding-failure cleanup and no-retry behavior

## 2. Automated Coverage

- [x] 2.1 Update backend-resolution and picker tests for compatibility-respecting Sixel-only local terminal acceptance
- [x] 2.2 Update UI and localization tests so settings can display only Sixel graphics or the applicable Text state
- [x] 2.3 Cover cache reuse across position changes, invalidation for card/size changes, slot independence, and terminal encoding failure

## 3. Repository and Documentation

- [x] 3.1 Delete the inactive `external/ratatui-image/` snapshot while retaining the crates.io `ratatui-image 11.0.6` dependency
- [x] 3.2 Update Chinese and English development documentation for Sixel-only Graphics Beta and remove external snapshot guidance
- [x] 3.3 Update graphics architecture and manual-test documentation for the Sixel/Text flow, cache key, fallback rules, and real-terminal matrix
- [x] 3.4 Confirm current source, tests, UI labels, and active docs no longer claim Kitty or IIP application support

## 4. Verification

- [x] 4.1 Run `cargo fmt --check` and `cargo check`
- [x] 4.2 Run `cargo test`
- [x] 4.3 Run `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 4.4 Record the WezTerm, Windows Terminal, tmux, non-Sixel, and SSH manual acceptance matrix for release testing

## 5. WezTerm Compatibility Regression

- [x] 5.1 Remove the WezTerm Sixel override and resolve its blacklisted picker result to `Text(Unsupported)`
- [x] 5.2 Replace forced-WezTerm-Sixel tests with regression coverage that retains no picker or protocol cache in the Text fallback
- [x] 5.3 Correct active documentation and rerun formatting, build, tests, Clippy, and strict OpenSpec validation
