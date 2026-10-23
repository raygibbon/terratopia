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

def launch(arguments, suffix):
    title=f"TerraTopia Phase 3 capture {os.getpid()} {suffix}"
    prior_focus=C.c_ulong();prior_revert=C.c_int()
    x.XGetInputFocus.argtypes=[C.c_void_p,C.POINTER(C.c_ulong),C.POINTER(C.c_int)]
    x.XSetInputFocus.argtypes=[C.c_void_p,C.c_ulong,C.c_int,C.c_ulong]
    x.XGetInputFocus(DISPLAY,C.byref(prior_focus),C.byref(prior_revert))
    process=subprocess.Popen(["xterm","-xrm","*allowSendEvents:true","-xrm","*input:false",
        "-xrm","*color4:#163456","-xrm","*color6:#36a6a6","-geometry","100x30",
        "-fa","DejaVu Sans Mono","-fs","12","-T",title,"-e",str(ROOT/"target/release/terratopia"),*arguments])
    try:
        for _ in range(40):
            tree=subprocess.check_output(["xwininfo","-root","-tree"],text=True)
            match=re.search(r'(0x[0-9a-f]+) "'+re.escape(title)+r'"',tree)
            if match:
                window=int(match[1],16)
                info=subprocess.check_output(["xwininfo","-id",hex(window),"-tree"],text=True)
                children=re.findall(r'^\s+(0x[0-9a-f]+) \(has no name\)',info,re.M)
                target=int(children[-1],16) if children else window
                x.XSetInputFocus(DISPLAY,prior_focus.value,prior_revert.value,0);x.XFlush(DISPLAY)
                time.sleep(.3)
                return process,window,target
            if process.poll() is not None:raise RuntimeError("Game exited before capture")
            time.sleep(.1)
        raise RuntimeError("xterm did not create a window")
    except BaseException:
        if process.poll() is None:process.terminate();process.wait(timeout=5)
        raise


def weather_in_pty(save):
    # Advance the actual game in real time while leaving desktop focus alone.
    import fcntl,pty,select,struct,termios
    master,slave=pty.openpty()
    fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack("HHHH",30,100,0,0))
    def connect():
        os.setsid();fcntl.ioctl(0,termios.TIOCSCTTY,0)
    process=subprocess.Popen([str(ROOT/"target/release/terratopia"),"--load","--save",str(save)],
        stdin=slave,stdout=slave,stderr=slave,preexec_fn=connect,env={**os.environ,"TERM":"xterm-256color"})
    def drain(duration):
        result=bytearray();until=time.monotonic()+duration
        while time.monotonic()<until:
            if select.select([master],[],[],max(0,until-time.monotonic()))[0]:result.extend(os.read(master,65536))
        return bytes(result)
    try:
        drain(.3);os.write(master,b"p")
        print("Waiting for actual regional weather in a PTY...",flush=True)
        observed=drain(41)
        assert b"Toxic storm" in observed, "no actual storm transition observed"
        os.write(master,b"q");drain(.2);process.wait(timeout=5)
        assert process.returncode==0
    finally:
        if process.poll() is None:process.kill();process.wait()
        os.close(master);os.close(slave)


def capture():
    OUT.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="terratopia-capture-") as folder:
        save=Path(folder)/"capture.save"
        process,window,target=launch(["--seed","2","--save",str(save)],"initial")
        try:
            key(target,"n");key(target,"Return");key(target,"e")
            for name in ["a","a","e","e","e","e","d","d","w","w","e","e","e","c","Return","BackSpace","d","d","s","s","s","s","e","e","c","Down"]:key(target,name)
            subprocess.run(["import","-window",hex(window),str(OUT/"living-crafting.png")],check=True,timeout=10)
            for name in ["Return","BackSpace","a","a","w","w","b","e","r"]:key(target,name)
            subprocess.run(["import","-window",hex(window),str(OUT/"living.png")],check=True,timeout=10)
            key(target,"m")
            subprocess.run(["import","-window",hex(window),str(OUT/"planet-survey.png")],check=True,timeout=10)
            key(target,"BackSpace");key(target,"q")
            assert process.wait(timeout=5)==0
        finally:
            if process.poll() is None:process.terminate();process.wait(timeout=5)
        weather_in_pty(save)
        process,window,target=launch(["--load","--save",str(save)],"storm")
        try:
            key(target,"p");key(target,"r")
            subprocess.run(["import","-window",hex(window),str(OUT/"storm.png")],check=True,timeout=10)
            key(target,"q");assert process.wait(timeout=5)==0
        finally:
            if process.poll() is None:process.terminate();process.wait(timeout=5)
    print("Captured real Phase 3 xterm gameplay, survey, crafting and storm",flush=True)

try:
    capture()
finally:
    x.XCloseDisplay(DISPLAY)
