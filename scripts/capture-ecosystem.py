#!/usr/bin/env python3
"""Capture actual Phase 4 xterm gameplay using the existing X11 capture helper."""
from pathlib import Path
import subprocess
import tempfile
import importlib.util

spec = importlib.util.spec_from_file_location("capture", Path(__file__).with_name("capture-terminal.py"))
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)
try:
    with tempfile.TemporaryDirectory(prefix="terratopia-ecosystem-capture-") as folder:
        process, window, target = capture.launch(["--seed", "827193", "--save", str(Path(folder)/"capture.save")], "ecosystem")
        try:
            for key in ["n", "Return", "r"]:
                capture.key(target, key)
            subprocess.run(["import", "-window", hex(window), str(capture.OUT/"ecosystem.png")], check=True, timeout=10)
            capture.key(target, "m")
            subprocess.run(["import", "-window", hex(window), str(capture.OUT/"ecosystem-inspection.png")], check=True, timeout=10)
            capture.key(target, "q")
            assert process.wait(timeout=5) == 0
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)
finally:
    capture.x.XCloseDisplay(capture.DISPLAY)
