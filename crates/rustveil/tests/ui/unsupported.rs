use rustveil::obfuscate;

#[obfuscate(require_renames)]
fn unsupported() { println!("visible"); }

fn main() {}
