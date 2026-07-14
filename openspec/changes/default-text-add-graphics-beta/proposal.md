## Why

Automatic terminal graphics are still experimental and can produce compatibility issues outside Windows Terminal. The setup should make text rendering the conservative default while keeping graphics available as an explicit beta option.

## What Changes

- Replace the `Auto` graphics choice with explicit `Text` and `Graphics (Beta)` choices.
- Default to `Graphics (Beta)` only in Windows Terminal, including WSL; default to `Text` in WezTerm and all other terminals and operating systems.
- Preserve SSH's forced text behavior and existing protocol detection/fallback safety.
- Allow users in any local terminal to opt into `Graphics (Beta)` manually.
- Update localized setup copy, automated coverage, and terminal documentation.

## Capabilities

### New Capabilities

- `graphics-beta-selection`: Covers environment-sensitive defaults, explicit beta opt-in, backend fallback, and localized setup reporting.

### Modified Capabilities

None.

## Impact

The change affects setup state initialization, terminal environment policy, graphics choice localization, UI expectations, and graphics-related developer/manual-test documentation. It changes the internal `GraphicsChoice` variants but introduces no persisted configuration or external dependency changes.
