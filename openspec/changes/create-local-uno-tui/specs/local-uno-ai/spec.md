## ADDED Requirements

### Requirement: AI runs entirely in the local process
The system SHALL choose and execute AI actions without network access, remote models, accounts, or external services.

#### Scenario: AI takes a turn
- **WHEN** the current player is an AI opponent
- **THEN** the application selects an action from local game state and advances the match locally

### Requirement: AI actions are always legal
The AI SHALL select only actions returned by the authoritative game legal-action API.

#### Scenario: AI has no playable card
- **WHEN** an AI has no legal card play before drawing
- **THEN** it draws, plays the drawn card if legal, or passes

### Requirement: AI provides three selectable difficulties
The system SHALL provide easy, normal, and hard local strategies with reproducible decisions under an injected random seed.

#### Scenario: Easy AI selects an action
- **WHEN** easy AI has multiple legal cards
- **THEN** it selects uniformly from those cards and chooses a legal wild color

#### Scenario: Normal AI selects an action
- **WHEN** normal AI has multiple legal cards
- **THEN** it favors color continuity and useful action cards while preserving wild cards when a colored play exists

#### Scenario: Hard AI faces an opponent near victory
- **WHEN** the next opponent has two or fewer cards
- **THEN** hard AI increases the priority of Draw, Skip, and Reverse cards that disrupt that opponent
