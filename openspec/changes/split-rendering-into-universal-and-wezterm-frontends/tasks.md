## 1. Shared application boundary

- [x] 1.1 Add frontend-neutral input, viewport, display capability, and semantic view types
- [x] 1.2 Refactor App and its tests away from Crossterm and the legacy graphics runtime

## 2. Universal Crossterm frontend

- [x] 2.1 Implement terminal environment classification, bounded Sixel response parsing, and default display policy
- [x] 2.2 Implement the universal cell buffer, layout, Crossterm event loop, and terminal lifecycle
- [x] 2.3 Implement direct icy_sixel encoding, caching, placement, invalidation, and cleanup

## 3. WezTerm Termwiz frontend

- [x] 3.1 Implement the Termwiz Surface layout and native input event loop
- [x] 3.2 Implement cached PNG EncodedFile card images with Termwiz text fallback
- [x] 3.3 Implement runtime dispatch and state-preserving Termwiz-to-universal fallback

## 4. Remove the legacy renderer

- [x] 4.1 Remove Ratatui, ratatui-image, base64-simd, and all application-owned Kitty/iTerm2 code
- [x] 4.2 Replace legacy UI/graphics tests with shared, universal, Termwiz, and dispatch coverage

## 5. Documentation and verification

- [x] 5.1 Update Chinese and English development, graphics architecture, README, and manual terminal matrix documentation
- [x] 5.2 Run formatting, all-target tests/checks, Clippy, dependency-tree checks, and dist planning
