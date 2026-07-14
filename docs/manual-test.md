# Local TUI Manual Test

Automated tests validate backend resolution, generated pixels, caching, responsive layout, and overlay invalidation with Ratatui's test backend. Native iTerm2/Sixel placement, ConPTY behavior, GPU rendering, and cross-platform terminal cleanup remain platform-only checks covered by the matrix below and by the existing cargo-dist release jobs.

## Launch and setup

1. Run `cargo run -p uno` in a terminal at least 70 × 22 cells.
2. Edit the player name and select 1, 2, 3, and 4 AI opponents in separate runs.
3. Confirm Easy, Normal, and Hard can each start a match.
4. Run `uno --help` and confirm it exits without entering raw terminal mode.
5. From a directory without Cargo or Git files, run `uno -v` and `uno --version`; confirm both print the same package version and 12-character Git commit, then exit without entering raw terminal mode.
6. Run `uno --help` and confirm it documents `--uninstall`, `-y`, and `--yes`.

## Managed uninstall

1. Install a release with the cargo-dist shell or PowerShell installer into a path containing spaces; on Windows also exercise a path containing `&`.
2. Run `uno --uninstall`, confirm it lists `uno`, `uno-update`, and `uno-receipt.json`, then enter `n`, an empty line, and end-of-input in separate installations. Confirm each cancels successfully and preserves every file.
3. Run `uno --uninstall` again and enter mixed-case `y` or `yes`; on Linux and macOS confirm the files are gone when the command returns, and on Windows confirm the scheduled cleanup removes them shortly after the process exits.
4. Reinstall and repeat with `uno --uninstall -y` and `uno --uninstall --yes`; confirm neither invocation prompts.
5. Run a development build while a separate release receipt exists. Confirm uninstall is refused and neither copy is removed.
6. Confirm unrelated files in `CARGO_HOME/bin`, the directory itself, shell startup files, and the Windows user PATH remain unchanged.
7. On Windows, keep another UNO process open during uninstall. Confirm cleanup retries and retains the receipt if the executable stays locked; close all UNO processes and retry.

## Gameplay

1. Select cards with the arrow keys and play with Enter.
2. Draw with `D`; confirm a second draw is rejected and only the drawn card can be played before passing.
3. Play a wild card and confirm the color picker can be confirmed or cancelled.
4. Open `:` and exercise `play <index>`, `draw`, `pass`, `help`, `new`, and `quit`.
5. Complete a match and start a new one from the result screen.

## Terminal behavior

1. Resize below 70 × 22 and confirm the resize prompt appears.
2. Open and close help with `?` and Esc.
3. Cancel and confirm the quit dialog.
4. Press Ctrl+C during a match and confirm the shell returns to a normal visible cursor and echo state.

## Card graphics

1. Start locally in Windows WezTerm at exactly 70 × 26; confirm Setup defaults to `Text` and emits no card images even though WezTerm supports iTerm2.
2. Select `Graphics (Beta)` and confirm Setup reports `Graphics (Beta) (iTerm2)` and both the selected hand card and discard top are wholly inside their own panels. Repeat in a normal-sized window and at 159 × 41 with 192 DPI scaling; confirm both images are centered and opposite gaps differ by at most one terminal cell.
3. Select several cards, play a card, draw a card, and start a new match. Confirm the selected and discard images stay in their respective panels and do not drift when their card contents change.
4. Resize WezTerm repeatedly, including transitions across 70 × 25 and 70 × 26. Confirm text/image mode changes cleanly, centered positions recompute, and old images leave no residue.
5. Open and close Help, quit confirmation, the wild-color picker, and the result screen in WezTerm; confirm images never cover an overlay and return at the correct positions afterward.
6. Exit WezTerm normally, with Ctrl+C, and through a forced panic in a debug build; confirm no image remains after the shell prompt returns.
7. Repeat in Windows Terminal 1.22 or newer from native PowerShell and local WSL; confirm Setup defaults to `Graphics (Beta) (Sixel)`, both previews render without scrolling, and WSL follows Windows Terminal rather than the Linux default.
8. Connect through OpenSSH, a WezTerm SSH domain, and SSH inside WSL; confirm Setup defaults to `Text`, no capability-query garbage appears, and cards remain colored text. Select Graphics Beta and confirm it reports the SSH text fallback without emitting image data.
9. Set only `SSH_AUTH_SOCK` in a local shell; confirm it does not force the SSH fallback.
10. In Linux, macOS, and a Windows console other than Windows Terminal, confirm Setup defaults to `Text`; manually select `Graphics (Beta)` and confirm a supported detected backend is used or an explicit text fallback is reported.
11. Select `Graphics: Text`; confirm Setup reports `Text` and no image is rendered at any terminal size, including 70 × 26 and 159 × 41.

## Localization

1. Start under a `zh-CN` or other `zh*` locale and confirm Chinese setup, game, help, result, and errors.
2. Start under a non-Chinese or unavailable locale and confirm English fallback.
3. On the setup screen, select Language/语言 and use Left/Right to switch between English and Simplified Chinese; confirm the whole screen updates immediately.
