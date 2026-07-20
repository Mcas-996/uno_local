## ADDED Requirements

### Requirement: Player hands use bounded concrete storage
The game SHALL retain the exact total card count for each player while storing no more than 200 concrete cards in that player's active batch.

#### Scenario: Factorial creates a capped hand
- **WHEN** Factorial changes a player's hand to one million cards
- **THEN** the public hand count SHALL be one million and the stored concrete batch SHALL contain at most 200 cards

#### Scenario: A small hand remains concrete
- **WHEN** a player has at most 200 cards
- **THEN** all cards SHALL remain concrete and existing play behavior SHALL be preserved

### Requirement: Human players navigate random virtual batches
The frontend SHALL replace the active batch after five boundary presses, using Down at the bottom for the next page and Up at the top for the previous page.

#### Scenario: Load the next batch
- **WHEN** a human remains on the bottom row and presses Down five times
- **THEN** the next virtual page SHALL load at most 200 newly generated cards without changing the total hand count

#### Scenario: Return to a previous batch
- **WHEN** a human remains on the top row of a later page and presses Up five times
- **THEN** the previous page SHALL be regenerated without promising its former identities

#### Scenario: Preserve and reset boundary progress
- **WHEN** a human moves horizontally while remaining on the boundary
- **THEN** the applicable press count SHALL be retained, but leaving the boundary, changing filters or turns, or loading a page SHALL reset it

### Requirement: Virtual generation respects player draw policies
Generated batches SHALL simulate consecutive draws under the player's guarantee and exclusion rule using a dedicated materialization cursor and SHALL NOT consume the shared draw pile or alter the total hand count.

#### Scenario: Generate an AI batch
- **WHEN** an AI batch is materialized
- **THEN** excluded ranks and guaranteed ranks SHALL match that AI's configured difficulty rule

#### Scenario: Generate a human batch
- **WHEN** a human batch is materialized under a configured guarantee rule
- **THEN** guaranteed positions SHALL be honored across consecutive page generations

### Requirement: Gameplay distinguishes active cards from total count
Legal actions, filtering, commands, and AI decisions SHALL inspect only active concrete cards, while public state, victory, and size-dependent effects SHALL use the total hand count.

#### Scenario: AI exhausts an active batch
- **WHEN** an AI plays the last active card while virtual cards remain
- **THEN** the game SHALL materialize another batch automatically and SHALL NOT declare that AI the winner

#### Scenario: Resolve large-hand effects
- **WHEN** Square Root, Factorial, discard wilds, hand swapping, or hand rotation affects a virtual hand
- **THEN** its total count SHALL follow the existing effect rule and concrete storage SHALL remain bounded

### Requirement: Rendering communicates virtual hand state
The hand panel SHALL show the total card count, current page, total pages, and active batch count, and bilingual help/status text SHALL describe random regeneration and five-press navigation.

#### Scenario: Render a partial final page
- **WHEN** the selected virtual page represents fewer than 200 cards
- **THEN** only that remaining page capacity SHALL be materialized and the panel SHALL display the unchanged total count
