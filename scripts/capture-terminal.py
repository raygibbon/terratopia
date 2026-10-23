#!/usr/bin/env python3
"""Capture TerraTopia in an actual xterm window (X11/Xwayland required).

Run `cargo build --release` first, then this script from the repository.
Requires xterm, xwininfo, ImageMagick import and libX11; no runtime dependency.
"""
import ctypes as C
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets/screenshots"

# Send actual X key events only to the emulator windows created by this script.
x = C.CDLL("libX11.so.6")
x.XOpenDisplay.restype = C.c_void_p
x.XStringToKeysym.argtypes = [C.c_char_p]
x.XStringToKeysym.restype = C.c_ulong
x.XKeysymToKeycode.argtypes = [C.c_void_p, C.c_ulong]
x.XKeysymToKeycode.restype = C.c_uint

class KeyEvent(C.Structure):
    _fields_ = [("type", C.c_int), ("serial", C.c_ulong), ("send_event", C.c_int),
                ("display", C.c_void_p), ("window", C.c_ulong), ("root", C.c_ulong),
                ("subwindow", C.c_ulong), ("time", C.c_ulong), ("x", C.c_int),
                ("y", C.c_int), ("x_root", C.c_int), ("y_root", C.c_int),
                ("state", C.c_uint), ("keycode", C.c_uint), ("same_screen", C.c_int)]

class XEvent(C.Union):
    _fields_ = [("key", KeyEvent), ("pad", C.c_long * 24)]

x.XSendEvent.argtypes = [C.c_void_p, C.c_ulong, C.c_int, C.c_long, C.POINTER(XEvent)]
x.XFlush.argtypes = [C.c_void_p]
x.XCloseDisplay.argtypes = [C.c_void_p]
DISPLAY = x.XOpenDisplay(None)
if not DISPLAY:
    raise SystemExit("An X11/Xwayland display is required; no images generated")

def key(window, name):
    state = 4 if name.startswith("Ctrl+") else 0
    name = name.removeprefix("Ctrl+")
    code = x.XKeysymToKeycode(DISPLAY, x.XStringToKeysym(name.encode()))
    for kind, mask in [(2, 1), (3, 2)]:
        event = XEvent()
        event.key = KeyEvent(type=kind, send_event=1, display=DISPLAY, window=window,
                             state=state, keycode=code, same_screen=1)
        x.XSendEvent(DISPLAY, window, 1, mask, C.byref(event))
    x.XFlush(DISPLAY)
    time.sleep(.15)

def capture():
    title = f"TerraTopia Phase 2 capture {os.getpid()}"
    save_dir=tempfile.TemporaryDirectory(prefix="terratopia-capture-")
    proc = subprocess.Popen(["xterm", "-xrm", "*allowSendEvents:true",
        "-xrm", "*color4:#163456", "-xrm", "*color6:#36a6a6",
        "-geometry", "100x30", "-fa", "DejaVu Sans Mono", "-fs", "12",
        "-T", title, "-e", str(ROOT / "target/release/terratopia"), "--seed", "827193", "--save", str(Path(save_dir.name)/"capture.save")])
    try:
        for _ in range(40):
            tree = subprocess.check_output(["xwininfo", "-root", "-tree"], text=True)
            match = re.search(r'(0x[0-9a-f]+) "' + re.escape(title) + r'"', tree)
            if match:
                window = int(match[1], 16)
                info = subprocess.check_output(["xwininfo", "-id", hex(window), "-tree"], text=True)
                children = re.findall(r'^\s+(0x[0-9a-f]+) \(has no name\)', info, re.M)
                target = int(children[-1], 16) if children else window
                break
            if proc.poll() is not None:
                raise RuntimeError("Game exited before capture")
            time.sleep(.1)
        else:
            raise RuntimeError("xterm did not create a window")
        time.sleep(.3)
        key(target,"n"); key(target,"Return"); key(target,"e")
        for name in ["a","a","e","e","e","e","d","d","w","w","e","e","e","c","Return","BackSpace","d","d","s","s","s","s","e","e","c","Down"]:
            key(target,name)
        OUT.mkdir(parents=True,exist_ok=True)
        subprocess.run(["import","-window",hex(window),str(OUT / "crafting.png")],check=True)
        for name in ["Return","BackSpace","a","a","w","w","b","e","r"]:
            key(target,name)
        subprocess.run(["import","-window",hex(window),str(OUT / "survival.png")],check=True)
        key(target,"q")
        if proc.wait(timeout=5)!=0:
            raise RuntimeError("Game exited unsuccessfully")
        print("Captured real xterm crafting.png and survival.png")
    finally:
        if proc.poll() is None:
            proc.terminate(); proc.wait(timeout=5)
        save_dir.cleanup()

try:
    capture()
finally:
    x.XCloseDisplay(DISPLAY)
