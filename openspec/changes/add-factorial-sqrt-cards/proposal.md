## Why

Holiday play is built around high-impact cards, but it has no mathematical hand-size effects. Factorial and square-root wilds add a new risk/reward pair while retaining the existing color-choice, turn-skip, AI, and terminal presentation model.

## What Changes

- Add two unrestricted Wild Factorial cards and two unrestricted Wild Square Root cards to Holiday, increasing it from 126 to 130 cards while leaving Standard unchanged.
- Make Factorial grow the next player's hand to `min(x!, x^7, 1,000,000)` and skip that player.
- Make Square Root randomly reduce the acting player's remaining hand to `floor(sqrt(x))` and still skip the next player.
- Add authoritative events, bilingual copy, AI strategy, card art, documentation, and bounded large-hand handling for the new cards.
- Give Wild Square Root the same per-player minimum deal frequency as Draw Eight and Wild Factorial the same minimum deal frequency as Wild Draw Sixteen.

## Capabilities

### New Capabilities

- `mathematical-wilds`: Holiday composition, legality, hand-size transformations, turn flow, AI behavior, presentation, and large-hand safety for Factorial and Square Root wilds.

### Modified Capabilities

None.

## Impact

The change affects the public `Rank` and `HandEffect` enums, Holiday deck construction and refill behavior, core resolution, AI scoring and deal filters, app hand access, both terminal renderers, localization, generated card art, README documentation, and automated tests. It adds no dependencies, persistence, networking, or CLI behavior.
