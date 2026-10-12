#![allow(clippy::needless_late_init)] // Deliberately test delayed initialization.
use rustveil::{obfuscate, veil_str};

#[obfuscate(seed = 72, strict, assume_bindings)]
fn shadow(x: i32) -> i32 {
    let x = x + 1;
    let y = {
        let x = x * 2;
        x + 3
    };
    let x = x + y;
    x
}
#[obfuscate(strict, assume_bindings)]
fn closures_and_loops(start: i32) -> i32 {
    let mut total = start;
    for n in 1..4 {
        let f = |n: i32| {
            let increment = n * 2;
            increment
        };
        total += f(n);
    }
    total
}
#[derive(Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}
#[obfuscate(strict, assume_bindings)]
fn point(input: i32) -> Point {
    let x = input;
    let y = x + 1;
    Point { x, y }
}
#[obfuscate(strict, assume_bindings)]
fn raw_identifiers() -> i32 {
    let r#type = 4;
    let f = || r#type + 1;
    f()
}
#[obfuscate(strict, assume_bindings)]
fn mutable_refs() -> i32 {
    let mut value = 1;
    let borrowed = &mut value;
    *borrowed += 3;
    value
}
#[obfuscate(strict, assume_bindings)]
fn delayed_init() -> i32 {
    let answer: i32;
    answer = 42;
    answer
}
#[obfuscate(strict, assume_bindings)]
fn restore_outer() -> i32 {
    let x = 12;
    let f = |x: i32| x * 2;
    let a = f(3);
    let mut total = 0;
    for x in 1..3 {
        total += x;
    }
    x + a + total
}
#[obfuscate]
fn macro_passthrough() -> String {
    let value = 7;
    format!("{value}")
}
#[obfuscate(strict, assume_bindings)]
const fn const_function(x: u32) -> u32 {
    let doubled = x * 2;
    doubled
}

impl Point {
    #[obfuscate(strict, assume_bindings)]
    fn sum(&self) -> i32 {
        let result = self.x + self.y;
        result
    }
}

#[obfuscate(require_renames, assume_bindings)]
fn patterns_and_guards(input: Option<Point>) -> i32 {
    let fallback = 7;
    let result = match input {
        Option::Some(Point { x, y }) if x > y => x - y,
        Option::Some(Point { x: n @ 0..=10, y }) => n + y,
        Option::Some(Point { x, .. }) => x,
        Option::None => fallback,
    };
    result + fallback
}

#[test]
fn patterns_work_through_the_public_macro() {
    assert_eq!(patterns_and_guards(Some(Point { x: 8, y: 3 })), 12);
    assert_eq!(patterns_and_guards(Some(Point { x: 2, y: 3 })), 12);
    assert_eq!(patterns_and_guards(Some(Point { x: -3, y: 3 })), 4);
    assert_eq!(patterns_and_guards(None), 14);
}

#[obfuscate(require_renames, assume_bindings)]
fn alternatives(input: Result<Point, Point>) -> i32 {
    let x = 100;
    let result = match input {
        Ok(Point { x, y }) | Err(Point { x, y }) if x > y => x - y,
        Ok(Point { x, .. }) | Err(Point { x, .. }) => x,
    };
    result + x
}

#[obfuscate(require_renames)]
fn nested_alternatives(input: Option<Result<i32, i32>>) -> i32 {
    match input {
        Some(Ok(ref n) | Err(ref n)) if *n > 0 => *n,
        Some(Ok(n @ -10..=0) | Err(n @ -10..=0)) => -n,
        _ => 0,
    }
}

#[test]
fn or_patterns_preserve_guards_and_outer_bindings() {
    for input in [Ok(Point { x: 7, y: 2 }), Err(Point { x: 7, y: 2 })] {
        assert_eq!(alternatives(input), 105);
    }
    for input in [Ok(Point { x: 2, y: 7 }), Err(Point { x: 2, y: 7 })] {
        assert_eq!(alternatives(input), 102);
    }
    for n in -12_i32..=12 {
        let expected = if n >= -10 { n.abs() } else { 0 };
        assert_eq!(nested_alternatives(Some(Ok(n))), expected);
        assert_eq!(nested_alternatives(Some(Err(n))), expected);
    }
    assert_eq!(nested_alternatives(None), 0);
}

#[test]
fn behavior_matches_original() {
    assert_eq!(shadow(2), 12);
    assert_eq!(closures_and_loops(5), 17);
    assert_eq!(point(4), Point { x: 4, y: 5 });
    assert_eq!(point(4).sum(), 9);
    assert_eq!(raw_identifiers(), 5);
    assert_eq!(mutable_refs(), 4);
    assert_eq!(delayed_init(), 42);
    assert_eq!(restore_outer(), 21);
    assert_eq!(macro_passthrough(), "7");
    const V: u32 = const_function(3);
    assert_eq!(V, 6);
}

fn cached_string() -> &'static str {
    veil_str!("Unicode: ગુજરાતી 🦀\0\n", 22)
}
#[test]
fn strings_preserve_utf8_empty_bytes_and_static_lifetime() {
    assert_eq!(cached_string(), "Unicode: ગુજરાતી 🦀\0\n");
    assert_eq!(veil_str!(""), "");
    assert_eq!(veil_str!(r#"raw \ string"#), r#"raw \ string"#);
    assert!(std::ptr::eq(
        cached_string().as_ptr(),
        cached_string().as_ptr()
    ));
}
#[test]
fn string_initialization_is_thread_safe() {
    let workers: Vec<_> = (0..16).map(|_| std::thread::spawn(cached_string)).collect();
    for worker in workers {
        assert_eq!(worker.join().unwrap(), cached_string());
    }
}
