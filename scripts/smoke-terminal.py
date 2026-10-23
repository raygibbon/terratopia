#!/usr/bin/env python3
"""Unix PTY checks following Terrabox's scripts/smoke-examples.py approach.

Run after cargo build --release. Uses only the Python standard library.
This checks real terminal bytes and termios, not emulator font/colour appearance.
"""
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import subprocess
import termios
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target/release/terratopia"


def smoke(exit_key):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    original = termios.tcgetattr(slave)

    def connect():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    process = subprocess.Popen(
        [str(BINARY), "--seed", "827193"], stdin=slave, stdout=slave,
        stderr=slave, preexec_fn=connect,
        env={**os.environ, "TERM": "xterm-256color"})
    output = bytearray()

    def drain(duration=.15):
        before = len(output)
        until = time.monotonic() + duration
        while time.monotonic() < until:
            if select.select([master], [], [], max(0, until - time.monotonic()))[0]:
                output.extend(os.read(master, 65536))
        return bytes(output[before:])

    def send(keys):
        os.write(master, keys)
        return drain()

    def idle():
        stat = Path(f"/proc/{process.pid}/stat")
        def ticks():
            fields = stat.read_text().split()
            return int(fields[13]) + int(fields[14])
        before = ticks() if stat.exists() else None
        assert not drain(.6), "unsolicited idle output"
        if before is not None:
            delta = ticks() - before
            assert delta <= 1, f"{delta} CPU ticks during 600ms idle"
            print(f"  idle CPU ticks: {delta}/600ms")

    def resize(width, height):
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
        os.kill(process.pid, signal.SIGWINCH)
        return drain()

    try:
        assert b"T E R R A T O P I A" in drain(.3)
        idle()
        assert b"Seed 827193" in send(b"n")
        initial = send(b"\r")
        assert b"TERRATOPIA" in initial and b"HP 100" in initial
        match = re.search(rb" X (-?\d+)  Y (-?\d+)", initial)
        assert match, initial[-1000:]
        x, y = map(int, match.groups())
        assert f" X {x+1}  Y {y}".encode() in send(b"d")
        assert f" X {x}  Y {y}".encode() in send(b"\x1b[D")
        idle()  # Playing has a sleeping clock but no unchanged presentations.
        assert b"PAUSED" in send(b"p")
        idle()
        assert not send(b"w"), "movement while paused must do nothing"
        assert b"Need 40x12" in resize(20, 6)
        assert b"PAUSED" in resize(100, 30)
        assert b"PAUSED" not in send(b"p")
        assert f" X {x}  Y {y+1}".encode() in send(b"\x1b[B")
        send(exit_key)
        process.wait(timeout=3)
        drain(.05)
        assert process.returncode == 0, output[-1000:]
        assert termios.tcgetattr(slave) == original, "terminal attributes not restored"
        assert b"\x1b[?1049l" in output, "alternate screen not left"
        assert b"\x1b[?25h" in output, "cursor not restored"
        assert b"panicked" not in output
        print(f"PASS: menu, landing, WASD/arrows, pause, resize, {exit_key!r} exit, restoration")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)


for key in (b"q", b"\x1b", b"\x03"):
    smoke(key)
