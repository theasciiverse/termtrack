# TermTrack

A terminal-rendered 3D platforming and maze game, made for speedrunning and
custom levels. Every frame is drawn in true colour, one character cell at a
time: corridors, half walls to climb, holes to jump, spikes, and the angry
pixels that hunt you. Find the end, as fast as you can.

This is the Asciiverse edition of TermTrack by the TermTrack team (TageDan
and abbfelarb, 2024). It plays in any terminal, offline, and keeps your best
times on your own computer. What changed is listed in PATCHES.md.

## Play

With [av](https://asciiverse.sh): `av install termtrack`, then start it from
av. From a release archive: unpack it and run `termtrack`; it plays the
levels in the `levels` folder beside it.

    termtrack                 # the levels beside the program
    termtrack my_levels       # a folder of your own levels

A terminal of at least 80x24 with true colour; bigger is better. Terminals
that report key releases (kitty, WezTerm, foot, Ghostty, Alacritty, Windows
Terminal) feel best; in others a key counts as held while it repeats.

## Controls

- `W` `A` `S` `D`: move
- arrow keys: turn and look up and down
- `Space`: jump
- `M`: the map of the floor you are on
- `R`: start the level again
- `E` or `Esc`: leave the level (or the game, from the menu)
- `Q` or `Ctrl-C`: quit at once

In the menu, `Up` and `Down` choose a level, `Enter` plays it and `L` shows
your best times on it. The clock starts when you first move.

## Your times

Each finished run is kept in `$XDG_STATE_HOME/termtrack/times`
(`~/.local/state/termtrack/times`; `%LOCALAPPDATA%\termtrack\times` on
Windows). A level is known by its name and its contents, so an edited level
starts a new list.

## Making levels

A level is a text file; everything before the last `.` in its name is the
level's name. Each character is one cell of the grid:

- `S` start
- `E` end
- `X` wall
- `x` half wall, a step up
- `v` spike
- `.` floor
- ` ` (space) hole
- `e` floor with an angry pixel

A line holding only `sep` starts the next floor up. For example:

    XXX
    XSX
    XvX
    X.X
    XxX
    XXX
    sep
    XXXXXXX
    X....EX
    X.XXXXX
    X.X
    X X
    XXX

Two floors: the start, a spike and a step up on the lower one, the end on
the upper one. Put your levels in a folder and run `termtrack <folder>`.

## Session log

`TERMTRACK_LOG=termtrack.log termtrack` writes every key as it arrives,
every change of screen and a snapshot of the player twice a second;
`RUST_LOG=trace` adds every frame. `RUST_LOG=debug termtrack 2>log` works
too.

## Building

    cargo build --release
    ./target/release/termtrack levels

## Credits

TermTrack by the TermTrack team: TageDan and abbfelarb, with thanks from
the team to GustavPetterssonBjorklund for suggesting security actions, and
to https://patorjk.com/software/taag/ for generating the title art. MIT
licence (LICENSE), Copyright (c) 2024 TermTrack.

Asciiverse edition: see PATCHES.md.
