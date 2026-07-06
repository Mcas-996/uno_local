# UNO Laptop Client

Rust workspace for a cross-platform UNO CLI client. The first version focuses on public-internet multiplayer foundations before a desktop UI.

## Checks

```powershell
cargo fmt --check
cargo test
cargo clippy --workspace --all-targets -- -D warnings
```

## Run

```powershell
cargo run -p uno -- host --name Alice --port 34567
cargo run -p uno -- join "<share-string>" --name Bob
```

Use `--no-stun` for local debugging or `--forwarded IP:PORT` when the host has configured UDP port forwarding.
