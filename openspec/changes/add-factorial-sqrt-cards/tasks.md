## 1. Core Rules

- [x] 1.1 Add mathematical wild ranks, exact 130-card Holiday composition, wild legality, and high-wild deal filtering
- [x] 1.2 Implement overflow-safe Factorial sizing, bulk target draws, Square Root random discards, turn skipping, and effect events
- [x] 1.3 Add core tests for composition, formula boundaries, direction, seeded discards, final-card wins, and draw rules

## 2. Local Play and Presentation

- [x] 2.1 Add effect-aware AI scoring and tests for both mathematical wilds
- [x] 2.2 Add bilingual names, logs, help text, deck labels, and app event integration
- [x] 2.3 Add four-color graphical card art and coverage for the new labels

## 3. Large-Hand Safety

- [x] 3.1 Deduplicate legal actions and avoid repeated full-hand AI scans or clones
- [x] 3.2 Add cached/windowed filtered hand layout while preserving selection, navigation, commands, and both frontends
- [x] 3.3 Add large repeated-hand tests for bounded legal actions and visible-window rendering

## 4. Documentation and Verification

- [x] 4.1 Document Holiday 130 and both mathematical rules in README
- [x] 4.2 Run formatting, full tests, Clippy, and OpenSpec verification; fix all findings

## 5. Mathematical Wild Guarantees

- [x] 5.1 Pair Square Root and Factorial with the existing Draw Eight and Draw Sixteen guarantee/exclusion tiers
- [x] 5.2 Update deterministic deal tests for Easy, Normal, Hard, Extreme, human schedules, and refill behavior
- [x] 5.3 Run formatting, full tests, Clippy, and strict OpenSpec validation
