# Actual TerraTopia terminal captures

All PNGs here are unedited captures of an actual Linux xterm window, with a
working X11/Xwayland display, DejaVu Sans Mono 12pt and 100x30 cells. They are
not generated art, composited UI or rasterized mock text. Client-area capture
excludes desktop surroundings and title bars. Images were visually inspected
for correct state, readable text and absence of personal information.

## Phase 2

- `crafting.png`: seed 827193, emergency supplies collected, iron/carbon mined,
  mining tool crafted, crystal gathered. Shelter kit selected; ingredient rows
  show owned/required iron, carbon and crystal.
- `survival.png`: kit crafted and placed at the initial position, its station
  powered with the emergency battery, and R scan performed. The player `@`
  occupies shelter `H`, so the shelter glyph is covered; the HUD says SHELTER.
  The neighbouring spacecraft `A` and starter resources are visible. Live fuel,
  vitals, objective, fog and named scan signals are genuine game output.

```sh
cargo build --release
python3 scripts/capture-terminal.py
```

The script follows Terrabox's capture approach, sends X key events only to the
xterm it creates, performs the gathering/crafting/building through game controls,
then exits via Q. An isolated temporary save path prevents replacing user saves.
Requirements: Python 3, libX11, xterm, xwininfo, ImageMagick `import` and a working
X11/Xwayland display. These are optional capture tools, not game dependencies.
The blue/cyan palette overrides match Terrabox's capture script; other colours
use xterm defaults. Colours vary with terminal themes.

## Historical Phase 1.1

- `scanner.png`: seed 827193, initial R scan, 1257 mapped cells and four sites.
- `exploration.png`: one move after scanning, with radius-7 sight and grey memory.

These older images are retained as historical artifacts. The current capture
script produces the Phase 2 files; the new resource mix/crash overlay means the
old scan counts are not a claim about current Phase 2 output.

Captures validate this emulator path, not native Windows/macOS/Termux/SSH,
physical keyboard behaviour or visible flicker. The full-night objective was
validated separately by the real-time PTY smoke test; these still images do not
claim to show a completed night.
