## Context

The setup currently stores `GraphicsChoice::Auto` by default on every launch. `GraphicsRuntime` separately detects SSH, WezTerm, Windows Terminal, and supported image protocols before the application state is created. The new policy changes the initial user choice without removing any existing local protocol support or the one-time startup query.

## Goals / Non-Goals

**Goals:**

- Make Text the conservative default except in Windows Terminal.
- Treat WSL hosted by Windows Terminal as Windows Terminal, while excluding WezTerm even when it inherits `WT_SESSION`.
- Present graphics as an explicit beta choice and retain manual opt-in on all local terminals.
- Keep SSH query avoidance and forced text rendering intact.

**Non-Goals:**

- Persisting the user's graphics preference between launches.
- Removing WezTerm, Kitty, iTerm2, or Sixel support.
- Changing image layout, card art, or minimum terminal dimensions.

## Decisions

### Derive the initial choice from the existing terminal environment model

Add a pure default-choice policy next to `TerminalEnvironment`: SSH and WezTerm choose Text; otherwise a non-empty `WT_SESSION` chooses Graphics Beta; every other environment chooses Text. This keeps environment precedence centralized and makes the native Windows/WSL/WezTerm matrix directly testable. Checking the compile target was rejected because it would incorrectly exclude WSL hosted by Windows Terminal.

### Keep backend detection separate from the setup choice

Continue querying graphics capabilities once at local startup and retaining the detected backend. Text suppresses rendering through `effective_backend`; selecting Graphics Beta exposes the cached backend without querying during the event loop. Lazy detection was rejected because synchronous terminal queries after input handling could complicate event ownership and make the first toggle block unpredictably.

### Replace Auto rather than add a third mode

Rename the internal choice to `GraphicsBeta` and expose only Text and Graphics Beta. Graphics Beta uses the same restricted backend resolver and fallback reasons as Auto did. Text copy no longer says `manual`, because Text can now be the environment-derived default.

## Risks / Trade-offs

- [Local Text-default sessions still receive a capability query at startup] → Preserve the agreed one-time behavior and continue skipping every query in SSH sessions.
- [Forged or inherited `WT_SESSION` can influence the default] → Give SSH and WezTerm explicit precedence and display the effective backend/fallback in setup.
- [Renaming an enum variant touches tests and documentation broadly] → Keep the backend types unchanged and update exact localized/UI assertions together.

## Migration Plan

Update the choice type and pure default policy first, wire the detected environment's recommendation into application construction, then update copy, tests, and documentation. No stored data migration is required. Rollback restores the Auto variant and unconditional Auto initialization.

## Open Questions

None.
