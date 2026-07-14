## ADDED Requirements

### Requirement: Users can select automatic or text card rendering
The setup screen SHALL offer Auto and Text graphics choices, SHALL default to Auto on each application launch, and SHALL display the effective backend or text fallback reason.

#### Scenario: Automatic backend is active
- **WHEN** a local terminal resolves a supported image backend in Auto mode
- **THEN** the setup screen shows Auto together with the resolved Kitty, iTerm2, or Sixel backend

#### Scenario: User disables image rendering
- **WHEN** the user selects Text
- **THEN** the application renders cards as colored number/action text and reports that Text was selected manually

### Requirement: SSH sessions always use text rendering
The application MUST resolve to Text without querying graphics capabilities when `SSH_CONNECTION`, `SSH_CLIENT`, or `SSH_TTY` identifies an SSH session, and MUST NOT treat `SSH_AUTH_SOCK` alone as an SSH session.

#### Scenario: Game runs through SSH
- **WHEN** any SSH session variable is non-empty at startup
- **THEN** the application skips terminal graphics queries, reports `Text: SSH`, and emits no image protocol payloads

#### Scenario: Only an SSH agent is available
- **WHEN** `SSH_AUTH_SOCK` is set but no SSH session variable is set
- **THEN** the application continues with normal local terminal detection

### Requirement: Windows terminals use their stable image protocol
In Auto mode, the application SHALL use iTerm2 inline images for local WezTerm sessions and Sixel images for local Windows Terminal sessions when the required backend is available, and SHALL otherwise use Text.

#### Scenario: Local WezTerm session
- **WHEN** WezTerm is identified by `WEZTERM_EXECUTABLE` or `TERM_PROGRAM` and iTerm2 rendering is available
- **THEN** the application selects iTerm2 even if `WT_SESSION` was inherited

#### Scenario: Local Windows Terminal session
- **WHEN** `WT_SESSION` identifies Windows Terminal and capability detection returns Sixel
- **THEN** the application selects Sixel in native Windows shells and local WSL sessions

#### Scenario: Expected Windows backend is unavailable
- **WHEN** WezTerm cannot use iTerm2 or Windows Terminal cannot use Sixel
- **THEN** the application selects Text instead of forcing another image protocol

### Requirement: Other local terminals use detected native images or text
In Auto mode outside the explicit Windows terminal mappings, the application SHALL accept a successfully detected Kitty, Sixel, or iTerm2 backend and SHALL map query failure, halfblocks, or an unexpected backend to Text.

#### Scenario: Kitty graphics are available locally
- **WHEN** a non-SSH local terminal successfully reports Kitty graphics support
- **THEN** the application selects the Kitty backend

#### Scenario: No accepted image protocol is available
- **WHEN** capability detection fails or resolves only to halfblocks
- **THEN** the application uses the colored text card renderer and remains fully playable

### Requirement: Card artwork is generated for every playable card
The application SHALL generate language-neutral RGBA artwork for every Standard and Holiday card rank, including colored numbers and actions, four-color Wild cards, and explicit Draw Two, Draw Four, Draw Eight, and Draw Sixteen markings.

#### Scenario: Holiday card is previewed
- **WHEN** Draw Eight or Wild Draw Sixteen becomes selected or appears on the discard pile
- **THEN** the generated preview visibly distinguishes its color behavior and penalty value while a localized text name remains present

### Requirement: Card images are responsive enhancements
The application SHALL retain the 70x22 minimum terminal size, SHALL render only colored number/action text below 70x26, and SHALL render generated images for the selected human card and discard-top card at 70x26 or larger when an image backend is active.

#### Scenario: Terminal has minimum supported dimensions
- **WHEN** the game area is between 70x22 and 70x25
- **THEN** no card image is rendered and the selected hand card and discard-top card remain identifiable by color and text

#### Scenario: Terminal has image-preview dimensions
- **WHEN** the game area is at least 70x26 and an image backend is active
- **THEN** side-by-side image panels show the selected human card and discard-top card with localized labels and the active color

### Requirement: Image placements follow UI lifecycle changes
The application SHALL reuse unchanged encoded previews and SHALL invalidate or clear image placements when the card, target size, backend, match, overlay visibility, or application lifecycle changes.

#### Scenario: Unchanged frame is redrawn
- **WHEN** the 50 ms render loop redraws without a preview card, size, or backend change
- **THEN** the application reuses the existing encoded previews rather than regenerating them

#### Scenario: Overlay covers the game
- **WHEN** help, quit confirmation, result, or wild-color selection is displayed
- **THEN** card images are suspended and cleared so the overlay is not obscured

#### Scenario: Terminal or application exits image mode
- **WHEN** the terminal shrinks below 70x26, a new match begins, the backend changes, or the application exits
- **THEN** stale card placements are removed and no previous card image remains visible
