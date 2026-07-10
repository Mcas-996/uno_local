## 1. Workspace and Core Rules

- [x] 1.1 Replace the multi-crate workspace with one package and add TUI, RNG, locale, and AI dependencies
- [x] 1.2 Refactor the game model to expose legal actions and enforce the post-draw turn phase
- [x] 1.3 Complete action-card behavior, Wild Draw Four validation, randomized setup, discard recycling, and round completion
- [x] 1.4 Add comprehensive deterministic core rule tests

## 2. Local AI

- [x] 2.1 Add an `ai` module with easy, normal, and hard difficulty policies
- [x] 2.2 Ensure AI uses only public state and its own hand and handles draw/play/pass sequences
- [x] 2.3 Add seeded tests for legality, reproducibility, difficulty preferences, and opponent-pressure behavior

## 3. Terminal UI

- [x] 3.1 Replace the host/join CLI with a default Ratatui/Crossterm application and guarded terminal lifecycle
- [x] 3.2 Implement localized setup, table, help, command bar, color chooser, result, quit, and minimum-size screens
- [x] 3.3 Connect shortcuts and command parsing to human actions and schedule non-blocking local AI turns
- [x] 3.4 Add reducer, command parser, locale, and TestBackend rendering tests

## 4. Cleanup and Verification

- [x] 4.1 Remove obsolete networking documentation and the unfinished network OpenSpec change, and rewrite README/manual guidance for the local game
- [x] 4.2 Run formatting, tests, Clippy with warnings denied, and release build checks
