use anyhow::{bail, Context, Result};
use clap::Parser;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use syn::visit_mut::{self, VisitMut};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(
    version,
    about = "Conservative Rust source obfuscation into a separate project copy"
)]
struct Args {
    /// Existing project/workspace root. Never modified.
    #[arg(long)]
    input: PathBuf,
    /// New sibling directory; must not exist or be inside the input.
    #[arg(long)]
    output: PathBuf,
    /// Deterministic name seed (not a secret).
    #[arg(long, default_value_t = 0)]
    seed: u64,
    /// Root-relative Rust file glob. Repeat to select several source trees.
    #[arg(long = "select", default_value = "**/*.rs")]
    select: Vec<String>,
    /// Exclude files or directories from the copy. Repeat as needed.
    #[arg(long)]
    exclude: Vec<String>,
    /// Inspect and report without creating the output directory.
    #[arg(long)]
    dry_run: bool,
    /// Assert bare identifier patterns are bindings; qualify constants and unit variants.
    /// Use only on reviewed source selections.
    #[arg(long)]
    assume_bindings: bool,
    /// Reject the run if any selected function/module/file is skipped.
    #[arg(long)]
    fail_on_skip: bool,
    /// Require at least this many renamed bindings before publishing the copy.
    #[arg(long, default_value_t = 0)]
    min_renamed: usize,
}

#[derive(Serialize)]
struct FunctionReport {
    name: String,
    renamed: usize,
    skipped: Option<String>,
}
#[derive(Serialize)]
struct FileReport {
    path: String,
    functions: Vec<FunctionReport>,
}
#[derive(Serialize)]
struct Report {
    format_version: u8,
    assume_bindings: bool,
    seed: u64,
    selected_files: usize,
    transformed_files: usize,
    skipped_entries: usize,
    renamed: usize,
    files: Vec<FileReport>,
}

struct Transformer {
    seed: u64,
    assume_bindings: bool,
    functions: Vec<FunctionReport>,
}
impl Transformer {
    fn function(&mut self, attrs: &[syn::Attribute], sig: &syn::Signature, body: &mut syn::Block) {
        let outcome = if attrs.is_empty() {
            rustveil_core::transform_with_options(sig, body, self.seed, self.assume_bindings)
        } else {
            rustveil_core::Outcome {
                renamed: 0,
                skipped: Some("function attributes require expansion/review".into()),
            }
        };
        self.functions.push(FunctionReport {
            name: sig.ident.to_string(),
            renamed: outcome.renamed,
            skipped: outcome.skipped,
        });
    }
}
impl VisitMut for Transformer {
    fn visit_item_fn_mut(&mut self, f: &mut syn::ItemFn) {
        self.function(&f.attrs, &f.sig, &mut f.block);
    }
    fn visit_impl_item_fn_mut(&mut self, f: &mut syn::ImplItemFn) {
        self.function(&f.attrs, &f.sig, &mut f.block);
    }
    fn visit_trait_item_fn_mut(&mut self, f: &mut syn::TraitItemFn) {
        if let Some(body) = &mut f.default {
            self.function(&f.attrs, &f.sig, body);
        }
    }
    fn visit_item_impl_mut(&mut self, i: &mut syn::ItemImpl) {
        if i.attrs.is_empty() {
            visit_mut::visit_item_impl_mut(self, i);
        } else {
            self.functions.push(FunctionReport {
                name: "<impl>".into(),
                renamed: 0,
                skipped: Some("impl attributes require review".into()),
            });
        }
    }
    fn visit_item_trait_mut(&mut self, i: &mut syn::ItemTrait) {
        if i.attrs.is_empty() {
            visit_mut::visit_item_trait_mut(self, i);
        } else {
            self.functions.push(FunctionReport {
                name: i.ident.to_string(),
                renamed: 0,
                skipped: Some("trait attributes require review".into()),
            });
        }
    }
    fn visit_item_mod_mut(&mut self, i: &mut syn::ItemMod) {
        if i.attrs.is_empty() {
            visit_mut::visit_item_mod_mut(self, i);
        } else {
            self.functions.push(FunctionReport {
                name: i.ident.to_string(),
                renamed: 0,
                skipped: Some("module attributes require review".into()),
            });
        }
    }
}

fn globs(patterns: &[String]) -> Result<GlobSet> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        b.add(Glob::new(p).with_context(|| format!("invalid glob {p}"))?);
    }
    Ok(b.build()?)
}
fn excluded(path: &Path, relative: &Path, extra: &GlobSet) -> bool {
    let default = path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
        matches!(
            n,
            ".git" | "target" | "node_modules" | ".rustveil-report.json"
        )
    });
    default || extra.is_match(relative)
}

