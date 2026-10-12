use rustveil::obfuscate;

#[obfuscate(seed = 1, seed = 2)]
fn invalid() {}

fn main() {}
