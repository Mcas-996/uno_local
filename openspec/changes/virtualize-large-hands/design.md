## Context

`Player` owns a `Vec<Card>`, while legality, effects, AI, filtering, and rendering assume the vector length is the complete hand size. Factorial can expand that vector to one million entries. The existing frontend window limits rendered cards but still caches one index per matching card.

## Goals / Non-Goals

**Goals:**
- Bound every player's stored card identities to 200 while preserving an exact total hand count.
- Keep loaded cards stable until the player changes pages.
- Support deterministic tests for random regeneration and player-specific draw policies.
- Preserve the meaning of all existing card effects at the count level.

**Non-Goals:**
- Preserving identities of abstract cards or reproducing the same page after navigation.
- Persisting games across processes.
- Allowing AI to search virtual pages for a stronger action.

## Decisions

- Introduce a core `Hand` value containing `cards`, `total_len`, and `page`. Core logic, rather than the renderer, owns materialization because AI and card effects also need bounded identities.
- Treat page positions as virtual ranges over `total_len`. Changing pages discards the active identities and generates the target page independently; returning to a page intentionally produces new identities.
- Add a separate materialization cursor per player. Page generation simulates consecutive player-specific draws and advances guarantee/exclusion scheduling without consuming the shared draw pile or changing the real received-card counter.
- Preserve an immediately drawn card in the active batch by abstracting another active card when full. This retains the existing draw-then-play rule without exceeding 200 identities.
- Expose total length and active cards through separate APIs. Public state, victory, and size effects use total length; legal actions, filtering, and AI use active cards.
- Store navigation boundary counters in `App`. A counter persists across horizontal movement while the selection remains on the boundary and resets after leaving it, switching filters or turns, or loading a page.
- Swap and rotate whole hands. Count-only effects update totals and rematerialize or clamp the active page; redistributed cards increment recipient totals and are only kept concretely when capacity permits.

## Risks / Trade-offs

- [A regenerated hand can contain cards different from those abstracted] → Make this explicit in help and page status; keep identities stable within the active page.
- [Wild Draw Four legality and AI decisions cannot inspect abstract cards] → Define legality over the active batch consistently for both humans and AI.
- [Guarantee schedules could be exploited by repeated paging] → Advance a dedicated materialization cursor rather than resetting each page.
- [Large effects have identity-sensitive discard piles] → Retain only the top discard as meaningful and use counts for cards buried by bulk discard effects.

