#!/usr/bin/env python3
"""Exercise the release binary in Unix PTYs. --full-night also waits for dawn.

Standard library only. Every run uses an isolated temporary save path.
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
import sys
import tempfile
import termios
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target/release/terratopia"


def smoke(exit_key, save, full_night=False, seed=827193):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    original = termios.tcgetattr(slave)

    def connect():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    process = subprocess.Popen(
        [str(BINARY), "--seed", str(seed), "--save", str(save)],
        stdin=slave, stdout=slave, stderr=slave, preexec_fn=connect,
        env={**os.environ, "TERM": "xterm-256color"})
    output = bytearray()

    def drain(duration=.1):
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
        assert not drain(.6), "unsolicited paused/menu output"
        if before is not None:
            delta = ticks() - before
            assert delta <= 1, f"{delta} CPU ticks in 600ms idle"
            print(f"  menu/paused CPU ticks: {delta}/600ms", flush=True)

    def resize(width, height):
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
        os.kill(process.pid, signal.SIGWINCH)
        return drain()

    try:
        assert b"T E R R A T O P I A" in drain(.3)
        idle()
        send(b"n")
        initial = send(b"\r")
        assert b"TERRATOPIA" in initial and b"HP 100" in initial and b"?" in initial
        match = re.search(rb" X (-?\d+)  Y (-?\d+)", initial)
        assert match
        x, y = map(int, match.groups())
        assert b"Cache: salvage pick" in send(b"e")
        scanned = send(b"r")
        assert b"LAST SCAN" in scanned and b"i Iron k Carbon v Crystal" in scanned
        assert scanned.count(b"?") < initial.count(b"?")
        assert b"Scan: +0 cells, +0 sites." in send(b"R")
        assert b"INVENTORY" in send(b"i")
        idle()
        assert b"Need 40x12" in resize(20, 6)
        assert b"INVENTORY" in resize(100, 30)
        send(b"\x7f")
        assert b"CRAFTING" in send(b"c")
        assert b"Missing ingredients" in send(b"\r")
        send(b"\x7f")
        assert b"SURVIVAL GUIDE" in send(b"h")
        idle()
        send(b"\x7f")
        send(b"aa")
        assert b"Harvested Iron" in send(b"eeee")
        send(b"ddww")
        assert b"Harvested Carbon" in send(b"eee")
        send(b"c")
        assert b"Crafted Mining tool" in send(b"\r")
        send(b"\x7f")
        send(b"ddssss")
        assert b"Harvested Crystal" in send(b"ee")
        send(b"c"); send(b"\x1b[B")
        assert b"Crafted Shelter kit" in send(b"\r")
        send(b"\x7f")
        send(b"aaww")
        assert b"Shelter built!" in send(b"b")
        assert b"Oxygen station powered" in send(b"e")
        assert b"Expedition saved." in send(b"\x1b[15~")
        saved = save.read_text()
        assert "TERRATOPIA_SAVE 2" in saved and "SHELTERS 1" in saved
        assert "HARVESTED 3" in saved
        assert f" X {x+1}  Y {y}".encode() in send(b"d")
        loaded = send(b"\x1b[20~")  # F9
        assert b"Expedition loaded." in loaded and b"PAUSED" in loaded
        idle()
        assert not send(b"r"), "scan must be inactive while paused"
        assert b"Need 40x12" in resize(20, 6)
        assert b"PAUSED" in resize(100, 30)
        send(b"p")
        if full_night:
            print("  Waiting in the constructed shelter for a real full night...", flush=True)
            start = len(output)
            until = time.monotonic() + 150
            while b"OBJECTIVE COMPLETE" not in output[start:]:
                assert process.poll() is None, "game exited before dawn"
                assert time.monotonic() < until, "dawn objective timed out"
                drain(2)
            assert b"Nightfall:" in output[start:]
            print("  PASS: real-time nightfall and dawn objective", flush=True)
        assert b"Expedition saved." in send(b"\x1b[15~")
        send(exit_key)
        process.wait(timeout=3)
        drain(.05)
        assert process.returncode == 0, output[-1000:]
        assert termios.tcgetattr(slave) == original, "terminal attributes not restored"
        assert b"\x1b[?1049l" in output and b"\x1b[?25h" in output
        assert b"panicked" not in output
        assert "SHELTERS 1" in save.read_text(), "quit must preserve expedition"
        print(f"PASS: crash/gather/craft/build, scan, menus, pause, resize, save/load, {exit_key!r}, restoration", flush=True)
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)


def save_error_smoke(folder):
    master, slave=pty.openpty()
    fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack("HHHH",24,100,0,0))
    original=termios.tcgetattr(slave)
    def connect():
        os.setsid()
        fcntl.ioctl(0,termios.TIOCSCTTY,0)
    save=Path(folder)/"missing-directory"/"expedition.save"
    process=subprocess.Popen([str(BINARY),"--seed","827193","--save",str(save)],
        stdin=slave,stdout=slave,stderr=slave,preexec_fn=connect,
        env={**os.environ,"TERM":"xterm-256color"})
    def drain(duration=.2):
        data=bytearray();until=time.monotonic()+duration
        while time.monotonic()<until:
            if select.select([master],[],[],max(0,until-time.monotonic()))[0]:
                data.extend(os.read(master,65536))
        return bytes(data)
    try:
        drain(.3);os.write(master,b"n\r");drain()
        os.write(master,b"q")
        assert b"Save failed:" in drain() and process.poll() is None
        os.write(master,b"\x11");drain();process.wait(timeout=3)
        assert process.returncode==0 and termios.tcgetattr(slave)==original
        assert not save.exists()
        result=subprocess.run([str(BINARY),"--save",str(save),"--load"],capture_output=True)
        assert result.returncode!=0 and b"TerraTopia:" in result.stderr
        print("PASS: save failure keeps game open, Ctrl+Q restores terminal, missing startup load fails cleanly",flush=True)
    finally:
        if process.poll() is None:
            process.kill();process.wait()
        os.close(master);os.close(slave)


def living_smoke(save, full_night=False):
    """Continue the same toxic-world expedition through the new exploration loop."""
    from collections import deque
    master,slave=pty.openpty()
    fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack("HHHH",24,100,0,0))
    original=termios.tcgetattr(slave)
    def connect():
        os.setsid();fcntl.ioctl(0,termios.TIOCSCTTY,0)
    process=subprocess.Popen([str(BINARY),"--load","--save",str(save)],stdin=slave,stdout=slave,stderr=slave,
        preexec_fn=connect,env={**os.environ,"TERM":"xterm-256color"})
    output=bytearray()
    header=save.read_text().splitlines()
    origin=tuple(map(int,next(line for line in header if line.startswith("PLAYER ")).split()[1:]))
    current=origin
    def drain(duration=.12):
        nonlocal current
        before=len(output);until=time.monotonic()+duration
        while time.monotonic()<until:
            if select.select([master],[],[],max(0,until-time.monotonic()))[0]:
                output.extend(os.read(master,65536))
        data=bytes(output[before:]);coords=re.findall(rb" X (-?\d+)  Y (-?\d+)",data)
        if coords: current=tuple(map(int,coords[-1]))
        assert b"LIFE SUPPORT FAILED" not in data, data[-1500:]
        return data
    def send(keys):
        os.write(master,keys);return drain()
    def valid(p):
        dx=p[0]-origin[0];dy=p[1]-origin[1]
        return abs(dx)<=4 and abs(dy)<=4 or 5<=dx<=16 and abs(dy)<=1
    def walk(dx,dy):
        target=(origin[0]+dx,origin[1]+dy);blocked=set();attempts=0
        while current!=target:
            attempts+=1;assert attempts<150,(current,target)
            queue=deque([(current,[])]);seen={current};path=None
            while queue:
                p,route=queue.popleft()
                if p==target:path=route;break
                for delta,key in [((1,0),b"d"),((-1,0),b"a"),((0,-1),b"w"),((0,1),b"s")]:
                    q=(p[0]+delta[0],p[1]+delta[1])
                    if q not in seen and q not in blocked and valid(q):seen.add(q);queue.append((q,route+[(q,key)]))
            if not path:
                drain(1);blocked.clear();continue
            q,key=path[0];before=current;data=send(key)
            if current==before:
                if b"Exhausted" in data:drain(1)
                else:blocked.add(q)
    try:
        assert b"PAUSED" in drain(.4)
        send(b"p");send(b"r")
        survey=send(b"m")
        assert b"PLANET SURVEY" in survey and b"Toxic jungle" in survey and b"grazer" in survey
        assert not drain(.6),"survey must pause the living simulation"
        send(b"\x7f")
        # Rest first to leave room for meaningful energy use on this excursion.
        send(b"z");drain(7);send(b"z")
        walk(0,-2);assert b"Harvested Carbon" in send(b"e")
        walk(5,0);assert b"Harvested Medicinal" in send(b"e")
        walk(7,1);assert b"Harvested Glow fungus" in send(b"e")
        send(b"c");send(b"\x1b[B"*5)
        assert b"Crafted Medical kit" in send(b"\r");send(b"\x7f")
        walk(7,-1);walk(12,-1);walk(12,0)
        assert b"Ruin salvage:" in send(b"e")
        assert b"Ruin already salvaged" in send(b"e")
        send(b"r");drain(2)
        assert b"turns hostile" in output,"exploration must encounter a hostile creature"
        walk(12,-1);walk(4,-1);walk(0,0)
        # Wait at the protected, powered shelter until an actual storm arrives.
        until=time.monotonic()+130
        while b"Weather: Toxic storm" not in output:
            assert time.monotonic()<until,"weather transition timed out"
            drain(2)
        assert b"STORM WARNING" in output
        send(b"\x1b[15~")
        saved=save.read_text()
        assert re.search(r"PLANTS [1-9]",saved) and re.search(r"RUINS [1-9]",saved)
        if full_night:
            print("  Waiting for dawn after the wildlife/flora/ruin/storm excursion...",flush=True)
            until=time.monotonic()+160
            while b"OBJECTIVE COMPLETE" not in output:
                assert time.monotonic()<until,"night objective timed out"
                drain(2)
        send(b"\x1b[15~")
        if full_night:
            camp_line=next(line for line in save.read_text().splitlines() if line.startswith("CAMP "))
            assert camp_line.split()[-1]=="1", "dawn success must persist"
        assert b"Expedition loaded." in send(b"l")
        send(b"p");send(b"r");assert b"PLANET SURVEY" in send(b"m");send(b"\x7f")
        send(b"q");process.wait(timeout=3)
        assert process.returncode==0 and termios.tcgetattr(slave)==original
        assert b"\x1b[?1049l" in output and b"\x1b[?25h" in output
        print("PASS: toxic planet, wildlife scan, flora, crafting, ruin loot, hostile escape, real forecast/storm, shelter, save/load",flush=True)
    finally:
        if process.poll() is None:process.kill();process.wait()
        os.close(master);os.close(slave)


if __name__ == "__main__":
    with tempfile.TemporaryDirectory(prefix="terratopia-pty-") as folder:
        for i,key in enumerate((b"q",b"\x1b",b"\x03")):
            smoke(key,Path(folder)/f"expedition-{i}.save",False,2 if i==0 else 827193)
        living_smoke(Path(folder)/"expedition-0.save","--full-night" in sys.argv)
        save_error_smoke(folder)
