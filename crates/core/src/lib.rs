//! Conservative, syntax-based local binding renaming. This is not name resolution.
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use syn::{
    visit::{self, Visit},
    visit_mut::{self, VisitMut},
    *,
};

#[derive(Debug, Default, Clone)]
pub struct Outcome {
    pub renamed: usize,
    pub skipped: Option<String>,
}

/// Explicit, version-stable FNV-1a mixing for reproducible names; not cryptography.
pub fn hash(bytes: &[u8], seed: u64) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325 ^ seed, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}

#[derive(Default)]
struct Guard {
    reason: Option<String>,
}
impl Guard {
    fn reject(&mut self, reason: &str) {
        if self.reason.is_none() {
            self.reason = Some(reason.into());
        }
    }
}
impl<'ast> Visit<'ast> for Guard {
    fn visit_macro(&mut self, _: &'ast Macro) {
        self.reject("macro tokens require expansion and name resolution");
    }
    fn visit_attribute(&mut self, _: &'ast Attribute) {
        self.reject("attributes inside the signature or body may affect bindings");
    }
    fn visit_item(&mut self, _: &'ast Item) {
        self.reject("nested items may introduce names");
    }
    fn visit_expr_let(&mut self, _: &'ast ExprLet) {
        self.reject("let chains and conditional patterns require name resolution");
    }
    fn visit_expr_const(&mut self, _: &'ast ExprConst) {
        self.reject("inline const blocks have a separate name environment");
    }
    fn visit_pat(&mut self, p: &'ast Pat) {
        match p {
            Pat::Ident(_)
            | Pat::Type(_)
            | Pat::Wild(_)
            | Pat::Tuple(_)
            | Pat::TupleStruct(_)
            | Pat::Struct(_)
            | Pat::Slice(_)
            | Pat::Reference(_)
            | Pat::Paren(_)
            | Pat::Rest(_)
            | Pat::Lit(_)
            | Pat::Path(_)
            | Pat::Range(_) => {}
            Pat::Or(_) => self.reject("or-patterns require shared binding identities"),
            Pat::Const(_) => self.reject("const patterns have a separate name environment"),
            _ => self.reject("unsupported pattern syntax"),
        }
        visit::visit_pat(self, p);
    }
    fn visit_expr(&mut self, e: &'ast Expr) {
        if matches!(e, Expr::Verbatim(_)) {
            self.reject("unrecognized expression syntax");
        }
        visit::visit_expr(self, e);
    }
}

fn collect_names(tokens: TokenStream, names: &mut HashSet<String>) {
    for token in tokens {
        match token {
            TokenTree::Ident(id) => {
                names.insert(id.to_string().trim_start_matches("r#").into());
            }
            TokenTree::Group(g) => collect_names(g.stream(), names),
            _ => {}
        }
    }
}

