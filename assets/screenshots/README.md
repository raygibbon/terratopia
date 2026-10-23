# Actual TerraTopia terminal captures

These are unedited window captures of the running TerraTopia release binary in
Linux xterm on X11/Xwayland, using seed 827193 at 100x30 cells. The font is
DejaVu Sans Mono at 12pt. Capture excludes desktop surroundings and title bars.
No generated art, compositing, text rasterization or mocked gameplay is used.

- `scanner.png`: N, Enter, R. A radius-20 scan reveals 1257 total cells and four
  sites; the three nearest signals appear in the LAST SCAN panel.
- `exploration.png`: one D movement afterwards. Local sight returns to radius 7,
  and the rest of the scanned area is remembered in grey. The panel explicitly
  retains the last scan's report.

```sh
cargo build --release
python3 scripts/capture-terminal.py
```

Capture tools: Python 3, libX11, xterm, xwininfo and ImageMagick `import`, with a
working X11/Xwayland display. These are optional developer tools, not game
runtime dependencies. The script follows Terrabox's capture approach and sends
synthetic X key events only to the xterm window it creates. It captures the
window and exits via Q. Both images were visually inspected for correct fog,
readable terrain and scanner text, and absence of personal information.

Palette entries blue/cyan use the same overrides as Terrabox's capture script;
other colours follow xterm defaults. The final game uses a black background for
wasteland to keep its magenta glyphs readable. Colours vary with terminal themes.
These captures validate this particular emulator path, not Windows, macOS,
Termux, SSH or physical keyboard behaviour.
