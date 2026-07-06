## ADDED Requirements

### Requirement: Connectivity uses STUN-assisted UDP by default
The system SHALL use STUN-assisted UDP endpoint discovery as the default public-internet connectivity path.

#### Scenario: Public endpoint is discovered
- **WHEN** a peer starts a host or join session with STUN enabled
- **THEN** the system queries configured STUN servers and records the peer's discovered public UDP endpoint

#### Scenario: STUN server is unavailable
- **WHEN** all configured STUN servers fail to return a usable endpoint
- **THEN** the system reports STUN discovery failure and does not claim that automatic public connectivity is available

### Requirement: Peers establish direct UDP sessions
The system SHALL establish direct UDP sessions between peers using room share information and handshake messages.

#### Scenario: Joiner reaches host
- **WHEN** a joiner sends a valid handshake to the host endpoint from the room share string
- **THEN** the host validates the room id and responds with join acceptance or rejection

#### Scenario: Handshake times out
- **WHEN** a peer does not receive a required handshake response before the timeout
- **THEN** the system reports connection failure and exposes port-forwarding fallback guidance

### Requirement: Protocol handles unreliable UDP delivery
The system SHALL provide reliability controls for critical room and gameplay messages over UDP.

#### Scenario: Critical message is acknowledged
- **WHEN** a peer sends a critical command, state update, or accepted event
- **THEN** the receiver acknowledges the message using the session id and sequence number

#### Scenario: Critical message is retried
- **WHEN** a sender does not receive acknowledgement for a critical message before the retry timeout
- **THEN** the sender retransmits the message until it is acknowledged or the session times out

### Requirement: Port-forwarding fallback is supported
The system SHALL support a manual fallback where the host forwards a UDP port and shares the externally reachable endpoint.

#### Scenario: Host uses forwarded endpoint
- **WHEN** a host supplies or confirms a manually forwarded UDP endpoint
- **THEN** the system includes that endpoint in the room share information instead of claiming automatic STUN traversal

#### Scenario: Joiner connects to forwarded endpoint
- **WHEN** a joiner uses room share information containing a forwarded endpoint
- **THEN** the system attempts the same room handshake against that endpoint

### Requirement: First version does not encrypt peer traffic
The system SHALL send first-version room and gameplay protocol messages without transport encryption.

#### Scenario: Peer traffic is plaintext
- **WHEN** peers exchange room, command, or game event messages
- **THEN** the system does not require TLS, DTLS, QUIC encryption, or application-level encryption for the message exchange
