## ADDED Requirements

### Requirement: Game supports standard UNO player counts for the first version
The system SHALL support games with 2-5 players in the first version and MUST reject starting a game outside that range.

#### Scenario: Valid player count starts
- **WHEN** a room has between 2 and 5 connected players and the host starts the game
- **THEN** the system creates the initial deck, hands, discard pile, turn order, and current player

#### Scenario: Invalid player count is rejected
- **WHEN** a room has fewer than 2 players or more than 5 players and the host starts the game
- **THEN** the system rejects the start request and keeps the room in the lobby state

### Requirement: Game state enforces UNO turn rules
The system SHALL validate player actions against UNO turn order, card ownership, playable card rules, draw actions, color choices, and win conditions.

#### Scenario: Player plays a valid card
- **WHEN** it is a player's turn and the player submits a card that matches the active color, rank, symbol, or wild rule
- **THEN** the system accepts the play, updates the discard pile, applies card effects, and advances the turn as required

#### Scenario: Player plays an invalid card
- **WHEN** a player submits a card they do not hold or a card that is not playable
- **THEN** the system rejects the action and leaves the authoritative game state unchanged

### Requirement: Game uses authoritative events
The system SHALL represent accepted game changes as ordered events that can be applied deterministically by connected peers.

#### Scenario: Event is applied once
- **WHEN** a peer receives an accepted event with a new sequence number
- **THEN** the peer applies the event exactly once and updates derived game state

#### Scenario: Duplicate event is ignored
- **WHEN** a peer receives an event sequence number that it has already applied
- **THEN** the peer ignores the duplicate without changing game state

### Requirement: Hidden card information is preserved
The system SHALL avoid revealing private player hand contents to other players during normal play.

#### Scenario: Player views own hand
- **WHEN** a player requests their hand from the CLI session
- **THEN** the system displays that player's cards

#### Scenario: Player views table state
- **WHEN** a player requests public table state
- **THEN** the system displays public information without revealing other players' card identities

### Requirement: Game model preserves future 10-player compatibility
The system SHALL avoid hard-coding protocol or state assumptions that prevent expanding the player limit to 10 later.

#### Scenario: Room model stores ordered players generically
- **WHEN** the system stores player order, hands, scores, host roles, or connection metadata
- **THEN** the data model uses collection-based player records instead of fixed 5-player fields
