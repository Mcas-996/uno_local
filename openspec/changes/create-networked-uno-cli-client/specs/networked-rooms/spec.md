## ADDED Requirements

### Requirement: Host creates shareable rooms
The system SHALL allow a host to create a room and produce shareable connection information that joiners can use without a central room directory.

#### Scenario: Room share string is produced
- **WHEN** a host creates a room
- **THEN** the system displays a share string containing protocol version, transport, host endpoint, room id, and host identity

#### Scenario: Joiner uses share string
- **WHEN** a joiner provides a valid room share string
- **THEN** the system parses the connection information and attempts to join the referenced room

### Requirement: Rooms manage player membership
The system SHALL maintain room membership, player display names, readiness state, and connection state before and during a game.

#### Scenario: Player joins lobby
- **WHEN** a join request is accepted before the game starts
- **THEN** the system adds the player to the room membership and broadcasts the updated lobby state

#### Scenario: Player leaves lobby
- **WHEN** a connected player leaves before the game starts
- **THEN** the system removes the player from membership and broadcasts the updated lobby state

### Requirement: Rooms use one active host and one standby host
The system SHALL maintain an active host and a standby host when at least two eligible peers are connected.

#### Scenario: Standby host is assigned
- **WHEN** a room has at least two connected peers and no standby host
- **THEN** the system assigns a standby host from eligible connected peers and broadcasts the host roles

#### Scenario: Active host disconnects
- **WHEN** the active host disconnects and a standby host is connected with current replicated state
- **THEN** the standby host becomes active host and resumes command validation and event broadcast

### Requirement: Rooms end when no host can continue
The system SHALL end or suspend a room when no eligible host-capable peer can continue authoritative operation.

#### Scenario: Both host-capable peers are unavailable
- **WHEN** the active host disconnects and no synchronized standby host is available
- **THEN** the system reports that the room cannot continue

### Requirement: Rooms replicate authoritative state to standby host
The system SHALL replicate enough room and game state to the standby host for deterministic failover.

#### Scenario: Standby receives state updates
- **WHEN** the active host accepts membership changes or game events
- **THEN** the standby host receives the updated membership, role state, game state, and event sequence position