fn run(args: Args) -> Result<Report> {
    let input = args.input.canonicalize().context("resolve --input")?;
    if !input.is_dir() {
        bail!("--input must be a directory");
    }
    // Resolve the parent so symlink aliases cannot defeat the containment check.
    let parent = args
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent
        .canonicalize()
        .context("output parent must already exist")?;
    let name = args
        .output
        .file_name()
        .context("--output needs a new directory name")?;
    let output = parent.join(name);
    if output.starts_with(&input) {
        bail!("--output must be outside --input");
    }
    if fs::symlink_metadata(&output).is_ok() {
        bail!("--output already exists; refusing to overwrite it");
    }
    let selected = globs(&args.select)?;
    let extra = globs(&args.exclude)?;
    // A failed parse/copy never publishes a partial output tree.
    let staging = if args.dry_run {
        None
    } else {
        Some(
            tempfile::Builder::new()
                .prefix(".rustveil-stage-")
                .tempdir_in(&parent)?,
        )
    };
    let mut report = Report {
        format_version: 2,
        assume_bindings: args.assume_bindings,
        seed: args.seed,
        selected_files: 0,
        transformed_files: 0,
        skipped_entries: 0,
        renamed: 0,
        files: Vec::new(),
    };
    let entries = WalkDir::new(&input)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !excluded(
                    entry.path(),
                    entry.path().strip_prefix(&input).unwrap(),
                    &extra,
                )
        });
    for entry in entries {
        let entry = entry?;
        if entry.depth() == 0 {
            continue;
        }
        let relative = entry.path().strip_prefix(&input)?;
        let destination = staging.as_ref().map(|d| d.path().join(relative));
        if entry.file_type().is_symlink() {
            bail!(
                "symlink not copied: {} (exclude or materialize it first)",
                relative.display()
            );
        }
        if entry.file_type().is_dir() {
            if let Some(dst) = destination {
                fs::create_dir_all(dst)?;
            }
            continue;
        }
        if !entry.file_type().is_file() {
            bail!("unsupported file type: {}", relative.display());
        }
        let process =
            relative.extension().is_some_and(|e| e == "rs") && selected.is_match(relative);
        let mut replacement = None;
        if process {
            let source = fs::read_to_string(entry.path())
                .with_context(|| format!("read {}", relative.display()))?;
            let mut ast = syn::parse_file(&source)
                .with_context(|| format!("parse {}", relative.display()))?;
            let mut t = Transformer {
                seed: rustveil_core::hash(
                    relative.to_string_lossy().replace('\\', "/").as_bytes(),
                    args.seed,
                ),
                assume_bindings: args.assume_bindings,
                functions: Vec::new(),
            };
            // Crate-level procedural attributes can inspect the entire file.
            // Conservatively skip all attributes, including harmless lints/docs.
            if ast.attrs.is_empty() {
                t.visit_file_mut(&mut ast);
            } else {
                t.functions.push(FunctionReport {
                    name: "<file>".into(),
                    renamed: 0,
                    skipped: Some("file-level attributes require review".into()),
                });
            }
            let count: usize = t.functions.iter().map(|f| f.renamed).sum();
            if count > 0 {
                replacement = Some(prettyplease::unparse(&ast));
                report.transformed_files += 1;
            }
            report.skipped_entries += t.functions.iter().filter(|f| f.skipped.is_some()).count();
            report.renamed += count;
            report.selected_files += 1;
            report.files.push(FileReport {
                path: relative.to_string_lossy().replace('\\', "/"),
                functions: t.functions,
            });
        }
        if let Some(dst) = destination {
            if let Some(source) = replacement {
                fs::write(&dst, source)?;
                fs::set_permissions(&dst, entry.metadata()?.permissions())?;
            } else {
                fs::copy(entry.path(), &dst)?;
            }
        }
    }
    // Apply the same coverage policy to inspection and copying. A rejected copy
    // is dropped with its staging directory, before any output is published.
    if args.fail_on_skip && report.skipped_entries > 0 {
        let details = report
            .files
            .iter()
            .flat_map(|file| {
                file.functions.iter().filter_map(|function| {
                    function
                        .skipped
                        .as_ref()
                        .map(|reason| format!("{}::{}: {reason}", file.path, function.name))
                })
            })
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "coverage check failed: {} skipped entries\n{details}",
            report.skipped_entries
        );
    }
    if report.renamed < args.min_renamed {
        bail!("coverage check failed: renamed {} bindings; --min-renamed requires {} ({} selected files)", report.renamed, args.min_renamed, report.selected_files);
    }
    if let Some(staging) = staging {
        fs::write(
            staging.path().join(".rustveil-report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        // Recheck before publishing. Do not run concurrently against the same output.
        if fs::symlink_metadata(&output).is_ok() {
            bail!("output appeared during processing; refusing to publish");
        }
        fs::rename(staging.path(), &output).context("publish completed project copy")?;
    }
    Ok(report)
}
fn main() -> Result<()> {
    let args = Args::parse();
    let report = run(args)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(root: &Path, output: &str) -> Args {
        Args {
            input: root.join("input"),
            output: root.join(output),
            seed: 7,
            select: vec!["**/*.rs".into()],
            exclude: vec![],
            dry_run: false,
            assume_bindings: false,
            fail_on_skip: false,
            min_renamed: 0,
        }
    }
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("input")).unwrap();
        dir
    }
    fn execute(path: &Path, executable: &Path) -> String {
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let output = std::process::Command::new(rustc)
            .arg("--edition=2021")
            .arg(path)
            .arg("-o")
            .arg(executable)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = std::process::Command::new(executable).output().unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    }
    #[test]
    fn transformed_project_compiles_and_matches_original() {
        let dir = fixture();
        let root = dir.path();
        let source = r#"
struct Point { x: i32, y: i32 }
fn evaluate() -> i32 {
    let mut x = 2;
    x += 1;
    let x = x + 2;
    let f = |x: i32| { let y = x + 1; y * 2 };
    let mut sum = 0;
    for x in 1..4 { sum += f(x); }
    let p = Point { x, y: sum };
    p.x + p.y
}
fn main() { println!("{}", evaluate()); }
"#;
        fs::write(root.join("input/main.rs"), source).unwrap();
        fs::write(root.join("input/data.bin"), [0, 255, 13]).unwrap();
        for (mode, name) in [(false, "safe"), (true, "reviewed")] {
            let mut options = args(root, name);
            options.assume_bindings = mode;
            let report = run(options).unwrap();
            assert!(report.renamed >= 3);
            let output = root.join(name);
            let after = fs::read_to_string(output.join("main.rs")).unwrap();
            assert!(after.contains("rv_"));
            assert_eq!(fs::read(output.join("data.bin")).unwrap(), [0, 255, 13]);
            assert_eq!(
                execute(
                    &root.join("input/main.rs"),
                    &root.join(format!("original-{name}{}", std::env::consts::EXE_SUFFIX))
                ),
                execute(
                    &output.join("main.rs"),
                    &root.join(format!("output-{name}{}", std::env::consts::EXE_SUFFIX))
                )
            );
        }
        assert_eq!(
            fs::read_to_string(root.join("input/main.rs")).unwrap(),
            source
        );
    }
    #[test]
    fn dry_run_has_no_output_and_selection_is_respected() {
        let dir = fixture();
        let root = dir.path();
        let source = "fn f() -> i32 { let mut x = 1; x += 1; x }";
        fs::write(root.join("input/chosen.rs"), source).unwrap();
        fs::write(root.join("input/other.rs"), source).unwrap();
        let mut options = args(root, "dry");
        options.dry_run = true;
        options.select = vec!["chosen.rs".into()];
        let report = run(options).unwrap();
        assert_eq!(report.selected_files, 1);
        assert!(!root.join("dry").exists());
        let mut options = args(root, "copy");
        options.select = vec!["chosen.rs".into()];
        run(options).unwrap();
        assert_eq!(
            fs::read_to_string(root.join("copy/other.rs")).unwrap(),
            source
        );
    }
    #[test]
    fn rejected_paths_and_parse_failure_do_not_publish() {
        let dir = fixture();
        let root = dir.path();
        fs::write(root.join("input/a.rs"), "fn f() {}").unwrap();
        assert!(run(args(root, "input/inside")).is_err());
        fs::create_dir(root.join("existing")).unwrap();
        fs::write(root.join("existing/keep"), "untouched").unwrap();
        assert!(run(args(root, "existing")).is_err());
        assert_eq!(
            fs::read_to_string(root.join("existing/keep")).unwrap(),
            "untouched"
        );
        fs::write(root.join("input/z.rs"), "fn broken(").unwrap();
        assert!(run(args(root, "failed")).is_err());
        assert!(!root.join("failed").exists());
        assert!(!fs::read_dir(root).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".rustveil-stage-")));
    }
    #[test]
    fn default_exclusions_and_repeatability() {
        let dir = fixture();
        let root = dir.path();
        fs::write(root.join("input/a.rs"), "fn f() { let mut x = 1; x += 1; }").unwrap();
        fs::create_dir(root.join("input/target")).unwrap();
        fs::write(root.join("input/target/invalid.rs"), "invalid(").unwrap();
        run(args(root, "one")).unwrap();
        run(args(root, "two")).unwrap();
        assert!(!root.join("one/target").exists());
        assert_eq!(
            fs::read(root.join("one/a.rs")).unwrap(),
            fs::read(root.join("two/a.rs")).unwrap()
        );
    }
    #[test]
    fn coverage_checks_reject_before_publishing_in_both_modes() {
        let dir = fixture();
        let root = dir.path();
        fs::write(
            root.join("input/a.rs"),
            "fn f() { let mut x = 1; x += 1; } fn skipped() { println!(\"x\"); }",
        )
        .unwrap();
        for dry_run in [false, true] {
            let mut options = args(root, "rejected");
            options.dry_run = dry_run;
            options.fail_on_skip = true;
            let error = run(options).err().unwrap().to_string();
            assert!(error.contains("a.rs::skipped"), "{error}");
            assert!(!root.join("rejected").exists());

            let mut options = args(root, "insufficient");
            options.dry_run = dry_run;
            options.min_renamed = 2;
            assert!(run(options)
                .err()
                .unwrap()
                .to_string()
                .contains("renamed 1 bindings"));
            assert!(!root.join("insufficient").exists());

            let mut options = args(root, "empty-selection");
            options.dry_run = dry_run;
            options.min_renamed = 1;
            options.select = vec!["missing/**/*.rs".into()];
            assert!(run(options)
                .err()
                .unwrap()
                .to_string()
                .contains("0 selected files"));
            assert!(!root.join("empty-selection").exists());
        }
        let mut options = args(root, "accepted");
        options.min_renamed = 1;
        let report = run(options).unwrap();
        assert_eq!(report.format_version, 2);
        assert_eq!(report.selected_files, 1);
        assert_eq!(report.transformed_files, 1);
        assert_eq!(report.skipped_entries, 1);
        assert!(!fs::read_dir(root).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".rustveil-stage-")));
    }
    #[test]
    fn patterns_and_match_scopes_preserve_runtime_behavior() {
        let dir = fixture();
        let root = dir.path();
        let source = r#"
struct Point { x: i32, y: i32 }
const LIMIT: i32 = 99;
fn evaluate(input: Option<i32>) -> i32 {
    let mut x = 20;
    x += 1;
    let (mut a, [b, c]) = (2, [3, 4]);
    a += b + c;
    let Point { x: mut px, ref y } = Point { x: a, y: x };
    px += *y;
    let f = |(x, y): (i32, i32)| x + y;
    let mut sum = 0;
    for (x, y) in [(1, 2), (3, 4)] { sum += f((x, y)); }
    let matched = match input {
        Option::Some(x) if x > 10 => x + 100,
        Option::Some(x @ 0..=10) => x + 200,
        Option::Some(ref x) => *x - 100,
        Option::None => x,
    };
    let [first, middle @ .., last] = &[1, 2, 3, 4];
    let &copy = first;
    let fallback = x;
    let Option::Some(x) = input else { return fallback + matched; };
    let ranged = match x { crate::LIMIT => 7, -5..=0 => 8, _ => 9 };
    px + sum + matched + copy + middle.len() as i32 + last + x + ranged
}
fn main() {
    for value in [Option::None, Some(-9), Some(-1), Some(0), Some(4), Some(12), Some(99)] {
        println!("{}", evaluate(value));
    }
}
"#;
        fs::write(root.join("input/main.rs"), source).unwrap();
        let original = execute(
            &root.join("input/main.rs"),
            &root.join(format!("original{}", std::env::consts::EXE_SUFFIX)),
        );
        for (reviewed, name) in [(false, "default"), (true, "reviewed")] {
            let mut options = args(root, name);
            options.assume_bindings = reviewed;
            options.min_renamed = 8;
            let report = run(options).unwrap();
            assert_eq!(report.skipped_entries, 1); // main contains println!
            assert_eq!(
                original,
                execute(
                    &root.join(name).join("main.rs"),
                    &root.join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
                )
            );
        }
    }
    #[test]
    #[cfg(unix)]
    fn symlinks_are_rejected() {
        let dir = fixture();
        let root = dir.path();
        fs::write(root.join("external"), "outside").unwrap();
        std::os::unix::fs::symlink(root.join("external"), root.join("input/link")).unwrap();
        assert!(run(args(root, "output")).is_err());
        assert!(!root.join("output").exists());
    }
}
