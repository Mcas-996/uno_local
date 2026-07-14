## 1. Graphics Foundation

- [x] 1.1 Upgrade Ratatui and Crossterm, add `ratatui-image` and `image` with no Chafa runtime dependency, and confirm every release target builds
- [x] 1.2 Add Auto/Text setup choice, effective backend and fallback-reason types, plus pure tests for SSH, WezTerm-before-WT, Windows Terminal, other-terminal, and failure resolution
- [x] 1.3 Initialize terminal capability detection after alternate-screen entry but before event reads, skip it for SSH, and downgrade query or protocol-construction errors to Text

## 2. Generated Card Art and Caching

- [x] 2.1 Implement RGBA drawing primitives and built-in glyphs for rounded colored cards, number/action symbols, Wild quadrants, and penalty markings
- [x] 2.2 Generate artwork for every Standard and Holiday `Card` and add pixel/dimension tests covering colors, actions, Wild, Draw Eight, and Wild Draw Sixteen
- [x] 2.3 Build the graphics runtime with a base-art cache and separate selected/discard protocol slots that re-encode only when card, backend, or target size changes

## 3. Responsive TUI Integration

- [x] 3.1 Add the localized Graphics setup row, Auto/Text keyboard handling, and effective labels for iTerm2, Sixel, Kitty, manual Text, SSH Text, and unsupported Text
- [x] 3.2 Preserve the current text table from 70x22 through 70x25 and add the 70x26-or-larger side-by-side selected/discard image panels with localized card names and active color
- [x] 3.3 Suspend and clear image placements for overlays, resize threshold crossings, new matches, backend changes, empty human hands, normal exit, and panic restoration

## 4. Verification and Documentation

- [x] 4.1 Extend App and Ratatui TestBackend coverage for the new setup selection, text fallback, responsive threshold, bilingual labels, overlays, and unchanged gameplay navigation
- [x] 4.2 Extend the manual test matrix for WezTerm iTerm2 and Windows Terminal 1.22+ Sixel in native PowerShell and WSL, plus SSH-forced Text, resize, overlay, and exit cleanup
- [x] 4.3 Run the full test suite and Clippy on all targets available locally, resolve regressions, and document any platform-only checks that require CI or manual terminals