struct Renamer {
    scopes: Vec<HashMap<String, Ident>>,
    used: HashSet<String>,
    seed: u64,
    serial: u64,
    renamed: usize,
    assume_bindings: bool,
}
impl Renamer {
    fn lookup(&self, id: &Ident) -> Option<Ident> {
        let key = id.to_string();
        self.scopes
            .iter()
            .rev()
            .find_map(|s| s.get(key.trim_start_matches("r#")).cloned())
    }
    fn bind(&mut self, pat: &mut Pat) {
        match pat {
            Pat::Ident(p) => {
                let old = p.ident.to_string();
                let old = old.trim_start_matches("r#");
                // A bare identifier pattern can name a unit struct or a constant.
                // Explicit mut/ref/@ bindings, or shadowing of a known renamed
                // binding, are unambiguous without compiler name resolution.
                let known = self.lookup(&p.ident).is_some_and(|id| id != p.ident);
                if !self.assume_bindings
                    && p.mutability.is_none()
                    && p.by_ref.is_none()
                    && p.subpat.is_none()
                    && !known
                {
                    self.scopes
                        .last_mut()
                        .unwrap()
                        .insert(old.into(), p.ident.clone());
                    return;
                }
                let candidate = loop {
                    self.serial += 1;
                    // Preserve deliberate underscore prefixes and ordinary unused-variable lints.
                    let prefix = if old.starts_with('_') { "_rv_" } else { "rv_" };
                    let name = format!(
                        "{prefix}{:016x}",
                        hash(&self.serial.to_le_bytes(), self.seed)
                    );
                    if self.used.insert(name.clone()) {
                        break name;
                    }
                };
                let new = Ident::new(&candidate, p.ident.span());
                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert(old.into(), new.clone());
                p.ident = new;
                self.renamed += 1;
                if let Some((_, subpat)) = &mut p.subpat {
                    self.bind(subpat);
                }
            }
            Pat::Type(p) => self.bind(&mut p.pat),
            Pat::Reference(p) => self.bind(&mut p.pat),
            Pat::Paren(p) => self.bind(&mut p.pat),
            Pat::Tuple(p) => p.elems.iter_mut().for_each(|p| self.bind(p)),
            Pat::TupleStruct(p) => p.elems.iter_mut().for_each(|p| self.bind(p)),
            Pat::Slice(p) => p.elems.iter_mut().for_each(|p| self.bind(p)),
            Pat::Struct(p) => {
                for field in &mut p.fields {
                    let before = self.renamed;
                    self.bind(&mut field.pat);
                    if field.colon_token.is_none() && self.renamed != before {
                        field.colon_token = Some(Default::default());
                    }
                }
            }
            Pat::Wild(_) | Pat::Rest(_) | Pat::Lit(_) | Pat::Path(_) | Pat::Range(_) => {}
            _ => unreachable!("guard rejected unsupported patterns"),
        }
    }
}
impl VisitMut for Renamer {
    fn visit_block_mut(&mut self, block: &mut Block) {
        self.scopes.push(HashMap::new());
        for statement in &mut block.stmts {
            self.visit_stmt_mut(statement);
        }
        self.scopes.pop();
    }
    fn visit_local_mut(&mut self, local: &mut Local) {
        // A let initializer and its else block still see the previous binding.
        if let Some(init) = &mut local.init {
            self.visit_expr_mut(&mut init.expr);
            if let Some((_, expr)) = &mut init.diverge {
                self.visit_expr_mut(expr);
            }
        }
        // Const arguments in a type annotation may refer to previous locals.
        self.visit_pat_mut(&mut local.pat);
        self.bind(&mut local.pat);
    }
    fn visit_expr_path_mut(&mut self, path: &mut ExprPath) {
        visit_mut::visit_expr_path_mut(self, path);
        if path.qself.is_none()
            && path.path.leading_colon.is_none()
            && path.path.segments.len() == 1
        {
            let segment = path.path.segments.first_mut().unwrap();
            if matches!(segment.arguments, PathArguments::None) {
                if let Some(new) = self.lookup(&segment.ident) {
                    segment.ident = new;
                }
            }
        }
    }
    fn visit_field_value_mut(&mut self, field: &mut FieldValue) {
        // `Record { value }` becomes `Record { value: _rv_... }`.
        if field.colon_token.is_none() {
            if let Expr::Path(path) = &field.expr {
                if let Some(id) = path.path.get_ident() {
                    if self.lookup(id).is_some_and(|new| new != *id) {
                        field.colon_token = Some(Default::default());
                    }
                }
            }
        }
        visit_mut::visit_field_value_mut(self, field);
    }
    fn visit_expr_for_loop_mut(&mut self, expr: &mut ExprForLoop) {
        self.visit_expr_mut(&mut expr.expr);
        self.scopes.push(HashMap::new());
        self.bind(&mut expr.pat);
        self.visit_block_mut(&mut expr.body);
        self.scopes.pop();
    }
    fn visit_expr_match_mut(&mut self, expr: &mut ExprMatch) {
        // The scrutinee sees the outer scope. Each arm's bindings are visible
        // in its guard and body, but never in another arm or after the match.
        self.visit_expr_mut(&mut expr.expr);
        for arm in &mut expr.arms {
            self.scopes.push(HashMap::new());
            self.bind(&mut arm.pat);
            if let Some((_, guard)) = &mut arm.guard {
                self.visit_expr_mut(guard);
            }
            self.visit_expr_mut(&mut arm.body);
            self.scopes.pop();
        }
    }
    fn visit_expr_closure_mut(&mut self, expr: &mut ExprClosure) {
        // Parameter types and return type are resolved outside the value bindings.
        for pat in &mut expr.inputs {
            self.visit_pat_mut(pat);
        }
        self.visit_return_type_mut(&mut expr.output);
        self.scopes.push(HashMap::new());
        for pat in &mut expr.inputs {
            self.bind(pat);
        }
        self.visit_expr_mut(&mut expr.body);
        self.scopes.pop();
    }
}

/// Rename only value bindings in the body. Parameter names and all public item
/// names remain unchanged. Reject uncertain syntax before making any edits.
pub fn transform(sig: &Signature, body: &mut Block, seed: u64) -> Outcome {
    transform_with_options(sig, body, seed, false)
}

