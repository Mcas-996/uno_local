## ADDED Requirements

### Requirement: Holiday contains mathematical wilds
The Holiday deck SHALL contain two Wild Factorial cards and two Wild Square Root cards in addition to its existing 126 cards, while the Standard deck SHALL remain unchanged.

#### Scenario: Construct Holiday
- **WHEN** the system constructs or refills a Holiday deck
- **THEN** it contains exactly 130 cards including two cards of each mathematical wild rank

### Requirement: Factorial transforms and skips the next player
Wild Factorial SHALL be playable without a color match, MUST require a valid color choice, SHALL change the next player's hand from `x` cards to `min(x!, x^7, 1,000,000)` cards through normal player-specific draws, and SHALL skip that player.

#### Scenario: Apply an uncapped factorial
- **WHEN** Wild Factorial targets a player holding 5 cards
- **THEN** that player finishes with 120 cards and loses their turn

#### Scenario: Cap a factorial
- **WHEN** Wild Factorial targets a player whose calculated result exceeds one million
- **THEN** that player finishes with exactly 1,000,000 cards

#### Scenario: Respect direction
- **WHEN** Wild Factorial is played counter-clockwise
- **THEN** the adjacent player in that direction is transformed and skipped

### Requirement: Square Root transforms the acting player and skips the next player
Wild Square Root SHALL be playable without a color match, MUST require a valid color choice, SHALL randomly discard the acting player's post-play hand from `x` cards to `floor(sqrt(x))` cards, SHALL keep itself on top of the discard pile, and SHALL skip the next player.

#### Scenario: Reduce a remaining hand
- **WHEN** a player has 10 cards after removing the played Wild Square Root
- **THEN** 7 random cards are discarded, 3 remain, and the next player loses their turn

#### Scenario: Seeded removal is reproducible
- **WHEN** identical seeded games resolve Wild Square Root from identical hands
- **THEN** they retain and discard identical cards

### Requirement: Final mathematical wild wins without its effect
The system SHALL preserve immediate victory when a mathematical wild is the acting player's final card.

#### Scenario: Play a final mathematical wild
- **WHEN** a player empties their hand by playing Factorial or Square Root
- **THEN** that player wins immediately without transforming any hand

### Requirement: Mathematical wilds share the matching guarantee frequency
Player-specific Holiday draw rules SHALL guarantee Wild Square Root as often as they guarantee Draw Eight and SHALL guarantee Wild Factorial as often as they guarantee Wild Draw Sixteen, using separate draw positions without replacing the existing guaranteed cards.

#### Scenario: Lower-tier guarantee
- **WHEN** a player draw rule guarantees one or more Draw Eight cards in a draw block
- **THEN** the same block contains the same number of guaranteed Wild Square Root cards

#### Scenario: Upper-tier guarantee
- **WHEN** a player draw rule guarantees one or more Wild Draw Sixteen cards in a draw block
- **THEN** the same block contains the same number of guaranteed Wild Factorial cards

#### Scenario: Exclusion tiers stay paired
- **WHEN** a difficulty excludes Draw Eight or Wild Draw Sixteen from a player's draws
- **THEN** it also excludes the corresponding Wild Square Root or Wild Factorial tier

### Requirement: Local play presents and handles mathematical wilds
The AI, bilingual text frontend, graphics frontend, help, logs, and documentation SHALL recognize both mathematical wilds, and hand interaction SHALL remain bounded and usable for hands up to the Factorial cap without exposing private cards.

#### Scenario: AI selects a mathematical wild
- **WHEN** a local AI has a legal mathematical wild action
- **THEN** it chooses a legal color and evaluates the action using its resulting hand-size effect

#### Scenario: Render a capped hand
- **WHEN** a human-controlled hand has one million cards
- **THEN** the frontend renders only the visible selection window while retaining filtered global card indices and navigation
