## ADDED Requirements

### Requirement: Managed installations can be uninstalled from the CLI
UNO SHALL expose `--uninstall` for cargo-dist-managed shell and PowerShell installations and SHALL display the resolved UNO executable, updater, and receipt paths before interactive removal.

#### Scenario: User confirms interactive uninstall
- **WHEN** a matching managed installation runs `uno --uninstall` and the user enters `y` or `yes` without regard to ASCII case
- **THEN** UNO removes or schedules removal of the installed `uno`, sibling `uno-update`, and install receipt

#### Scenario: User cancels interactive uninstall
- **WHEN** the user enters any other response, an empty line, or reaches end-of-input at the confirmation prompt
- **THEN** UNO leaves all files unchanged, prints a cancellation message, and exits successfully

#### Scenario: User explicitly bypasses confirmation
- **WHEN** a matching managed installation runs `uno --uninstall -y` or `uno --uninstall --yes`
- **THEN** UNO performs the uninstall without reading confirmation input

### Requirement: Uninstall validates ownership before deletion
UNO MUST require a readable cargo-dist receipt for the UNO application whose installation prefix matches the running executable before deleting or scheduling deletion of any file.

#### Scenario: Receipt is absent or invalid
- **WHEN** `--uninstall` is run without a readable matching UNO cargo-dist receipt
- **THEN** UNO exits with an error, leaves all files unchanged, and directs the user to the original package manager or manual removal

#### Scenario: Receipt belongs to another executable location
- **WHEN** an UNO receipt exists but its install prefix does not match the running executable directory or its parent layout
- **THEN** UNO refuses the uninstall and does not delete the installed copy referenced by the receipt or the running copy

#### Scenario: Legacy cargo-dist prefix is used
- **WHEN** a valid receipt stores either the installation root or its `bin` directory as `install_prefix`
- **THEN** UNO accepts the receipt only when one of those locations canonically matches the running executable directory

### Requirement: Uninstall preserves shared installation state
UNO SHALL NOT remove shared PATH entries, shell configuration, Windows PATH registry values, Cargo metadata, or the shared installation directory while uninstalling.

#### Scenario: Other Cargo binaries share the install directory
- **WHEN** UNO is uninstalled from `CARGO_HOME/bin` alongside unrelated files
- **THEN** only UNO's executable, optional updater, receipt, and an otherwise-empty receipt directory are removed

### Requirement: Self-removal is platform safe
UNO SHALL remove itself synchronously where the operating system permits and SHALL defer removal until process exit on Windows without interpolating filesystem paths into executable script text.

#### Scenario: Unix uninstall succeeds
- **WHEN** a validated uninstall runs on Linux or macOS
- **THEN** all targeted files are absent before the command reports successful completion

#### Scenario: Windows uninstall is scheduled
- **WHEN** a validated uninstall runs on Windows and the cleanup helper starts successfully
- **THEN** UNO reports scheduled cleanup, exits successfully, and the helper removes the binaries before deleting the receipt

#### Scenario: Windows helper cannot start
- **WHEN** the Windows cleanup helper cannot be launched
- **THEN** UNO exits with an error and does not report that uninstall was scheduled
