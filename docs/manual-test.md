# Local TUI Manual Test

Automated tests validate backend resolution, generated pixels, caching, responsive layout, and overlay invalidation with Ratatui's test backend. Native iTerm2/Sixel placement, ConPTY behavior, GPU rendering, and cross-platform terminal cleanup remain platform-only checks covered by the matrix below and by the existing cargo-dist release jobs.

## Launch and setup

1. Run `cargo run -p uno` in a terminal at least 70 × 22 cells.
2. Edit the player name and select 1, 2, 3, and 4 AI opponents in separate runs.
3. Confirm Easy, Normal, and Hard can each start a match.
4. Run `uno --help` and confirm it exits without entering raw terminal mode.

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

1. Start locally in Windows WezTerm at 70 × 26 or larger; confirm Setup reports `Auto (iTerm2)` and the selected hand card plus discard top render as images.
2. Repeat in Windows Terminal 1.22 or newer from native PowerShell and local WSL; confirm Setup reports `Auto (Sixel)` and both previews render without scrolling the screen.
3. Resize each supported terminal between 70 × 25 and 70 × 26; confirm the UI switches cleanly between colored text cards and image previews without residue.
4. Open and close Help, quit confirmation, the wild-color picker, and the result screen; confirm images never cover an overlay and return correctly afterward.
5. Start a new match and exit normally, with Ctrl+C, and through a forced panic in a debug build; confirm no image remains after the shell prompt returns.
6. Connect through OpenSSH, a WezTerm SSH domain, and SSH inside WSL; confirm Setup reports `Auto (Text: SSH)`, no capability-query garbage appears, and cards remain colored text.
7. Set only `SSH_AUTH_SOCK` in a local shell; confirm it does not force the SSH fallback.
8. Select `Graphics: Text`; confirm Setup reports `Text (manual)` and no image is rendered at any terminal size.

## Localization

1. Start under a `zh-CN` or other `zh*` locale and confirm Chinese setup, game, help, result, and errors.
2. Start under a non-Chinese or unavailable locale and confirm English fallback.
3. On the setup screen, select Language/语言 and use Left/Right to switch between English and Simplified Chinese; confirm the whole screen updates immediately.
