## 1. CLI and Receipt Validation

- [x] 1.1 Add JSON dependencies and implement receipt discovery, parsing, ownership checks, and safe uninstall target derivation
- [x] 1.2 Extend CLI parsing, help output, confirmation input, cancellation, and force flags

## 2. Cross-Platform Removal

- [x] 2.1 Implement synchronous Unix removal with optional updater handling and receipt cleanup
- [x] 2.2 Implement deferred Windows PowerShell cleanup with safe path passing, retries, and receipt retention on failure

## 3. Tests and Documentation

- [x] 3.1 Add unit tests for receipt validation, legacy layouts, prompt answers, and invalid sources
- [x] 3.2 Add isolated copied-binary integration tests for confirmation, force flags, cancellation, mismatch protection, and eventual Windows deletion
- [x] 3.3 Document uninstall usage, supported installation sources, preserved shared state, and the manual platform matrix

## 4. Verification

- [x] 4.1 Run formatting, all-target tests, Clippy with warnings denied, and OpenSpec validation
