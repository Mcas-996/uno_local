## 1. Core hand model

- [x] 1.1 Add bounded hand storage, total-count APIs, seeded virtual batch generation, and player-specific materialization cursors
- [x] 1.2 Update play, draw, legality, victory, AI inputs, and all hand effects to use active cards or total counts as specified
- [x] 1.3 Add core tests for the 200-card bound, generation policies, immediate drawn cards, AI refill, and virtual hand effects

## 2. Interaction and presentation

- [x] 2.1 Add five-press next/previous page navigation and reset behavior for single and dual-player controls
- [x] 2.2 Render page metadata and add bilingual virtual-hand status/help text
- [x] 2.3 Replace global-index window tests with virtual page, filtering, command, and rendering coverage

## 3. Documentation and verification

- [x] 3.1 Update README and development/manual-test documentation for bounded random hand pages
- [x] 3.2 Run formatting, full tests, Clippy, and strict OpenSpec validation; fix all findings
