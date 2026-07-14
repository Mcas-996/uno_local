## 1. Refill implementation

- [x] 1.1 Add runtime and deterministic refill seed generation using OS entropy and seeded RNG state
- [x] 1.2 Replace discard recycling with variant-aware full-deck replenishment in every draw path
- [x] 1.3 Update draw availability so an empty pile remains drawable through automatic replenishment

## 2. Verification

- [x] 2.1 Add tests for Standard and Holiday refill size, discard preservation, repeated refills, and deterministic seeded sequences
- [x] 2.2 Add penalty-boundary and draw-rule regression tests
- [x] 2.3 Run formatting, the full test suite, Clippy, and OpenSpec validation
