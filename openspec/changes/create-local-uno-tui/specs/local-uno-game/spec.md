## ADDED Requirements

### Requirement: Local matches support standard first-version player counts
The system SHALL start an offline match with one human and between one and four AI opponents.

#### Scenario: Player configures a match
- **WHEN** the player selects between one and four AI opponents and starts the match
- **THEN** the system creates a shuffled game for two to five total players without contacting an external service

### Requirement: Turns enforce draw and pass phases
The system SHALL expose legal actions and MUST reject actions outside the current turn phase.

#### Scenario: Player draws a card
- **WHEN** the current player draws once
- **THEN** the system allows only the drawn card to be played when legal or allows the player to pass

#### Scenario: Player attempts an illegal pass
- **WHEN** the current player attempts to pass before drawing
- **THEN** the system rejects the action without changing game state

### Requirement: Basic UNO action cards are applied
The system SHALL apply Skip, Reverse, Draw Two, Wild, and Wild Draw Four effects and SHALL require a color choice for wild cards.

#### Scenario: Reverse is played in a two-player game
- **WHEN** a player plays Reverse in a two-player game
- **THEN** the opponent is skipped and the same player receives the next turn

#### Scenario: Wild Draw Four is restricted
- **WHEN** a player holds a card matching the active color
- **THEN** the system rejects that player's attempt to play Wild Draw Four

### Requirement: A local round can finish without exhausting its deck
The system SHALL recycle and shuffle the discard pile except its top card when more cards must be drawn, and SHALL end the round when a player empties their hand.

#### Scenario: Draw pile is exhausted
- **WHEN** a draw is required and the draw pile is empty while recyclable discards exist
- **THEN** the system shuffles recyclable discards into a new draw pile and completes the draw

#### Scenario: Player plays their final card
- **WHEN** a player legally plays their final card
- **THEN** the system records that player as winner and rejects further game actions
