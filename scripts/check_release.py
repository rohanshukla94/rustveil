"""Check release behavior and plaintext visibility for the current toolchain.

This regression check is not a security guarantee. Uses Python's standard library.
"""
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
MARKER = b"RustVeil workspace demo"
EXPECTED = "RustVeil workspace demo \u2014 ready: 1710\n".encode()


def main():
    for enabled in (False, True):
        mode = "encoded" if enabled else "plain"
        target = ROOT / "target" / "release-check" / mode
        command = [
            os.environ.get("CARGO", "cargo"), "build", "--locked", "--release",
            "-p", "workspace-app", "--target-dir", str(target),
        ]
        if enabled:
            command += ["--features", "obfuscation"]
        subprocess.run(command, cwd=ROOT, check=True)
        binary = target / "release" / ("workspace-app.exe" if os.name == "nt" else "workspace-app")
        output = subprocess.run([str(binary)], capture_output=True, check=True).stdout
        if output.replace(b"\r\n", b"\n") != EXPECTED:
            raise SystemExit(f"{mode}: unexpected runtime output: {output!r}")
        artifact = binary.read_bytes()
        visible = MARKER in artifact
        if visible == enabled:
            raise SystemExit(f"{mode}: plaintext marker presence was {visible}")
        print(f"{mode}: output matches; plaintext marker present={visible}; {len(artifact)} bytes", flush=True)


if __name__ == "__main__":
    main()
