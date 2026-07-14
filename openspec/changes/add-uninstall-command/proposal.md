## Why

Users who install UNO through the published cargo-dist shell or PowerShell installer can update the application, but they currently have no supported way to remove the installed executable, updater, and install receipt. A first-party uninstall command can remove only UNO-owned files while protecting shared Cargo paths and package-manager installations.

## What Changes

- Add `uno --uninstall` with an interactive `y`/`yes` confirmation.
- Add `uno --uninstall -y` and `uno --uninstall --yes` for explicit non-interactive confirmation.
- Validate the cargo-dist receipt against the running executable before deleting anything.
- Remove the installed `uno`, sibling `uno-update`, and receipt while preserving shared PATH and `CARGO_HOME` state.
- Support synchronous self-removal on Unix and deferred self-removal on Windows.
- Document and test the supported installation sources, cancellation, and failure behavior.

## Capabilities

### New Capabilities

- `managed-uninstall`: Safe removal of a cargo-dist-managed UNO installation through the UNO CLI.

### Modified Capabilities

None.

## Impact

The CLI entry point gains new flags and a dedicated uninstall module. Receipt parsing adds direct `serde` and `serde_json` dependencies. CLI integration tests, user documentation, and the manual cross-platform test matrix gain uninstall coverage; release packaging remains unchanged.
