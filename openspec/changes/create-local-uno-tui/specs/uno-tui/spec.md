## ADDED Requirements

### Requirement: The application launches into a configurable terminal UI
The `uno` executable SHALL open a cross-platform TUI by default and SHALL allow the player to configure name, AI count, and difficulty before starting.

#### Scenario: Application starts interactively
- **WHEN** the player runs `uno` without arguments
- **THEN** the application opens the local-game setup screen

### Requirement: The table exposes complete local-game information safely
The TUI SHALL show public opponent state, turn and direction, discard and active color, a bounded event log, and the human player's hand without exposing AI hands.

#### Scenario: Match is rendered
- **WHEN** a local match is active
- **THEN** the player can distinguish card colors by both styling and text and can see whose turn it is

### Requirement: Gameplay supports shortcuts and a command bar
The TUI SHALL support keyboard card selection and action shortcuts as well as stable English commands for equivalent actions.

#### Scenario: Player uses keyboard navigation
- **WHEN** the player selects a card with arrow keys and presses Enter
- **THEN** the application submits that card or opens a color chooser for a wild card

#### Scenario: Player uses the command bar
- **WHEN** the player enters `play <index>`, `draw`, `pass`, `help`, `new`, or `quit`
- **THEN** the application performs the matching action or displays a localized validation message

### Requirement: The TUI is localized and restores the terminal
The application SHALL use Simplified Chinese for `zh*` system locales, English otherwise, and SHALL restore terminal state on normal exit and panic.

#### Scenario: Locale is Chinese
- **WHEN** the detected system locale begins with `zh`
- **THEN** setup, gameplay, help, result, and error text is displayed in Simplified Chinese

#### Scenario: Terminal is too small
- **WHEN** the terminal is below the supported table size
- **THEN** the application displays a localized resize prompt without corrupting the terminal
