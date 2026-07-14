## Why

Long-running games can exhaust every drawable card, causing Draw Eight and Wild Draw Sixteen penalties to add no cards. The game needs an inexhaustible local supply while preserving independently shuffled decks.

## What Changes

- Refill an empty draw pile with a complete deck matching the selected variant: 108 Standard cards or 118 Holiday cards.
- Shuffle every refill with a fresh pseudorandom generator seeded from new operating-system entropy.
- Keep the discard pile intact instead of recycling it when the draw pile becomes empty.
- Preserve deterministic refill behavior in seeded tests.

## Capabilities

### New Capabilities

- `deck-refill`: Automatic variant-aware deck replenishment, refill shuffling, and continued penalty draws.

### Modified Capabilities

None.

## Impact

The authoritative rules engine and its tests in `src/core.rs` change. No public API, UI control, or dependency is added; the existing `rand` dependency supplies OS entropy and seeded pseudorandom generators.
