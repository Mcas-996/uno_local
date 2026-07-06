## ADDED Requirements

### Requirement: CLI exposes host and join flows
The system SHALL provide cross-platform CLI commands that allow a user to host a new UNO room or join an existing room using shared connection information.

#### Scenario: Host starts a room
- **WHEN** a user runs the host command
- **THEN** the CLI creates a local room session and displays shareable connection information for joiners

#### Scenario: Player joins a room
- **WHEN** a user runs the join command with valid shareable connection information
- **THEN** the CLI attempts to connect to the host room and reports whether the join succeeds or fails

### Requirement: CLI enters an interactive game session
The system SHALL provide an interactive CLI session after a user hosts or joins a room.

#### Scenario: Session accepts game commands
- **WHEN** a connected player enters a supported command such as play, draw, pass, color, hand, state, or leave
- **THEN** the CLI submits the action or displays the requested local game information

#### Scenario: Session rejects invalid commands
- **WHEN** a connected player enters an unsupported or malformed command
- **THEN** the CLI displays an error without disconnecting the player from the room

### Requirement: CLI displays connection diagnostics
The system SHALL display actionable diagnostics for connection setup, NAT traversal, and fallback guidance.

#### Scenario: STUN path fails
- **WHEN** the CLI cannot establish a STUN-assisted UDP session before the connection timeout
- **THEN** the CLI explains that the host may need to configure UDP port forwarding and share the forwarded endpoint

#### Scenario: Network session disconnects
- **WHEN** the CLI detects that the room session is no longer reachable
- **THEN** the CLI displays the disconnection reason and returns the user to a safe terminal state

### Requirement: CLI remains portable across desktop operating systems
The system SHALL run the same gameplay and networking commands on macOS, Windows, and Linux.

#### Scenario: Platform-independent commands
- **WHEN** the CLI is built for macOS, Windows, or Linux
- **THEN** the host and join command names, required arguments, and game-session commands remain consistent across platforms
