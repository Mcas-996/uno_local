# Local TUI Manual Test

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

## Localization

1. Start under a `zh-CN` or other `zh*` locale and confirm Chinese setup, game, help, result, and errors.
2. Start under a non-Chinese or unavailable locale and confirm English fallback.
