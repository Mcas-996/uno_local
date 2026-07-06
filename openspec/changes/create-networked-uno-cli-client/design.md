## Context

The repository is starting from an OpenSpec-only state, so this change establishes the initial application architecture. The first deliverable is a Rust CLI UNO client that proves public-internet multiplayer, room coordination, and game-state synchronization before a desktop UI is added.

The user-facing target is eventually a desktop application feel across macOS, Windows, and Linux, but the first implementation surface is CLI-oriented for easier debugging. The networking model must avoid a dedicated game server, use STUN for public endpoint discovery, accept unencrypted trusted-friend play, and provide a port-forwarding fallback when NAT traversal fails.

## Goals / Non-Goals

**Goals:**

- Build a cross-platform Rust CLI that can host, join, and play networked UNO games.
- Support public-internet connectivity with STUN-assisted UDP as the primary path.
- Allow host-created rooms with shareable connection information.
- Support 2-5 players in the first version and keep protocol limits compatible with 10 players.
- Use two host-capable peers so the room can continue if one host disconnects.
- Keep UNO rules and state transitions independent from CLI and networking code.
- Define reusable protocol and state boundaries for a later desktop UI.

**Non-Goals:**

- No graphical desktop UI in the first implementation.
- No encryption, authentication, account system, matchmaking, or central room directory.
- No anti-cheat guarantees against malicious hosts or peers.
- No TURN or hosted relay service in the first implementation.
- No guarantee that every NAT environment can connect without manual port forwarding.

## Decisions

### Use a Rust workspace with separated CLI, core gameplay, room, and networking modules

The project will use Rust so the same implementation can target macOS, Windows, and Linux. Gameplay logic will be isolated from the CLI and network transport so future UI work can reuse the same core.

Alternative considered: build a UI-first Tauri application. This was deferred because the first risk is public connectivity, not visual presentation.

### Use UDP plus STUN for the primary connectivity path

Peers will bind UDP sockets and use STUN to discover their public endpoint. Hosts will print a shareable connection string containing protocol version, public endpoint, room id, and host identity. Joiners will use that data to send join/punch messages to the host endpoint.

Alternative considered: TCP-only connection with manual port forwarding. TCP is simpler to debug but does not pair well with STUN-assisted hole punching, so it is better as a fallback mode or troubleshooting path than the primary public-connectivity strategy.

### Keep port forwarding as the first fallback

When STUN-assisted connectivity fails, the CLI will explain that the host must forward the selected UDP port and share the externally reachable address. This keeps the first version serverless while giving users a practical recovery path.

Alternative considered: TURN/relay fallback. That would improve success rates but introduces a server dependency and bandwidth cost, which conflicts with the first-version serverless constraint.

### Use host authority with a standby host

The active host validates player commands, applies UNO rules, and broadcasts accepted game events. A second host-capable peer receives enough replicated room and game state to take over if the active host disconnects.

Alternative considered: full peer-to-peer consensus. Consensus would reduce host trust assumptions but is overbuilt for trusted friend games and would slow the first implementation.

### Use event-based game synchronization

Peers will exchange commands and accepted events rather than synchronizing arbitrary UI state. The core gameplay module will derive visible game state from the authoritative event stream and hidden per-player hand state.

Alternative considered: synchronize full game snapshots after every action. Snapshots are useful for recovery and late join flows, but event synchronization is easier to validate and test for turn-based gameplay.

### Use a CLI session with subcommands and an interactive game prompt

The CLI will expose commands for hosting and joining, then enter an interactive game prompt for player actions such as playing cards, drawing, passing, choosing colors, and leaving.

Alternative considered: one command per action process invocation. That is awkward for a live network session because it would need persistent background state.

## Risks / Trade-offs

- STUN cannot traverse every NAT type -> The CLI will detect connection timeout and guide the user to UDP port forwarding.
- Active host can cheat or misbehave -> First version is scoped to trusted friend games and does not claim adversarial fairness.
- Standby host failover can diverge if replication is incomplete -> Host state replication will include membership, turn state, deck state, discard pile, hands, draw stack, direction, and event sequence numbers.
- UDP is unreliable and unordered -> The protocol will include session ids, sequence numbers, acknowledgements, idempotent event application, and retry timeouts for critical messages.
- Two-host coordination adds complexity -> Only one active host will accept commands at a time, and standby promotion will be deterministic based on replicated host role order.
- Future desktop UI could couple to CLI behavior -> CLI commands will use the same core and protocol APIs that a later desktop UI must call.

## Migration Plan

This is an initial implementation, so there is no existing application data or user workflow to migrate. The implementation should begin with the core gameplay and protocol contracts, then add CLI host/join flows, STUN connectivity, and room failover.

Rollback is removing the new Rust project files and OpenSpec change before archive; no persistent external systems are modified.

## Open Questions

- Which public STUN server list should be used as the default, and how many fallback servers should be configured?
- Should the first implementation include a TCP manual-port-forwarding mode, or keep fallback UDP-only?
- How much room state should be shown in the CLI for debugging versus hidden to preserve normal UNO gameplay?
