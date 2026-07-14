## ADDED Requirements

### Requirement: UI-controlled fitted placement
The system SHALL calculate a fitted terminal-image size from the card art, detected font-cell size, and available preview panel, and the UI SHALL center that size into a final rectangle contained by the panel before protocol encoding.

#### Scenario: Selected and discard previews fit their panels
- **WHEN** image previews are laid out in the selected-card and discard-top panels
- **THEN** each fitted, centered rectangle is wholly contained in its own panel and differs from perfect centering by no more than one terminal cell

### Requirement: Anchored local WezTerm output
The system SHALL wrap local, non-tmux WezTerm iTerm2 image data with one-based absolute cursor positions that anchor upstream clearing and image output at the final rectangle and leave the cursor at the next cell Ratatui accounts for.

#### Scenario: Render a WezTerm card preview
- **WHEN** a card is encoded for a final rectangle in a local WezTerm session outside tmux
- **THEN** its data positions the cursor at the rectangle top-left before the unchanged upstream clear/image sequence and positions it at the next Ratatui cell afterward

#### Scenario: Render independent preview slots
- **WHEN** the selected-card and discard-top slots display cards at different rectangles, including the same logical card
- **THEN** each slot's protocol data contains its own rectangle coordinates and cannot reuse the other slot's positioned data

### Requirement: Position-aware protocol reuse
The system SHALL cache each preview protocol by its card and complete final rectangle.

#### Scenario: Rectangle remains unchanged
- **WHEN** a preview card and its final rectangle are unchanged across redraws
- **THEN** the system reuses the existing encoded protocol

#### Scenario: Origin or dimensions change
- **WHEN** either the origin or dimensions of a preview's final rectangle change
- **THEN** the system regenerates that preview's encoded protocol

### Requirement: Backend compatibility
The system SHALL leave upstream protocol data unchanged for ordinary iTerm2 terminals, Sixel, Kitty, tmux sessions, and text mode.

#### Scenario: Render outside local non-tmux WezTerm
- **WHEN** a supported image backend is active but the session is not local non-tmux WezTerm iTerm2
- **THEN** the application uses the unwrapped `ratatui-image` protocol behavior

### Requirement: Fail-closed WezTerm wrapping
The system SHALL switch to `Text(Encoding)`, discard both preview protocol caches, and emit no unanchored image when required WezTerm iTerm2 data cannot be verified and wrapped safely.

#### Scenario: Unexpected iTerm2 protocol structure
- **WHEN** a local non-tmux WezTerm encoding lacks the expected clear or image framing
- **THEN** the current preview returns no image, both preview caches are empty, and later frames remain in text mode without retrying

### Requirement: Inactive dependency fallback
The repository SHALL retain an unmodified source snapshot of `ratatui-image` v11.0.6 with its provenance and emergency activation procedure documented, while the default build SHALL continue to use the crates.io dependency.

#### Scenario: Build under normal conditions
- **WHEN** the project dependency graph is resolved without an emergency override
- **THEN** `ratatui-image` resolves from crates.io and `external/ratatui-image/` does not participate in the build
