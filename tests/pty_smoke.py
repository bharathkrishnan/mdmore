#!/usr/bin/env python3
"""Test the pager in a pseudoterminal using the Python standard library."""
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


class Session:
    def __init__(self, binary, directory, args, source=None, extra_env=None):
        self.master, self.slave = pty.openpty()
        self.resize(120, 18)
        env = dict(os.environ, TERM="xterm-256color")
        env.pop("NO_COLOR", None)
        env.update(extra_env or {})

        def setup():
            os.setsid()
            fcntl.ioctl(self.slave, termios.TIOCSCTTY, 0)

        # Keep a session leader alive until after mdmore exits. On macOS the
        # slave ceases to answer termios queries when the session leader exits.
        wrapper = """
import os, subprocess, sys, termios
tty = os.open('/dev/tty', os.O_RDONLY)
before = termios.tcgetattr(tty)
code = subprocess.call(sys.argv[1:])
after = termios.tcgetattr(tty)
if before != after:
    print('MDMORE_TTY_BROKEN', flush=True)
    sys.exit(99)
print('MDMORE_TTY_RESTORED', flush=True)
sys.exit(code)
"""
        self.process = subprocess.Popen(
            [sys.executable, "-c", wrapper, binary, *args], cwd=directory, env=env,
            stdin=subprocess.PIPE if source is not None else self.slave,
            stdout=self.slave, stderr=self.slave, preexec_fn=setup,
        )
        if source is not None:
            self.process.stdin.write(source)
            self.process.stdin.close()
        self.output = bytearray()

    def resize(self, columns, rows):
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0))
        if hasattr(self, "process"):
            os.killpg(self.process.pid, signal.SIGWINCH)

    def send(self, data):
        start = len(self.output)
        os.write(self.master, data)
        return start

    def wait_for(self, expected, since=0, timeout=8):
        deadline = time.monotonic() + timeout
        while expected not in self.output[since:]:
            plain = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", self.output[since:])
            if b"\x1b" not in expected and expected in plain:
                break
            if time.monotonic() > deadline:
                raise AssertionError(f"missing {expected!r}; output={bytes(self.output[since:])!r}")
            ready, _, _ = select.select([self.master], [], [], 0.05)
            if ready:
                self.output.extend(os.read(self.master, 65536))
        return bytes(self.output[since:])

    def finish(self, key=b"q"):
        self.send(key)
        deadline = time.monotonic() + 8
        while self.process.poll() is None:
            if time.monotonic() > deadline:
                raise AssertionError(f"pager did not exit; output={bytes(self.output[-2000:])!r}")
            ready, _, _ = select.select([self.master], [], [], 0.05)
            if ready:
                self.output.extend(os.read(self.master, 65536))
        assert self.process.returncode == 0
        self.wait_for(b"MDMORE_TTY_RESTORED")
        assert b"MDMORE_TTY_BROKEN" not in self.output

    def close(self):
        if self.process.poll() is None:
            os.killpg(self.process.pid, signal.SIGKILL)
        os.close(self.master)
        os.close(self.slave)
        self.process.wait(timeout=3)


def main():
    binary = str(Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/mdmore").resolve())
    source = "".join(
        f"# Section {i:04}\n\nA **styled** paragraph. Marker needle {i:04}.\n\n"
        for i in range(500)
    )
    with tempfile.TemporaryDirectory(prefix="mdmore-pty-") as directory:
        Path(directory, "fixture.md").write_text(source)
        Path(directory, "code.md").write_text("```rust\nfn main() { println!(\"hello\"); }\n```\n")

        session = Session(binary, directory, ["fixture.md"])
        try:
            initial = session.wait_for(b"q:quit")
            assert b"\x1b[?1049h" in initial and b"38;5;14" in initial
            assert b"Section 0000" in initial and b"Section 0499" not in initial
            start = session.send(b" ")
            page = session.wait_for(b"q:quit", start)
            assert b"Section 0000" not in page
            start = session.send(b"g")
            session.wait_for(b"Section 0000", start)
            start = session.send(b"/needle\r")
            session.wait_for(b"48;5;3", start)
            start = session.send(b"n")
            session.wait_for(b"needle 0001", start)
            start = session.send(b"N")
            session.wait_for(b"needle 0000", start)
            start = session.send(b"?")
            session.wait_for(b"Next page", start)
            start = session.send(b"x")
            session.wait_for(b"q:quit", start)
            start = len(session.output)
            session.resize(54, 12)
            session.wait_for(b"\x1b[12;1H", start)
            start = session.send(b"G")
            session.wait_for(b"END", start)
            start = session.send(b"/needle 0499\r")
            session.wait_for(b"48;5;3", start)
            start = session.send(b"n")
            session.wait_for(b"No further match", start)
            session.finish()
            print("PASS: paging, colors, search, help, resize, end, and terminal restoration")
        finally:
            session.close()

        unicode_source = "".join(
            f"## 節 {i:04}\n\n**東京世界 한국어 か\u3099** 👩🏽‍💻 e\u0301 {i:04}\n\n"
            for i in range(100)
        )
        Path(directory, "unicode.md").write_text(unicode_source)
        session = Session(binary, directory, ["unicode.md"])
        try:
            initial = session.wait_for(b"q:quit")
            for cluster in ["東京", "한국어", "か\u3099", "👩🏽‍💻", "e\u0301"]:
                assert cluster.encode() in initial
            start = session.send(b" ")
            session.wait_for("東京".encode(), start)
            start = session.send(b"/" + "東京".encode() + b"\r")
            session.wait_for(b"48;5;3", start)
            start = len(session.output)
            session.resize(24, 12)
            session.wait_for(b"\x1b[12;1H", start)
            start = session.send(b"n")
            session.wait_for(b"48;5;3", start)
            session.finish()
            print("PASS: Unicode clusters, paging, search, resize, and terminal restoration")
        finally:
            session.close()

        Path(directory, "large.md").write_text(source * 200)
        session = Session(binary, directory, ["large.md"])
        try:
            session.wait_for(b"q:quit")
            start = session.send(b"/missing-query\r\x1b")
            session.wait_for(b"Cancelled", start)
            session.finish()
            print("PASS: long searches can be cancelled between rendering batches")
        finally:
            session.close()

        session = Session(binary, directory, [], source=source.encode())
        try:
            session.wait_for(b"q:quit")
            start = session.send(b" ")
            session.wait_for(b"q:quit", start)
            session.finish(b"\x03")
            print("PASS: piped stdin uses the controlling terminal; Ctrl-C restores terminal")
        finally:
            session.close()

        session = Session(binary, directory, ["code.md"])
        try:
            output = session.wait_for(b"END")
            assert b"38;2;" in output, "missing syntax colors"
            session.finish()
            print("PASS: fenced code uses syntax colors in the pager")
        finally:
            session.close()

        session = Session(binary, directory, ["fixture.md"], extra_env={"NO_COLOR": "1"})
        try:
            output = session.wait_for(b"q:quit")
            assert b"38;" not in output and b"48;" not in output
            session.finish()
            print("PASS: NO_COLOR disables styling while retaining pagination")
        finally:
            session.close()

        session = Session(binary, directory, ["--color", "always", "fixture.md"], extra_env={"NO_COLOR": "1"})
        try:
            output = session.wait_for(b"q:quit")
            assert b"38;5;14" in output
            session.finish()
            print("PASS: --color always overrides NO_COLOR")
        finally:
            session.close()


if __name__ == "__main__":
    main()
