# UNO Online

UNO Online is a Rust workspace for a cross-platform, serverless UNO client. The current version is a CLI-first implementation that focuses on proving the hard parts before a desktop UI is added: public-internet connectivity, room creation, host failover, protocol boundaries, and reusable UNO game logic.

The project is designed for macOS, Windows, and Linux. It currently avoids external runtime services: there is no game server, account system, matchmaking service, or central room directory.

## Current Status

This repository contains an early but working foundation:

- Rust workspace with separated crates for CLI, gameplay, protocol, networking, and room logic.
- CLI commands for hosting and joining rooms.
- Share strings for serverless room joining.
- UDP socket setup and direct join handshake.
- STUN Binding Request support for public endpoint discovery.
- Manual UDP port-forwarding fallback.
- Active host plus standby host room model.
- UNO gameplay state, card model, deterministic setup, basic action validation, and event sequencing.
- Unit tests for gameplay, protocol, networking, room failover, and CLI parsing.

Important limitation: the `join` command currently proves the network handshake path, but it does not yet enter a full remote interactive game session. The `host` command has the local debug prompt.

## Repository Layout

```text
crates/
  uno_core/       UNO cards, rules, game state, commands, events
  uno_protocol/   room ids, peer ids, share strings, wire messages
  uno_net/        UDP sockets, STUN discovery, handshakes, reliable message queue
  uno_room/       room membership, active/standby host roles, failover state
  uno_cli/        command-line host/join interface

docs/
  manual-test.md  manual test commands and debugging notes

openspec/
  change proposal, design, specs, and implementation task tracking
```

## Requirements

- Rust 1.91 or newer
- Cargo
- Network access if testing STUN or GitHub operations
- UDP port access for multiplayer testing

No third-party Rust crates are currently used. The first implementation is intentionally std-only so it can compile without downloading registry dependencies.

## Build and Check

```powershell
cargo fmt --check
cargo test
cargo clippy --workspace --all-targets -- -D warnings
```

On this codebase, the test suite covers gameplay, protocol parsing, local UDP handshakes, fallback behavior, host failover, and CLI parsing.

## Release Distribution

Releases are configured with `cargo-dist` and GitHub Releases. Push a version tag such as `v0.1.0` to build and publish release artifacts.

Unix install:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Mcas-996/uno_online/releases/latest/download/uno-installer.sh | sh
```

Windows PowerShell install:

```powershell
irm https://github.com/Mcas-996/uno_online/releases/latest/download/uno-installer.ps1 | iex
```

## Running the CLI

Show help:

```powershell
cargo run -p uno -- help
```

Host a room:

```powershell
cargo run -p uno -- host --name Alice --port 34567
```

The host prints a share string:

```text
share: uno://v1/udp/203.0.113.10:34567/room/room-123456/host/host-123456
```

Send the whole `uno://...` string to another player.

Join a room:

```powershell
cargo run -p uno -- join "uno://v1/udp/203.0.113.10:34567/room/room-123456/host/host-123456" --name Bob
```

Local debug mode without STUN:

```powershell
cargo run -p uno -- host --name Alice --no-stun
```

Single-player debug mode:

```powershell
cargo run -p uno -- host --name Alice --peer alice --no-stun --debug
```

Host with manual UDP port forwarding:

```powershell
cargo run -p uno -- host --name Alice --port 34567 --forwarded 203.0.113.10:34567
```

## Interactive Host Commands

After hosting, the CLI enters a local debug prompt:

```text
uno>
```

Supported commands:

```text
ready
start
state
hand
draw
pass
play red 5
play blue skip
play green reverse
play red wild
leave
```

In `--debug` mode, `start` is allowed with one connected player and `play` requires an explicit player id:

```text
play alice red 5
play alice blue skip
play alice red wild
```

The prompt is currently useful for validating local room/gameplay behavior. Full remote command forwarding from joiners to the active host is a remaining implementation task.

## How It Works

### 1. Serverless Room Discovery

There is no central room directory. A host creates a room locally and prints a share string.

```text
Host
  |
  | creates room
  v
uno://v1/udp/<endpoint>/room/<room-id>/host/<host-id>
```

The share string contains the minimum information a joiner needs:

- protocol version: `v1`
- transport: `udp`
- host endpoint: IP address and UDP port
- room id
- host peer id
- optional forwarded-endpoint marker

