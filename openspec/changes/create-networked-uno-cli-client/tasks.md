## 1. Rust Project Setup

- [x] 1.1 Create a Rust workspace with crates or modules for CLI, gameplay core, room coordination, networking, and protocol types
- [ ] 1.2 Add dependencies for CLI parsing, async runtime, UDP sockets, serialization, IDs/randomness, logging/tracing, and test utilities
- [x] 1.3 Add cross-platform build commands and baseline CI-friendly checks for formatting, linting, and tests

## 2. Gameplay Core

- [x] 2.1 Define UNO card, color, rank, action, player, hand, deck, discard pile, direction, and turn-state types
- [x] 2.2 Implement deterministic game initialization for 2-5 players using collection-based player storage compatible with a future 10-player limit
- [ ] 2.3 Implement validation for play, draw, pass, color choice, turn order, card ownership, playable cards, card effects, and win detection
- [x] 2.4 Implement authoritative game events with sequence numbers, deterministic event application, and duplicate-event handling
- [x] 2.5 Add gameplay tests for valid starts, invalid player counts, valid card play, invalid card play, event idempotency, and hidden hand visibility

## 3. Protocol and Serialization

- [x] 3.1 Define protocol versioning, peer ids, room ids, session ids, host roles, endpoints, and share-string data structures
- [x] 3.2 Define messages for STUN status, room handshake, join acceptance/rejection, membership updates, readiness, commands, accepted events, acknowledgements, retries, and disconnects
- [ ] 3.3 Implement serialization and deserialization for all protocol messages with validation errors for malformed data
- [x] 3.4 Implement share-string encoding and parsing containing protocol version, transport, endpoint, room id, and host identity
- [x] 3.5 Add protocol tests for round-trip serialization, invalid messages, and share-string parsing

## 4. Peer Connectivity

- [x] 4.1 Implement configurable STUN server settings with a default public STUN list and command-line override support
- [x] 4.2 Implement UDP socket binding and STUN public endpoint discovery for host and join sessions
- [x] 4.3 Implement direct UDP room handshake between joiner and host using parsed share information
- [x] 4.4 Implement reliability controls for critical UDP messages using sequence numbers, acknowledgements, retransmission, and session timeout
- [x] 4.5 Implement connection failure reporting that distinguishes STUN discovery failure, handshake timeout, and session timeout
- [x] 4.6 Implement UDP port-forwarding fallback mode where the host can publish a manually forwarded endpoint
- [x] 4.7 Add networking tests with local UDP sockets for handshake success, timeout behavior, retry behavior, duplicate handling, and forwarded-endpoint parsing

## 5. Networked Rooms and Host Failover

- [x] 5.1 Implement room creation, membership tracking, display names, readiness, and lobby-state broadcast
- [ ] 5.2 Implement host-authoritative command validation and accepted-event broadcast to connected peers
- [x] 5.3 Implement active host and standby host role assignment when at least two eligible peers are connected
- [x] 5.4 Replicate membership, host roles, game state, event sequence position, and pending critical-message state to the standby host
- [x] 5.5 Implement deterministic standby promotion when the active host disconnects and replicated state is current
- [x] 5.6 Implement room suspension or termination when no synchronized host-capable peer remains
- [x] 5.7 Add room tests for join, leave, readiness, host role assignment, standby replication, active-host disconnect, and no-host continuation failure

## 6. CLI Experience

- [x] 6.1 Implement host command that creates a room, discovers or accepts the public endpoint, and prints the share string
- [x] 6.2 Implement join command that accepts a share string and reports connection progress, success, or actionable failure
- [ ] 6.3 Implement interactive session commands for play, draw, pass, color, hand, state, ready, start, and leave
- [x] 6.4 Implement CLI diagnostics for STUN failure, handshake timeout, session disconnect, invalid commands, and port-forwarding fallback guidance
- [x] 6.5 Ensure command names, arguments, and interactive commands are consistent on macOS, Windows, and Linux
- [x] 6.6 Add CLI-level tests for command parsing, malformed input, share-string input, and safe terminal exit on disconnect

## 7. End-to-End Verification

- [ ] 7.1 Add local multi-process or multi-task integration tests for a host and at least one joiner completing a minimal game flow
- [x] 7.2 Add an integration test covering a 2-host room where the standby host takes over after active-host disconnect
- [x] 7.3 Add an integration test covering STUN-disabled or STUN-failed mode that surfaces port-forwarding fallback guidance
- [x] 7.4 Document manual test commands for hosting, joining, playing turns, simulating disconnects, and testing a forwarded UDP endpoint
- [x] 7.5 Run formatting, linting, unit tests, and integration tests before marking the implementation complete
