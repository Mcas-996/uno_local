## ADDED Requirements

### Requirement: Empty draw piles receive a complete new deck
The system SHALL immediately replace an empty draw pile with a complete deck matching the active variant and SHALL leave the discard pile unchanged.

#### Scenario: Refill a Standard game
- **WHEN** a card must be drawn while the Standard draw pile is empty
- **THEN** the system shuffles a new 108-card Standard deck and completes the draw from it

#### Scenario: Refill a Holiday game
- **WHEN** a card must be drawn while the Holiday draw pile is empty
- **THEN** the system shuffles a new 118-card Holiday deck and completes the draw from it

#### Scenario: Preserve discarded cards
- **WHEN** an empty draw pile is refilled
- **THEN** the discard pile and its visible top remain unchanged

### Requirement: Runtime refills use independent shuffle seeds
Every runtime refill MUST use a new pseudorandom generator seeded only from fresh operating-system random entropy.

#### Scenario: Shuffle consecutive runtime refills
- **WHEN** the draw pile is exhausted more than once in a game
- **THEN** each new deck is shuffled with a separately seeded pseudorandom generator

#### Scenario: Reproduce seeded tests
- **WHEN** two seeded games perform the same actions through a refill boundary
- **THEN** their refilled draw sequences are identical

### Requirement: Penalties continue across refill boundaries
The system SHALL allow penalty draws to exhaust one deck, refill it, and continue until the applicable penalty count is complete unless a player draw rule rejects every remaining card.

#### Scenario: Wild Draw Sixteen crosses a refill boundary
- **WHEN** Wild Draw Sixteen resolves with fewer than sixteen cards in the draw pile
- **THEN** the target receives the remaining cards plus cards from a newly shuffled deck for a total penalty of sixteen