Because there is no server, the application cannot provide short global room codes yet. The share string is the room invitation.

### 2. STUN-Assisted Public Endpoint Discovery

Most players are behind NAT. The host usually knows only its local address, such as `192.168.x.x`, which is not reachable from the public internet.

STUN helps the host discover how it appears from the public internet:

```text
Host client ---- STUN Binding Request ----> STUN server
Host client <--- public IP:port mapping ---- STUN server
```

The client can then include the discovered public UDP endpoint in the share string.

STUN does not relay game traffic. It only helps discover the public mapping. After discovery, peers still try to communicate directly over UDP.

### 3. UDP Join Handshake

The joiner parses the share string and sends a UDP join request to the host endpoint:

```text
Joiner                                Host
  |                                    |
  | JoinRequest(room, peer, name)      |
  |----------------------------------->|
  |                                    |
  | JoinResponse(accepted/rejected)    |
  |<-----------------------------------|
```

The host validates the room id and returns a join decision. If no response arrives before the timeout, the CLI reports a handshake timeout and suggests the port-forwarding fallback.

### 4. Port-Forwarding Fallback

STUN is not guaranteed to work on every network. Symmetric NAT, some mobile networks, campus networks, company networks, or carrier-grade NAT can block direct UDP connectivity.

When automatic connectivity fails, the host can manually forward a UDP port on their router:

```text
Router UDP 34567 -> Host machine UDP 34567
```

The host then runs:

```powershell
cargo run -p uno -- host --port 34567 --forwarded <public-ip>:34567
```

The generated share string uses that forwarded endpoint.

### 5. Host-Authoritative Gameplay

The active host is authoritative for trusted-friend games:

```text
Player command -> Active host -> validate -> accepted event -> peers
```

The active host is responsible for:

- validating turn order
- checking card ownership
- checking whether a card is playable
- applying draw, pass, reverse, skip, wild, and draw-card effects
- generating ordered game events
- broadcasting accepted events

This is simpler than full peer-to-peer consensus and fits the first version. It does not provide anti-cheat protection against a malicious host.

### 6. Event-Based State Synchronization

The game model uses commands and events rather than arbitrary UI state.

Examples of commands:

```text
PlayCard
Draw
Pass
ChooseColor
```

Examples of events:

```text
GameStarted
CardPlayed
CardDrawn
TurnPassed
ColorChosen
GameWon
```

Each accepted event has a sequence number. Peers apply events in order and ignore duplicates. This gives the later desktop UI a clean contract: render state derived from the event stream, not from network-specific internals.

### 7. Active Host and Standby Host

Rooms are designed to have one active host and one standby host:

```text
Active Host
  |
  | replicated room/game state
  v
Standby Host
```

The standby host receives enough replicated state to take over if the active host disconnects:

- membership
- host roles
- game state
- event sequence position
- pending critical message state

If the active host disconnects and the standby host is synchronized, the standby is promoted. If no synchronized standby exists, the room is suspended or ended.

## Protocol Notes

The current protocol is intentionally small and text-based. It includes:

- `JoinRequest`
- `JoinResponse`
- `Membership`
- `Ready`
- `Command`
- `Event`
- `Ack`
- `Ping`
- `Disconnect`

Critical UDP messages use session ids, sequence numbers, acknowledgements, retries, and timeouts. This gives the project a reliability layer without committing yet to QUIC, WebRTC, or a third-party networking stack.

## Security Model

The first version is for trusted friend games.

Current non-goals:

- no encryption
- no authentication
- no accounts
- no matchmaking
- no anti-cheat
- no hosted relay
- no TURN fallback

Traffic is plaintext by design for this first version.

## License

This project is licensed under the GNU General Public License v3.0 only. See [LICENSE](LICENSE).

## Roadmap

Remaining work tracked in OpenSpec includes:

- fuller UNO rule coverage
- complete serialization/deserialization for all event payloads
- active-host broadcast of accepted events to connected peers
- full remote interactive gameplay for joiners
- multi-process integration tests
- possible migration from std-only code to `clap`, `tokio`, `serde`, and `tracing`
- later desktop UI with a native desktop-app feel

## Manual Testing

See [docs/manual-test.md](docs/manual-test.md) for manual host, join, forwarded-port, and disconnect testing commands.