/// `assume_bindings` asserts that every bare identifier pattern in the function
/// is a variable binding, not a constant, unit struct, or enum variant. Qualify
/// constant/variant paths (e.g. `Option::None`) in reviewed code before use.
pub fn transform_with_options(
    sig: &Signature,
    body: &mut Block,
    seed: u64,
    assume_bindings: bool,
) -> Outcome {
    let mut guard = Guard::default();
    guard.visit_signature(sig);
    guard.visit_block(body);
    if let Some(reason) = guard.reason {
        return Outcome {
            renamed: 0,
            skipped: Some(reason),
        };
    }
    let mut used = HashSet::new();
    collect_names(sig.to_token_stream(), &mut used);
    collect_names(body.to_token_stream(), &mut used);
    let mut renamer = Renamer {
        scopes: vec![HashMap::new()],
        used,
        seed: hash(sig.to_token_stream().to_string().as_bytes(), seed),
        serial: 0,
        renamed: 0,
        assume_bindings,
    };
    renamer.visit_block_mut(body);
    Outcome {
        renamed: renamer.renamed,
        skipped: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(source: &str) -> (ItemFn, Outcome) {
        let mut f: ItemFn = syn::parse_str(source).unwrap();
        let r = transform_with_options(&f.sig, &mut f.block, 17, true);
        (f, r)
    }
    #[test]
    fn shadowing_initializer_uses_previous_binding() {
        let (f, out) = run("fn f(x: i32) -> i32 { let x = x + 1; let x = x * 2; x }");
        assert_eq!(out.renamed, 2);
        let text = f.to_token_stream().to_string();
        assert!(text.contains("= x + 1"));
        assert!(!text.contains("let x"));
    }
    #[test]
    fn unsafe_syntax_is_unchanged() {
        for source in [
            "fn f() { let x = 1; println!(\"{x}\"); }",
            "fn f(x: Option<i32>) { match x { Some(a) | Some(a) => (), _ => () } }",
            "fn f(x: Option<i32>) { if let Some(a) = x {} }",
            "fn f(x: i32) { match x { const { 1 } => (), _ => () } }",
            "fn f() { let x = 1; const A: i32 = 2; }",
            "fn f() { #[allow(unused)] let x = 1; }",
        ] {
            let original: ItemFn = syn::parse_str(source).unwrap();
            let (f, out) = run(source);
            assert!(out.skipped.is_some(), "{source}");
            assert_eq!(f, original);
        }
    }
    #[test]
    fn deterministic_and_seeded() {
        let source = "fn f() -> i32 { let value = 9; value }";
        let (first, _) = run(source);
        assert_eq!(first, run(source).0);
        let mut other: ItemFn = syn::parse_str(source).unwrap();
        transform_with_options(&other.sig, &mut other.block, 18, true);
        assert_ne!(first, other);
    }
    #[test]
    fn field_shorthand_is_expanded() {
        let (f, _) = run("fn f() -> Point { let x = 1; Point { x } }");
        assert!(f.to_token_stream().to_string().contains("Point { x : rv_"));
    }
    #[test]
    fn default_preserves_ambiguous_unit_patterns() {
        for source in ["fn f() { let Unit = Unit; }", "fn f() { let unit = unit; }"] {
            let mut f: ItemFn = syn::parse_str(source).unwrap();
            let before = f.clone();
            assert_eq!(transform(&f.sig, &mut f.block, 1).renamed, 0);
            assert_eq!(before, f);
        }
    }
    #[test]
    fn default_renames_explicit_bindings_and_their_shadowing() {
        let mut f: ItemFn =
            syn::parse_str("fn f() -> i32 { let mut x = 2; x += 1; let x = x + 2; x }").unwrap();
        assert_eq!(transform(&f.sig, &mut f.block, 1).renamed, 2);
    }
    #[test]
    fn nested_patterns_preserve_field_names_and_binding_modes() {
        let (f, out) =
            run("fn f(p: Point) { let Point { x: (ref a, mut b), ref y } = p; b += *a + *y; }");
        assert_eq!(out.renamed, 3);
        assert!(out.skipped.is_none());
        let text = f.to_token_stream().to_string();
        assert!(text.contains("x : (ref rv_"), "{text}");
        assert!(text.contains("y : ref rv_"), "{text}");
        syn::parse2::<ItemFn>(f.to_token_stream()).unwrap();
    }
    #[test]
    fn default_preserves_bare_variants_and_constants_in_matches() {
        let mut f: ItemFn = syn::parse_str("fn f(value: Option<i32>) -> i32 { match value { Some(mut n) => { n += 1; n }, None => 0 } }").unwrap();
        let out = transform(&f.sig, &mut f.block, 1);
        assert_eq!(out.renamed, 1);
        assert!(out.skipped.is_none());
        assert!(f.to_token_stream().to_string().contains("None => 0"));
        let mut f: ItemFn =
            syn::parse_str("fn f(value: i32) -> i32 { match value { LIMIT => 1, _ => 0 } }")
                .unwrap();
        let before = f.clone();
        assert_eq!(transform(&f.sig, &mut f.block, 1).renamed, 0);
        assert_eq!(f, before);
    }
    #[test]
    fn at_bindings_are_unambiguous_in_default_mode() {
        let mut f: ItemFn =
            syn::parse_str("fn f(value: i32) -> i32 { match value { n @ 1..=9 => n, _ => 0 } }")
                .unwrap();
        let out = transform(&f.sig, &mut f.block, 1);
        assert_eq!(out.renamed, 1);
        assert!(out.skipped.is_none());
    }
    #[test]
    fn pattern_attributes_reject_the_whole_function_without_edits() {
        let original: ItemFn = syn::parse_str("fn f(p: Point) { let mut n = 1; let Point { #[cfg(feature = \"x\")] x, .. } = p; n += 1; }").unwrap();
        let mut f = original.clone();
        assert!(transform_with_options(&f.sig, &mut f.block, 1, true)
            .skipped
            .is_some());
        assert_eq!(f, original);
    }
}
