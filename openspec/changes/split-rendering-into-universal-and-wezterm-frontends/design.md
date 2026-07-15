## Context

The application currently mixes game state with Crossterm key types and mixes Ratatui layout with a graphics runtime that owns protocol selection, caching, and direct Kitty output. The checked-in WIP is test-clean but real-terminal placement and cleanup remain fragile. The replacement must retain one executable, the complete bilingual game, cargo-dist installation, and the `70 x 26` minimum while removing both Ratatui crates.

Termwiz 0.23.3 can render encoded image files through its iTerm2 renderer, but its Sixel renderer is unfinished and its RGBA iTerm2 branch is unimplemented. The universal frontend therefore needs an independent Sixel encoder, while the WezTerm frontend must pass PNG bytes as `ImageDataType::EncodedFile` and leave Termwiz's `use_image` feature disabled.

## Goals / Non-Goals

**Goals:**

- Share all game, AI, localization, semantic view data, and input actions between two terminal frontends.
- Select the safe frontend once using deterministic environment precedence and preserve game state during recoverable frontend fallback.
- Provide direct Text/Sixel rendering outside WezTerm and Termwiz-managed Text/iTerm2 rendering in local WezTerm.
- Make image cleanup, overlay suspension, resize, encoding failure, and terminal restoration testable.

**Non-Goals:**

- Pixel-identical layouts between frontends.
- Graphics over SSH or tmux.
- Public frontend-selection flags, persisted display preferences, or multiple release artifacts.
- Fixing or forking Termwiz, icy_sixel, Ratatui, or ratatui-image.

## Decisions

### One package, one binary, two internal frontends

`main` retains the current CLI and constructs one frontend-neutral `App`. Runtime environment classification uses SSH first, tmux second, local WezTerm third, and universal otherwise. SSH and tmux force universal Text; local WezTerm selects Termwiz; every other local terminal selects universal. A multi-binary or long-lived branch design was rejected because it would duplicate installation and allow game behavior to drift.

### Shared semantic application boundary

The shared layer defines neutral key/modifier/input and viewport types plus a semantic view model for setup, game, overlays, and result screens. Frontends translate native events into shared input and lay out the same view data independently. Display choice remains shared setup state, while capability/default/effective-backend policy is supplied by the active frontend.

### Crossterm retained renderer for universal terminals

The universal frontend owns a minimal cell buffer, styles, borders, wrapping, diff output, and terminal guard. It queries primary device attributes, cell pixel size (`CSI 16 t`), and device status once with a 250 ms deadline. Sixel is available only when both protocol support and nonzero cell pixels are confirmed. It encodes resized RGBA art through exactly pinned `icy_sixel 0.5.0`, caches by card/cell-size/target-size, clears changed rectangles, flushes text, and then places Sixel with saved cursor plus absolute CUP. Arbitrary fallback cell dimensions and repeated queries were rejected because they can mis-size output or consume user input.

### Termwiz Surface renderer for local WezTerm

The WezTerm frontend uses exactly pinned `termwiz 0.23.3` `BufferedTerminal`, `Surface`, and input APIs without widgets or `use_image`. Card art is cached as PNG bytes and passed explicitly as `ImageDataType::EncodedFile` in `Change::Image`; this keeps image placement in Termwiz and avoids its unimplemented RGBA path. A missing image capability or PNG encoding failure disables images for the process and redraws Termwiz text. Initial terminal construction or later terminal I/O failure restores the terminal and transfers the same `App` to universal Text.

### Cleanup and fallback are frontend lifecycle operations

Each frontend owns raw mode, alternate screen, cursor visibility, repaint, and exit cleanup behind a guard. Overlays and undersized screens never include image operations. Resize performs a full clear and invalidates placement caches. Panic cleanup remains process-wide. Direct Kitty output and tmux passthrough are deleted.

## Risks / Trade-offs

- [The custom universal buffer recreates a subset of TUI behavior] -> Keep primitives deliberately small and cover every page at minimum and large dimensions with snapshots.
- [Terminal query replies can be fragmented or mixed with input] -> Use a bounded byte parser before the event loop, terminate on DSR, and treat every parse/timeout error as Text.
- [Immediate-mode Sixel can outlive text cells] -> Track old rectangles, clear before repaint, suppress images under overlays, and clear on resize/exit.
- [Termwiz APIs and image behavior are unstable] -> Pin 0.23.3 exactly and capture renderer output in tests before any upgrade.
- [Termwiz failure after a game starts can lose presentation state] -> Keep `App` outside the frontend and make failure return ownership for universal Text fallback.

## Migration Plan

1. Preserve the current WIP as a checkpoint commit.
2. Add shared input/view boundaries while the existing tests still run.
3. Add and verify the universal frontend, then add Termwiz and dispatcher tests.
4. Delete Ratatui/ratatui-image/application Kitty code and replace UI tests.
5. Update active documentation and the terminal manual matrix, then run all-target and release checks.

Rollback is the checkpoint commit; no data or persisted configuration migration is required.

## Open Questions

None.
