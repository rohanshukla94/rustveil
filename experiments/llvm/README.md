# LLVM constant reconstruction experiment

This standalone tool rewrites selected integer operands in LLVM IR. It runs
after Rust has resolved names and emitted optimized IR. It is separate from
the Cargo workspace and is not part of the crates.io packages.

For a constant `C`, the tool emits a private mutable global initialized to `K`
and replaces the operand with `volatile_load(K) ^ (C ^ K)`. The volatile load
prevents LLVM from folding the expression back into a literal. A deterministic
seed chooses the masks; it is not a secret.

Both pieces of each constant remain in the binary. A reverse engineer can
recover them. This experiment establishes a compiler transformation and a way
to measure it, not resistance to deobfuscation.

## Build

Requirements: LLVM **20** development libraries and tools, CMake 3.20+, a C++17
compiler, Python 3.9+, and Rust 1.88.0. The Rust compiler and external LLVM tools
must use the same LLVM major version. Rust 1.95's LLVM 22 is not supported here.

Tested on Windows x64 with MSVC 19.44, LLVM 20.1.8, and Rust 1.88.0 (LLVM 20.1.5).
An isolated conda-forge environment can supply the LLVM tools:

```powershell
micromamba create --yes --prefix ./target/llvm20 --channel conda-forge `
  llvmdev=20.1.8 cmake ninja zlib libxml2-devel

$llvmPrefix = Join-Path (Get-Location) 'target/llvm20/Library'
$env:PATH = "$llvmPrefix/bin;$env:PATH"
cmake -S experiments/llvm -B target/llvm-build `
  -G 'Visual Studio 17 2022' -A x64 "-DCMAKE_PREFIX_PATH=$llvmPrefix"
cmake --build target/llvm-build --config Release
```

## Run the checks

```powershell
python experiments/llvm/check.py `
  --tool target/llvm-build/Release/rustveil-llvm.exe `
  --llvm-bin target/llvm20/Library/bin `
  --output target/llvm-checks
```

Each run gets its own directory containing original, transformed, and
reoptimized IR, object files, assembly, executables, and a JSON report.
The harness checks:

- LLVM verification before and after rewriting, then another `-O3` pipeline.
- Retention of every generated volatile load after optimization.
- All `u8` and `u16` inputs, signed/unsigned boundaries, and 100,000 generated
  inputs each for `i32` and `u64`, against independent Rust reference functions.
- Repeatability, seed variation, stale memory-attribute removal, and rejection
  of missing, duplicate, or unsupported selections without partial output.
- Seven alternating runtime samples, checksums, and object/executable sizes.

Timing covers two million calls to the `u64` fixture, not the whole test suite.
It is a small local measurement, not an application benchmark. The checked-in
[Windows result](results/windows-x64.json) records one run; rerun on your target.

To transform your own compatible IR:

```text
rustveil-llvm input.ll --function=exact_symbol --seed=42 -o output.ll
```

Repeat `--function` for additional definitions. Every selected function must
contain an eligible operand. Existing output files are refused.

## Scope

Only scalar `i8`, `i16`, `i32`, and `i64` constants used by binary instructions
and integer comparisons are changed. Calls, return values, switch cases, PHI
nodes, vectors, floating-point values, and other widths are left alone. This
is not whole-program constant coverage or a Cargo build integration.

The pass removes inferred memory and speculation attributes throughout the
module, including call sites, because the new volatile loads invalidate them.
This is conservative and may reduce optimization outside selected functions.
The verifier catches malformed IR; it does not prove semantic equivalence.

Next work should measure real functions, narrow attribute invalidation, and
evaluate how easily a deobfuscator recovers the constants. Control-flow passes
and runtime integration are separate work.

LLVM references: [IR language reference](https://releases.llvm.org/20.1.0/docs/LangRef.html),
[LLVM programmer's manual](https://releases.llvm.org/20.1.0/docs/ProgrammersManual.html).
