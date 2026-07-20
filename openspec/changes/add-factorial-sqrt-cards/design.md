## Context

The authoritative `core` module stores hands as `Vec<Card>`, resolves single-card effects after removing the played card, and skips all effects when that play wins immediately. Wilds reuse one color-choice action path. Holiday already refills from a complete generated deck and can create large hands, but the app currently clones and lays out an entire visible human hand every frame and legal actions contain duplicate entries for duplicate cards.

## Goals / Non-Goals

**Goals:**

- Add deterministic, overflow-safe mathematical hand transformations without changing Standard play.
- Preserve existing wild color selection, direction-aware turn movement, final-card victory, tailored draw rules, hidden-hand boundaries, and seeded random tests.
- Keep interaction usable when Factorial reaches its one-million-card cap.

**Non-Goals:**

- Add stacking or batch play for mathematical wilds.
- Add a new setup toggle, hand filter, dependency, persistence format, or network protocol.
- Let users choose which cards Square Root discards.

## Decisions

### Model both cards as Holiday wild ranks

Add `WildFactorial` and `WildSquareRoot` to `Rank`, `Card::is_wild`, and the Holiday deck with two copies each. They use the existing required color choice and unrestricted wild legality. Standard remains unchanged, and every refill uses the 130-card Holiday composition.

### Resolve transformations authoritatively after removing the played card

Factorial targets the next player in the current direction, computes `min(x!, x^7, 1_000_000)` using checked/saturating integer arithmetic with early termination, draws the difference through `draw_card_for`, then advances again to skip that player. Square Root reads the actor's remaining hand length, computes exact integer `floor(sqrt(x))`, randomly removes the excess, places removed cards under the played wild so it remains the discard top, and advances twice. Existing immediate victory prevents either effect from resolving when the played card empties the hand.

Expose the outcome through explicit `HandEffect` variants carrying the affected player and before/after counts so presentation never inspects private cards.

### Bound work created by duplicate million-card hands

Reserve bulk draw capacity and retain the existing refill/deal policy. Deduplicate legal card actions by card value, compute AI hand statistics once per choice, and avoid cloning full AI hands. Replace per-frame visible-hand cloning/full layout with cached filtered hand indices and a bounded 257-card window centered on the selection; rendering materializes only rows from that window. Invalidate the cache after hand mutations, filter changes, and match changes while preserving filtered global indices.

### Keep AI and presentation consistent with Holiday wilds

Classify both ranks with the existing high Holiday wild deal filters. Factorial receives the highest disruptive preference; Square Root is scored from how many cards it removes from the actor. Add bilingual names/help/logs and four-quadrant wild art labeled `x!` and `SQRT`.

### Pair mathematical wild guarantees with their matching penalty tiers

Whenever a draw rule guarantees Draw Eight, reserve an adjacent independent draw for Wild Square Root; whenever it guarantees Wild Draw Sixteen, reserve an adjacent independent draw for Wild Factorial. This preserves the existing Draw Eight and Draw Sixteen slots while giving their mathematical partner the same guaranteed count. Easy AI excludes both pairs, Normal AI may receive Draw Eight/Square Root but excludes Draw Sixteen/Factorial, Hard AI receives one Draw Eight and one Square Root per seven cards, and Extreme AI receives two of each lower tier plus one of each upper tier per seven cards. Human schedules place paired guarantees immediately before their existing partner slots to avoid collisions.

## Risks / Trade-offs

- [A one-million-card hand still consumes memory and takes time to construct] → Cap strictly, reserve once, deduplicate downstream actions, and test pure cap arithmetic separately from ordinary-size integration effects.
- [Cached hand layout can become stale] → Centralize invalidation on every event/setup/filter transition and cover it with navigation/render tests.
- [Square Root can be extremely strong] → Keep only two copies in the optional Holiday deck and retain the established final-card no-effect rule.
- [Tailored draw rules affect Factorial contents] → Route every added card through the existing player-specific draw path intentionally.
