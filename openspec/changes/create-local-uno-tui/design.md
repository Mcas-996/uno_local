## Context

The repository currently separates gameplay, protocol, networking, rooms, and a line-oriented CLI. Only the host debug prompt can play locally, while the join path ends after a handshake. The replacement must work on Windows, macOS, and Linux, restore terminal state reliably, and keep game rules independent from presentation and AI policy.

## Goals / Non-Goals

**Goals:**

- Launch directly into an offline terminal game for one human and one to four local AI players.
- Make rules expose enough state for both a human UI and AI without duplicating validation.
- Provide deterministic AI tests while keeping real matches shuffled and varied.
- Support Chinese and English presentation selected from the operating-system locale.

**Non-Goals:**

- Networking, rooms, accounts, remote AI, persistence, matchmaking, or telemetry.
- UNO declarations and penalties, Wild Draw Four challenges, multi-round scoring, or house rules.
- Mouse-first interaction or graphical desktop rendering.

## Decisions

### Keep rules, AI, and TUI as separate modules

The `core` module owns authoritative rules and legal-action discovery, `ai` selects only from legal actions, and `app` plus `ui` own application state and rendering. These module boundaries prevent UI or AI code from mutating card state directly without imposing separate-package overhead. Strategy tests remain colocated with the AI module and independent of terminal rendering.

### Model the post-draw decision explicitly

The game tracks whether the current player has drawn and which card was drawn. Before drawing, a player can play any legal card or draw. After drawing, only that card can be played; otherwise the player passes. This replaces the current unrestricted draw/pass commands and gives the UI and AI a single legal-action source.

### Inject randomness

Game construction and AI decisions accept an RNG, while convenience constructors use thread-local randomness. Tests use seeded RNGs. Discard recycling shuffles every discarded card except the visible top card.

### Use scored local AI policies

Easy chooses randomly, normal scores color continuity and action cards while preserving wilds, and hard adds opponent-pressure weighting. Strategies receive public state plus only their own hand and never access hidden opponent cards. AI turns are scheduled by the TUI event loop after a short deadline rather than run on network or worker services.

### Use a reducer-like TUI state

Input events are translated into application actions, applied to an `App`, and then rendered. Ratatui's `TestBackend` can therefore verify screens without a real terminal. Crossterm raw/alternate-screen setup is guarded so normal exits and panics restore the terminal.

### Centralize localization

UI messages are represented by message keys. A locale detector maps `zh*` to Simplified Chinese and all other or unknown locales to English. Stable command words remain English, while help text explains them in the selected language.

## Risks / Trade-offs

- Terminal capabilities and color palettes vary → show card color as text as well as styling and provide a minimum-size screen.
- Hard AI is heuristic rather than game-theoretic → guarantee legality and documented preferences, not optimal play.
- Removing network crates is a breaking change → update help, README, OpenSpec, and release metadata together.
- Event history can grow during long games → keep only a bounded UI log while the small core event list remains acceptable for a single round.

## Migration Plan

Remove the network-oriented crates and CLI paths, consolidate the application into one package, add the AI and TUI dependencies, update the lockfile, and replace the old active OpenSpec change after verification. No user data migration is required. Rollback is a source-level revert because the application stores no persistent state.

## Open Questions

None.
