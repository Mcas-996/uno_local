## Context

`Game` currently owns one `StdRng`, shuffles the initial deck with it, and recycles all but the visible discard when the draw pile empties. Penalty draws stop silently when no card is available. Long games can nevertheless contain more cards than the selected deck because player draw guarantees synthesize missing Holiday cards, so recycling cannot guarantee that an AI penalty has cards to draw.

## Goals / Non-Goals

**Goals:**

- Make both deck variants refill automatically whenever the draw pile is empty.
- Give every runtime refill an independently seeded shuffle using fresh OS entropy.
- Keep seeded tests reproducible across refill boundaries.
- Allow action-card penalties to continue through a refill.

**Non-Goals:**

- Adding refill notifications or setup options.
- Limiting the number of refills or deduplicating cards across decks.
- Changing player-specific draw guarantees and exclusions.

## Decisions

- Centralize the empty-pile check in the authoritative draw path. Before any draw, an empty pile is replaced by `deck(self.deck_variant)`, shuffled, and then consumed. This covers normal and penalty draws without duplicating effect logic.
- Do not recycle discards. The selected behavior is to add a new deck immediately when the draw pile empties, leaving the entire discard history and visible top unchanged.
- Model refill seeding as an internal production-or-deterministic strategy. Production fills a 32-byte seed from `OsRng` before constructing a refill-only `StdRng`. Seeded test constructors derive refill seeds deterministically from their existing RNG state.
- Update draw availability so an empty pile remains drawable because a refill can always be created. Existing per-player exclusions still apply while a non-empty pile contains no allowed card.

## Risks / Trade-offs

- [The discard pile grows for the duration of very long games] -> Accept this as the explicit immediate-refill behavior; each card is small and games are local.
- [Wall-clock time is not random] -> Combine it with 256 bits from the operating system rather than relying on time alone.
- [Runtime shuffles cannot be asserted exactly] -> Keep injected deterministic seed behavior for sequence tests.
- [A player rule can reject every remaining card before the pile is literally empty] -> Preserve existing exclusion semantics; automatic refill is triggered only by an empty draw pile as requested.
