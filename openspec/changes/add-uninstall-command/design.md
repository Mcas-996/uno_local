## Context

UNO is a single Rust executable distributed by cargo-dist shell and PowerShell installers with `install-path = "CARGO_HOME"` and a sibling `uno-update` executable. Those installers write an `uno-receipt.json` under the user's configuration directory. The CLI currently parses a small fixed set of arguments without Clap, the application stores no persistent user data, and crate-level lints forbid unsafe code. Windows does not allow a running executable to synchronously delete itself.

## Goals / Non-Goals

**Goals:**

- Remove only a verified cargo-dist installation of the running UNO executable.
- Provide an interactive confirmation and explicit `-y`/`--yes` bypass.
- Handle modern and legacy cargo-dist receipt prefix layouts on Windows, macOS, and Linux.
- Preserve retry information when binary deletion fails.

**Non-Goals:**

- Uninstall package-manager, `cargo install`, development, unmanaged, or manually copied builds.
- Remove shared PATH entries, Cargo metadata, `CARGO_HOME/bin`, or unrelated application data.
- Add an updater or change release packaging.

## Decisions

1. **Treat the receipt as authorization, not as a deletion manifest.** Parse the provider, source app name, binaries, and install prefix, then require the canonical running executable directory to equal either the prefix or its `bin` child. Derive deletion targets from the verified running executable and known `uno-update` name. This prevents a forged or stale receipt from selecting arbitrary files. Without a matching receipt, return guidance instead of falling back to self-deletion.

2. **Keep the existing explicit CLI parser.** Accept only `--uninstall`, `--uninstall -y`, and `--uninstall --yes`; all other combinations retain the existing unknown/additional-argument error behavior. Interactive confirmation prints the three paths, flushes stdout, and accepts trimmed ASCII-case-insensitive `y` or `yes`. Any other answer, blank input, or EOF is a successful cancellation.

3. **Use platform-specific self-removal.** Unix removes the optional updater, running executable, and receipt synchronously. Windows launches hidden Windows PowerShell with constant script text and passes all paths through environment variables, avoiding command interpolation. The helper waits for the caller to exit, retries file removal for bounded time, and removes the receipt only after both binaries are absent. Successfully starting the helper reports that cleanup is scheduled; helper startup failure is an immediate error.

4. **Leave shared environment configuration intact.** `CARGO_HOME/bin` may contain Rust and other cargo-dist applications, and the receipt records whether PATH modification was allowed rather than whether UNO uniquely created a PATH entry. The implementation therefore removes neither directories above the receipt folder nor registry/dotfile PATH entries. The receipt folder is removed only if empty.

5. **Test through copied binaries.** Integration tests copy the built UNO executable into isolated temporary install trees, create representative receipts and updater placeholders, and override HOME/config environment variables. This exercises real self-removal without touching the developer's installation.

## Risks / Trade-offs

- [A Windows cleanup helper can fail after the caller has returned success] → Report the operation as scheduled rather than completed, retry locked files, and retain the receipt when either binary remains so the user can retry.
- [Another running UNO process can keep the Windows executable locked] → Use bounded retries and document that all UNO processes should be closed before retrying.
- [Old receipts represent `install_prefix` inconsistently] → Accept only the two known safe relationships between the current executable directory and receipt prefix instead of branching on provider version.
- [Adding JSON parsing increases direct dependencies] → Reuse the existing transitive `serde` package, add the small `serde_json` parser, and deserialize only required receipt fields.

## Migration Plan

Ship the command in the next normal cargo-dist release. Existing users who update through `uno-update` already have a compatible receipt and can then uninstall. Rollback is a source revert; no persistent format owned by UNO is introduced.

## Open Questions

None.
