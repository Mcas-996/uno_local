## ADDED Requirements

### Requirement: Graphics Beta exposes only Sixel
The application SHALL expose Sixel as its only terminal image backend and SHALL represent every non-Sixel result as Text with a fallback reason. This requirement supersedes completed historical requirements that exposed Kitty or iTerm2 IIP as valid application backends.

#### Scenario: Sixel backend label
- **WHEN** Graphics Beta is selected and the effective backend is graphical
- **THEN** the setup screen identifies the choice as Graphics Beta with Sixel

#### Scenario: Unsupported protocol label
- **WHEN** Graphics Beta is selected but the environment does not resolve to Sixel
- **THEN** the setup screen identifies the effective Text fallback and its applicable reason

### Requirement: Terminal capability resolution is Sixel-only
The application SHALL resolve SSH directly to `Text(Ssh)` without querying graphics capabilities and SHALL enable graphics on any local terminal only when the picker reports Sixel. The application SHALL NOT bypass a dependency compatibility blacklist. Kitty, iTerm2 IIP, Halfblocks, missing results, and query failures SHALL resolve to `Text(Unsupported)`.

#### Scenario: SSH skips graphics query
- **WHEN** any supported SSH session marker is non-empty
- **THEN** the runtime selects `Text(Ssh)` without constructing or querying a graphics picker

#### Scenario: WezTerm respects compatibility fallback
- **WHEN** a local WezTerm session obtains a picker whose compatibility result is not Sixel
- **THEN** the runtime resolves Graphics Beta to `Text(Unsupported)` and emits no image data

#### Scenario: Windows Terminal detects Sixel
- **WHEN** a local Windows Terminal session reports Sixel
- **THEN** the runtime resolves Graphics Beta to Sixel

#### Scenario: Non-WezTerm local terminal detects Sixel
- **WHEN** any other local terminal reports Sixel
- **THEN** the runtime resolves Graphics Beta to Sixel

#### Scenario: Non-Sixel local capability
- **WHEN** a non-WezTerm local terminal reports Kitty, iTerm2 IIP, Halfblocks, no protocol, or a query failure
- **THEN** the runtime resolves Graphics Beta to `Text(Unsupported)`

### Requirement: Graphics defaults remain terminal-specific
The application SHALL default Windows Terminal to Graphics Beta and SHALL default WezTerm and other terminals to Text, while allowing a user to manually select Graphics Beta and see the effective Sixel or Text result.

#### Scenario: Windows Terminal default
- **WHEN** setup opens in Windows Terminal without an explicit session choice
- **THEN** Graphics Beta is the selected choice

#### Scenario: WezTerm default and unsupported opt-in
- **WHEN** setup opens in WezTerm without an explicit session choice
- **THEN** Text is selected and manually selecting Graphics Beta reports the unsupported Text fallback without emitting image data

### Requirement: Sixel encoding is independent of panel position
The runtime SHALL fit a card image to the panel, center the resulting size in the UI, and cache the encoded protocol by card and fitted size. Selected and discard previews SHALL retain independent cache slots.

#### Scenario: Position-only change reuses encoding
- **WHEN** the same card is rendered at the same fitted size at a different screen position
- **THEN** the runtime reuses the cached Sixel protocol without encoding again

#### Scenario: Card or size change re-encodes
- **WHEN** the card or fitted size changes for a preview slot
- **THEN** the runtime creates a new Sixel protocol for that slot

#### Scenario: Preview slots remain independent
- **WHEN** selected and discard previews render the same card and size
- **THEN** each slot owns a distinct cached protocol instance

### Requirement: Encoding failure degrades stably to Text
The runtime SHALL respond to a protocol encoding failure by selecting `Text(Encoding)`, dropping the picker, clearing both preview caches, and making no further encoding attempts during the process lifetime.

#### Scenario: Encoding failure clears graphics state
- **WHEN** Sixel protocol creation fails in either preview slot
- **THEN** the picker and both preview caches are cleared and the effective backend becomes `Text(Encoding)`

#### Scenario: Rendering continues after failure
- **WHEN** later frames render after an encoding failure
- **THEN** the full Text card interface remains usable and no graphics encoding is retried

### Requirement: Protocol ownership remains in ratatui-image
The application SHALL use crates.io `ratatui-image 11.0.6` for Sixel encoding, rendering, cleanup, and Ratatui integration and SHALL NOT retain an application-owned Kitty/IIP implementation or an inactive external source snapshot. This requirement supersedes the historical external snapshot and WezTerm IIP positioning requirements.

#### Scenario: Build dependency remains crates.io release
- **WHEN** Cargo resolves application dependencies
- **THEN** `ratatui-image 11.0.6` is resolved from crates.io without a local path patch

#### Scenario: Repository capability surface
- **WHEN** current source, tests, UI labels, and active project documentation are inspected
- **THEN** they contain no branch or claim that Kitty or iTerm2 IIP is a supported application graphics backend

#### Scenario: Unsupported terminal remains usable
- **WHEN** a terminal supports Kitty or iTerm2 IIP but not Sixel
- **THEN** the application uses Text and emits no Kitty or iTerm2 image data
