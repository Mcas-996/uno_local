# Manual Test Commands

## Host with STUN

```powershell
cargo run -p uno -- host --name Alice --port 34567
```

Share the printed `uno://...` string with another peer.

## Join

```powershell
cargo run -p uno -- join "uno://v1/udp/203.0.113.10:34567/room/room-1/host/host-1" --name Bob
```

## Host with manual UDP forwarding

Forward UDP port `34567` on the router to the host machine, then run:

```powershell
cargo run -p uno -- host --name Alice --port 34567 --forwarded 203.0.113.10:34567
```

## Local Debug Flow

```powershell
cargo run -p uno -- host --name Alice --no-stun
```

Inside the prompt:

```text
ready
start
state
draw
pass
leave
```

## Simulating Disconnects

Start a host and at least one joiner, then terminate the active host process. A room with a synchronized standby host is expected to promote the standby host. If no synchronized standby host exists, the room reports that it cannot continue.
