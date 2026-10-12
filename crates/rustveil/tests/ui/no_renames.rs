use rustveil::obfuscate;

#[obfuscate(require_renames)]
fn unchanged(value: i32) -> i32 { value + 1 }

fn main() {}
