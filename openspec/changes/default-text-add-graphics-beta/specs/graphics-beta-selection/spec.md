## ADDED Requirements

### Requirement: Setup offers explicit text and beta graphics choices
The setup screen SHALL offer `Text` and `Graphics (Beta)` choices, SHALL display the effective image backend or text fallback reason for Graphics Beta, and SHALL display Text without labeling it as a manual selection.

#### Scenario: Text choice is displayed
- **WHEN** Text is the initial choice or is selected by the user
- **THEN** the setup screen displays `Text` in English or `文字` in Chinese and renders no image cards

#### Scenario: Graphics Beta choice is displayed
- **WHEN** Graphics Beta is selected and an image backend is available
- **THEN** the setup screen identifies Graphics Beta together with the effective iTerm2, Sixel, or Kitty backend

### Requirement: Windows Terminal is the only graphics-default environment
The application SHALL default to Graphics Beta in local Windows Terminal sessions, including WSL, and SHALL default to Text in WezTerm and every other local terminal on Windows, Linux, macOS, or Unix.

#### Scenario: Native Windows Terminal starts
- **WHEN** `WT_SESSION` identifies Windows Terminal and the environment is not WezTerm or SSH
- **THEN** Graphics Beta is the initial setup choice

#### Scenario: WSL hosted by Windows Terminal starts
- **WHEN** a Linux process inherits `WT_SESSION` from Windows Terminal and the environment is not WezTerm or SSH
- **THEN** Graphics Beta is the initial setup choice

#### Scenario: WezTerm inherits Windows Terminal state
- **WHEN** WezTerm is identified and `WT_SESSION` is also present
- **THEN** Text is the initial setup choice

#### Scenario: Another terminal starts
- **WHEN** neither Windows Terminal nor SSH is identified
- **THEN** Text is the initial setup choice regardless of operating system or detected image protocol

### Requirement: Local users can opt into beta graphics
The application SHALL continue detecting local graphics capabilities once at startup and SHALL allow a user in any local terminal to select Graphics Beta and use an accepted detected backend or its safe text fallback.

#### Scenario: Text-default WezTerm opts into graphics
- **WHEN** the user selects Graphics Beta in a local WezTerm session with iTerm2 support
- **THEN** the application renders image cards using the cached iTerm2 backend

#### Scenario: Unsupported local terminal opts into graphics
- **WHEN** the user selects Graphics Beta and no accepted backend was detected
- **THEN** the application remains playable with text cards and reports the unsupported fallback

### Requirement: SSH remains text-only
The application MUST default to Text, MUST skip graphics capability queries, and MUST resolve Graphics Beta to the SSH text fallback whenever an SSH session variable is present.

#### Scenario: SSH session starts inside Windows Terminal
- **WHEN** an SSH session variable and `WT_SESSION` are both present
- **THEN** Text is the initial choice and no graphics query or image payload is emitted
