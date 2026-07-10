## Why

The network-first CLI is cumbersome for a local game and leaves the primary join flow unable to play a full match. The project will become a self-contained terminal UNO game so one player can immediately play complete matches against local AI without rooms, networking, or external services.

## What Changes

- **BREAKING** Remove the host/join CLI, rooms, share strings, UDP/STUN connectivity, host failover, and the `uno_net`, `uno_protocol`, and `uno_room` crates.
- Add a cross-platform Ratatui/Crossterm interface that launches by default and supports both keyboard navigation and a command bar.
- Add local matches for one human against one to four in-process AI opponents with easy, normal, and hard difficulty levels.
- Complete the core rules needed to finish a basic UNO round, including draw/pass phases, action cards, draw-pile recycling, randomized shuffling, and win detection.
- Add system-locale-based Chinese and English UI text, with English as the fallback.
- Replace networking documentation and the unfinished network-oriented OpenSpec change with local-game documentation and specifications.

## Capabilities

### New Capabilities

- `local-uno-game`: Complete offline UNO rounds for one human and one to four AI opponents.
- `local-uno-ai`: In-process AI opponents with selectable difficulty and always-legal decisions.
- `uno-tui`: Cross-platform terminal setup, gameplay, help, command, and result screens.

### Modified Capabilities

None.

## Impact

- The five-crate workspace contracts to one `uno` package, with rules, AI, application state, localization, and rendering kept as source modules.
- The `uno` command no longer accepts host/join networking arguments and opens the TUI by default.
- The `core` module gains explicit turn-phase and legal-action APIs used by both the TUI and AI.
- New runtime dependencies include Ratatui, Crossterm, random-number support, and system locale detection.
