## Why

Factorial hands can reach one million cards, but the core currently materializes every card and the frontend retains a global index for every filtered card. Large hands therefore consume excessive memory and make otherwise bounded rendering depend on an unbounded game-state representation.

## What Changes

- Store at most 200 concrete cards per player and represent the remainder as an abstract count.
- Let human players replace the active random batch after five boundary navigation presses and regenerate prior batches instead of preserving their identities.
- Generate virtual batches with player-specific guarantee and exclusion rules without consuming the public draw pile.
- Make AI players use their active batch and automatically refresh only after that batch is exhausted.
- Preserve total-count semantics for victory, public state, Factorial, Square Root, discard wilds, and 7-0 hand movement.

## Capabilities

### New Capabilities
- `virtual-large-hands`: Bounded hand storage, random batch materialization, boundary navigation, AI access, and large-hand effect semantics.

### Modified Capabilities

## Impact

The core player-hand representation and hand-related APIs change, with corresponding updates to AI input, app navigation, rendering, bilingual help/status text, tests, and large-hand documentation. No external dependency or save-format migration is required.
