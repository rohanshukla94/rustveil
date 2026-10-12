"""Compile, transform, optimize, and compare a Rust fixture. No third-party modules."""
import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess
import tempfile


def run(command, **kwargs):
    return subprocess.run([str(part) for part in command], check=True, text=True,
                          capture_output=True, **kwargs).stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tool", type=Path, required=True)
    parser.add_argument("--llvm-bin", type=Path, required=True)
    parser.add_argument("--toolchain", default="1.88.0")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="constants-", dir=args.output.resolve()))
    fixture = Path(__file__).resolve().parent / "fixtures"
    suffix = ".exe" if os.name == "nt" else ""
    tool = args.tool.resolve()
    llvm = args.llvm_bin.resolve()
    rustc = ["rustc", f"+{args.toolchain}"]
    version = run([*rustc, "--version", "--verbose"])
    if "LLVM version: 20." not in version:
        raise SystemExit("Use a Rust toolchain built with LLVM 20 (tested: 1.88.0)")
    opt = llvm / f"opt{suffix}"
    llc = llvm / f"llc{suffix}"
    if "version 20." not in run([opt, "--version"]):
        raise SystemExit("opt must be LLVM 20")

    original = output / "original.ll"
    run([*rustc, fixture / "kernel.rs", "--edition=2021", "--crate-type=lib",
         "--emit=llvm-ir", "-Copt-level=3", "-Cpanic=abort", "-o", original])
    encoded = output / "encoded.ll"
    selection = ["--function=mix", "--function=small", "--function=medium", "--function=signed"]
    options = [tool, original, *selection, "--seed=42"]
    report = json.loads(run([*options, "-o", encoded]))
    repeat = output / "repeat.ll"
    run([*options, "-o", repeat])
    if encoded.read_bytes() != repeat.read_bytes():
        raise AssertionError("same seed did not reproduce the IR")
    other = output / "other-seed.ll"
    run([tool, original, *selection, "--seed=43", "-o", other])
    if encoded.read_bytes() == other.read_bytes():
        raise AssertionError("different seeds produced identical IR")

    # Failed selections and existing outputs must leave no partial replacement.
    before = encoded.read_bytes()
    failures = [
        [*options, "-o", encoded],
        [tool, original, "--function=missing", "-o", output / "missing.ll"],
        [*options, "--function=mix", "-o", output / "duplicate.ll"],
        [tool, fixture / "attributes.ll", "--function=unsupported", "-o", output / "unsupported.ll"],
        [tool, fixture / "attributes.ll", "--function=callee", "--function=unsupported",
         "-o", output / "partial.ll"],
    ]
    for command in failures:
        if subprocess.run([str(part) for part in command], capture_output=True).returncode == 0:
            raise AssertionError("expected rejected invocation")
    rejected_outputs = ("missing.ll", "duplicate.ll", "unsupported.ll", "partial.ll")
    if encoded.read_bytes() != before or any((output / name).exists() for name in rejected_outputs):
        raise AssertionError("rejected invocation changed an output")

    attributes = output / "attributes.ll"
    run([tool, fixture / "attributes.ll", "--function=callee", "-o", attributes])
    text = attributes.read_text()
    if "memory(none)" in text or "speculatable" in text:
        raise AssertionError("stale memory attributes remain on a function or call site")
    run([opt, "-passes=verify", "-disable-output", attributes])

    binaries = {}
    for mode, source in (("plain", original), ("encoded", encoded)):
        optimized = output / f"{mode}.optimized.ll"
        run([opt, "-S", "-passes=default<O3>,verify", source, "-o", optimized])
        if mode == "encoded":
            loads = optimized.read_text().count("load volatile")
            if loads != report["encoded_constants"]:
                raise AssertionError(f"expected {report['encoded_constants']} volatile loads; got {loads}")
            report["loads_after_o3"] = loads
        obj = output / f"{mode}.obj"
        run([llc, "-O=3", "-filetype=obj", optimized, "-o", obj])
        run([llc, "-O=3", "-filetype=asm", optimized, "-o", output / f"{mode}.s"])
        binary = output / f"{mode}{suffix}"
        run([*rustc, fixture / "driver.rs", "--edition=2021", "-Copt-level=3",
             "-Cpanic=abort", "-C", f"link-arg={obj}", "-o", binary])
        binaries[mode] = binary
        report[f"{mode}_object_bytes"] = obj.stat().st_size
        report[f"{mode}_executable_bytes"] = binary.stat().st_size

    timings = {mode: [] for mode in binaries}
    checksums = set()
    # Alternate order to reduce systematic warm-up bias. Timings are descriptive.
    for sample in range(7):
        order = list(binaries) if sample % 2 == 0 else list(reversed(binaries))
        for mode in order:
            checksum, elapsed = run([binaries[mode]]).split()
            checksums.add(checksum)
            timings[mode].append(int(elapsed))
    if len(checksums) != 1:
        raise AssertionError("runtime checksums differ")
    report["samples_ns"] = timings
    report["median_runtime_ratio"] = statistics.median(timings["encoded"]) / statistics.median(timings["plain"])
    report["checksum"] = checksums.pop()
    report["rustc"] = version.strip()
    report["llvm"] = run([opt, "--version"]).strip()
    report["checks_per_run"] = {"u8_exhaustive": 256, "u16_exhaustive": 65536,
                               "i32_edges": 7, "u64_edges": 6,
                               "i32_generated": 100_000, "u64_generated": 100_000}
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    print(f"Artifacts: {output}")


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        raise SystemExit(f"Command failed: {error.cmd}\n{error.stdout}\n{error.stderr}") from error
