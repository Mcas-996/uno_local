## Why

Players need a cross-platform UNO client that can play over the public internet without requiring a dedicated game server. Starting with a Rust CLI client keeps the first version debuggable while proving the networking, room, and game-state foundations before adding a desktop UI.

## What Changes

- Create a Rust-based CLI application as the first client surface for hosting and joining UNO games.
- Support public-internet play where a host creates a room and other players join using shared connection information.
- Use STUN-assisted UDP connectivity for the primary public networking path.
- Provide a fallback path that tells users to use host-side port forwarding when STUN-assisted connectivity fails.
- Model rooms with host authority for validating commands and broadcasting game events.
- Support two host-capable peers so a room can survive one host unexpectedly disconnecting.
- Support 2-5 players in the first version while preserving room and protocol assumptions that can scale to 10 players later.
- Keep first-version networking unencrypted and aimed at trusted friend games rather than adversarial play.
- Defer graphical desktop UI work until after the CLI networking and gameplay foundations are validated.

## Capabilities

### New Capabilities

- `uno-cli-client`: Cross-platform Rust CLI commands and interactive flows for hosting, joining, and playing UNO games.
- `uno-gameplay`: UNO rules, game state, player actions, turn progression, and event validation.
- `networked-rooms`: Room creation, join flows, player membership, shared connection information, and host failover behavior.
- `peer-connectivity`: STUN-assisted UDP connectivity, direct peer session establishment, and port-forwarding fallback behavior.

### Modified Capabilities

None.

## Impact

- Introduces a new Rust project structure for CLI, gameplay, and networking components.
- Adds runtime dependencies for command-line parsing, async networking, serialization, logging/tracing, and STUN/UDP connectivity.
- Defines local protocol and state synchronization contracts that future desktop UI clients must reuse rather than reimplement.
- Establishes a public networking model that does not depend on a dedicated game server, while accepting that some networks will require manual port forwarding.
