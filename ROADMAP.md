# Engineering assessment and roadmap

## Current protection

RustVeil has a useful foundation: a small shared AST transformer, explicit
feature activation, deterministic builds, safe runtime decoding, and staged
project copying. Keeping those components inspectable suits an open-source tool.

Local renaming obscures distributed source. Optimized binaries already discard
most local names, so renaming is not a meaningful binary protection measure.
The string macro emits reversible encoded bytes. Its seed, ciphertext, and
decoder ship together; static analysis or runtime observation can recover the
strings. Decoded strings remain cached for the process lifetime. No client-side
secret protection is claimed. Artifact checks measure plaintext visibility for
one known marker on the tested toolchain, not general resistance to analysis.

The syntax-only engine cannot resolve bare constant/variant patterns. Reviewed
mode remains a caller assertion, even when transformed code compiles. Public
APIs, exported names, fields, and serialization contracts are not renamed.

## First release improvements

- Support destructuring and match arms without losing binding scope or field names.
- Reject insufficient coverage through opt-in build checks; report actual work.
- Compile and execute original/transformed fixtures in both binding modes.
- Check feature passthrough, compiler errors, minimum Rust support, and release
  binary string visibility. Publish all four crates in dependency order.

## Next milestones, in order

1. **Correctness corpus and measurements.** Add generated programs and fuzzing
   with a semantic oracle, drop-order/borrow/async fixtures, and real application
   compatibility samples. Measure transformation coverage, build time, binary
   growth, cold string decoding, and steady-state overhead against baselines.
2. **Resolve names before broadening transforms.** Design a compiler-backed
   layer with stable symbol identities and an explicit toolchain policy. That
   removes the reviewed-mode constant/variant ambiguity and enables safe work
   across modules. Do not guess symbol identity from identifier spelling.
3. **Experimental binary passes.** Evaluate narrowly scoped control-flow and
   constant/data transformations in compiler IR, with differential execution,
   optimization-survival checks, performance budgets, and decompiler review.
   Keep this backend optional; do not present source renaming as its substitute.
4. **Release evidence.** Publish compatibility/performance results and artifact
   inspection results for each supported toolchain and target. Define repeatable
   reverse-engineering exercises before making comparative strength claims.

Or-patterns, conditional let chains, and macro-expanded code require explicit
scope/expansion handling before they can enter the supported syntax set.
Source comment/license preservation also needs a token-preserving rewrite mode;
the current CLI regenerates selected changed files with prettyplease.

## Release standard

Changes must preserve behavior on supported inputs, fail or visibly skip
unsupported inputs, and keep build outputs reproducible for a given version,
seed, toolchain, and input. Stronger protection must have measured benefit and
an explicit cost. No "unbreakable", encryption, or secret-storage claims.
