## ADDED Requirements

### Requirement: One executable selects a bounded frontend
The application SHALL publish one `uno` executable and SHALL select universal Text for SSH or tmux, the Termwiz frontend for local WezTerm, and the universal frontend for every other local terminal. SSH and tmux detection MUST take precedence over terminal-emulator hints.

#### Scenario: Local WezTerm selects Termwiz
- **WHEN** a local process has a WezTerm environment hint and is not inside SSH or tmux
- **THEN** the application runs the Termwiz frontend

#### Scenario: Remote and multiplexer sessions stay text-only
- **WHEN** an SSH session variable or tmux environment is present
- **THEN** the application runs universal Text without a graphics capability query

#### Scenario: Other terminals select universal
- **WHEN** the process is local, outside tmux, and has no WezTerm environment hint
- **THEN** the application runs the universal frontend

### Requirement: Frontends share application behavior
Both frontends SHALL use the same game, AI, localization, command, input-action, and semantic view state. They SHALL expose the same pages and controls, while their exact layout and decoration MAY differ.

#### Scenario: Equivalent input changes equivalent state
- **WHEN** either frontend translates the same key and modifier into a shared application input
- **THEN** the shared application state transition is identical

#### Scenario: Every overlay suppresses images
- **WHEN** help, quit confirmation, wild-color selection, result, or undersized-terminal content covers the game
- **THEN** the active frontend renders that content without a card image operation

### Requirement: Universal frontend supports safe Text and Sixel
The universal frontend SHALL always provide the complete colored Text interface. It SHALL expose Sixel only after a one-time bounded query confirms Sixel and a nonzero terminal cell pixel size, SHALL default to Sixel when available, and SHALL otherwise default to Text. The setup SHALL allow switching Text and Graphics Beta and SHALL report the effective result.

#### Scenario: Confirmed Sixel defaults to graphics
- **WHEN** the startup query confirms Sixel and terminal cell pixel dimensions before its 250 ms deadline
- **THEN** Graphics Beta is initially selected and card previews use direct Sixel output

#### Scenario: Missing information falls back to Text
- **WHEN** the query times out, is malformed, lacks Sixel, or lacks valid cell pixel dimensions
- **THEN** Text is initially selected and no Sixel bytes are emitted

#### Scenario: Sixel placement changes safely
- **WHEN** a displayed card, target size, terminal cell size, layout, overlay, or terminal size changes
- **THEN** the old image rectangle is cleared and only a currently visible image is encoded or placed at its final absolute rectangle

#### Scenario: Encoding failure is terminal for graphics
- **WHEN** direct Sixel encoding fails
- **THEN** the frontend clears image state, switches to Text for the process lifetime, and does not retry encoding

### Requirement: Local WezTerm uses Termwiz-owned images
The WezTerm frontend SHALL use Termwiz 0.23.3 Surface changes for text and images, SHALL default to Graphics Beta, and SHALL allow Text selection. It SHALL provide PNG bytes as encoded image data to Termwiz and SHALL NOT send application-owned Kitty or iTerm2 control sequences.

#### Scenario: WezTerm renders an encoded card
- **WHEN** Graphics Beta is effective and a card preview is visible
- **THEN** the frontend submits an encoded PNG `Change::Image` for the final cell rectangle and Termwiz owns protocol output

#### Scenario: WezTerm image creation fails
- **WHEN** PNG creation or Termwiz image capability is unavailable
- **THEN** the Termwiz frontend disables images for the process lifetime and remains usable with Text cards

#### Scenario: Termwiz terminal fails
- **WHEN** Termwiz cannot initialize or returns a terminal I/O failure
- **THEN** terminal state is restored and the same application state continues in universal Text

### Requirement: Terminal state is always restored
Both frontends SHALL restore raw mode, alternate screen, cursor visibility, and application-owned visual state after normal exit, recoverable fallback, and panic cleanup. Resize SHALL invalidate graphics placement and force a safe repaint.

#### Scenario: Normal exit leaves no preview
- **WHEN** the player exits normally
- **THEN** no card image remains at the shell prompt and the terminal is in cooked primary-screen mode with a visible cursor

#### Scenario: Panic restores terminal
- **WHEN** rendering or event handling panics
- **THEN** the process-wide panic restoration attempts cooked mode, primary screen, clearing, and cursor display

### Requirement: Packaging and minimum-size behavior remain stable
The application SHALL retain the existing CLI, single cargo-dist application, installer/updater/uninstaller contract, and `70 x 26` minimum complete UI size.

#### Scenario: Existing CLI remains available
- **WHEN** a user invokes help, version, or uninstall options
- **THEN** the existing command behavior and single `uno` executable contract are preserved

#### Scenario: Terminal is undersized
- **WHEN** either frontend has fewer than 70 columns or 26 rows
- **THEN** it displays the resize message without emitting a card image
