# Changes from upstream

This is the Asciiverse edition of TermTrack by the TermTrack team (TageDan
and abbfelarb; MIT, Copyright (c) 2024 TermTrack), continuing from
upstream's main at 90d3acb (2025-05-16), after their 1.0.0 release of
2024-10-07. The `upstream` remote is https://github.com/TermTrack/TermTrack;
the `asciiverse` branch starts from it.

## Plays in any terminal

- Upstream's main did not build: input had moved from device_query (the
  desktop's keyboard under X11, not the terminal's) to the terminal's own
  key events in main.rs and the menus, but not in the level itself.
  `src/input.rs` now reads the terminal for all of it.
- Terminals with the kitty keyboard protocol, and the Windows console,
  report key releases: a key is held from press to release, as upstream
  meant. Others (GNOME Terminal, Konsole, xterm, tmux, macOS Terminal) send
  no releases; there a key counts as held until its key repeat stops, the
  repeat delay learned from the player's keyboard, and a key that was
  repeating stays held while another is pressed.
- Menus take one step per key press instead of polling held keys in busy
  loops (which also kept a CPU core at 100% while a menu was open).
- `Q` or `Ctrl-C` quits; `Esc` works as `E`; `R` retries from the end
  screens too. The terminal is restored on exit and on a panic.
- The menu's arrows are U+2190 to U+2193 instead of Supplemental Arrows-C
  glyphs that many fonts lack; the hint for `L` is shorter, so it no longer
  breaks mid-word at 80 columns.

## Offline: your own best times

- The online leaderboard (danielsson.pythonanywhere.com, validated with a
  secret salt given at build time) is gone, and with it reqwest, sha3, hex
  and serde. The game makes no network connections.
- Every finished run is kept in `$XDG_STATE_HOME/termtrack/times`
  (`~/.local/state/termtrack`, `%LOCALAPPDATA%\termtrack` on Windows), keyed
  as the leaderboard keyed levels: name plus CRC32 of the level file.
- `L` in the menu shows your best times on a level; the end screen shows
  your time, whether it is a new best, and your best five. The name entry
  went with the leaderboard.

## Silent

- The six MP3s in `sounds/` are third-party recordings whose terms are not
  in the repository (`menu.mp3` is tagged "Bit Shift" by Kevin MacLeod,
  `enter.mp3` "Notification - positive bleep" by Joshua Chivers, `pop.mp3`
  "Mouth pop - single loud" by THE FOUNDATION; `background.mp3`,
  `jump.mp3` and `walk.mp3` carry no tags). They are left out, with rodio
  and `src/audio.rs`.
- `_term_track_/` (the 2024 prebuilt zips, binaries plus those sounds) is
  left out; releases are built from source.

## Levels beside the program

- `level_pack_0`'s four levels joined `showcase.txt` in `levels/`.
- With no argument the game plays `levels/` beside the program (where the
  release archive puts it), else `levels/` in the current folder; a folder
  named on the command line works as before. Levels are listed in name
  order (upstream listed them in directory order).

## Session logging

- `TERMTRACK_LOG=<file>` (or `RUST_LOG=debug` with stderr) logs every key
  event as it arrives, every change of screen (menu, level, map, exit
  dialog, end of a run and why), and a snapshot of the player twice a
  second (position, turn, floor, time, keys held, fps); `RUST_LOG=trace`
  adds a line for every frame. log and env_logger.

## Other

- The unused `terminal-renderer` git dependency is dropped.
- A test loads every shipped level.
- `.gitignore` no longer lists `Cargo.lock`, which is committed and used
  with `--locked`.
- Upstream's `static.yml` (a GitHub Pages deploy of the prebuilt zips) is
  replaced by `release.yml`.

## Packaging (new files)

- `.github/workflows/release.yml`: static musl Linux (x86_64, aarch64),
  macOS (Apple silicon, Intel) and Windows (x86_64, aarch64) archives named
  for av, with `levels/` beside the program, and SHA256SUMS; Rust 1.93.1.
- `asciiverse/record-preview.py` and `asciiverse/termtrack.cast`: the 100x30
  catalog preview, level_1 run by a route bot in a pty that speaks the kitty
  keyboard protocol, written as asciicast v3 at 10 frames a second with
  only changed cells (colours in steps of 7) to stay under 2 MB.
- README.md rewritten for players, keeping upstream's level format; this
  file.

- **The view fills the terminal** (`src/renderer.rs`): upstream capped the
  3D view at 220x65 cells (every cell is a traced ray) and left the rest of
  a bigger terminal black, in the top left corner. Now the view is rendered
  within the same budget, scaled down evenly so its shape stays true, and
  stretched over the whole terminal, so a big window is filled at the same
  frame rate (about 36 fps at 300x90 here). Menus centre on the whole
  terminal. Also: after each row the colour memory is reset with the
  background, so a row never starts with a cell drawn black.
