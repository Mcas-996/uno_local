# UNO Local TUI

A cross-platform, fully offline UNO game for one player against local AI. The application runs in the terminal, needs no account or game server, and makes no network connection during play.

## Features

- Ratatui/Crossterm interface for Windows, macOS, and Linux.
- One human player against 1–4 local AI opponents.
- Easy, normal, and hard AI difficulty levels.
- Keyboard navigation and an optional command bar.
- Simplified Chinese for `zh*` system locales and English elsewhere.
- Standard two-to-five-player card setup, action cards, wild color choices, draw/pass phases, discard recycling, and single-round win detection.

The project intentionally does not include rooms, host/join commands, networking, accounts, remote AI, UNO-call penalties, Wild Draw Four challenges, or multi-round scoring.

## Requirements

- Rust 1.91 or newer
- A terminal of at least 70 × 22 cells

## Run

```powershell
cargo run -p uno
```

Installed binaries start with:

```powershell
uno
```

Show non-interactive help:

```powershell
uno --help
```

## Controls

### Setup

- `↑` / `↓`: select player name, AI count, difficulty, or Start
- `←` / `→`: adjust AI count or difficulty
- Type and Backspace: edit the selected player name
- `Enter`: advance or start the match
- `Esc`: exit

### Match

- `←` / `→`: select a card
- `Enter`: play the selected card
- `D`: draw
- `P`: pass after drawing
- `:`: open the command bar
- `?`: help
- `Q`: quit confirmation

The command bar accepts `play <index>`, `draw`, `pass`, `help`, `new`, and `quit`.

## Rules Included

- A player may play a matching color, matching rank, or wild card.
- Playing a number card also stacks every other card of the same number from that player's hand; the selected card remains on top.
- After drawing, only the newly drawn card may be played; otherwise the player passes.
- Wild Draw Four is legal only when the player has no card matching the active color.
- Skip, Reverse, Draw Two, Wild, and Wild Draw Four are supported. Reverse acts as Skip in a two-player game.
- When the draw pile is empty, all but the top discard are shuffled into a new draw pile.
- The first player to empty their hand wins the round.

## Development

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release -p uno
```

Source layout:

```text
src/
  core.rs  authoritative cards, rules, turns, and events
  ai.rs    local easy, normal, and hard AI policies
  app.rs   application state and input handling
  i18n.rs  Chinese and English localization
  ui.rs    terminal rendering
```

## License

GNU Affero General Public License v3.0 only. See [LICENSE](LICENSE).

used cargo-dist

## Something else
For your cybersecurity, run it in a new docker if you dont want to be attacked.
