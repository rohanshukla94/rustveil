#[rv::obfuscate(seed = 42, strict, assume_bindings)]
fn calculate_total(unit_price: u64, count: u64) -> u64 {
    let subtotal = unit_price.saturating_mul(count);
    let discount = if count >= 10 { subtotal / 20 } else { 0 };
    subtotal - discount
}

fn main() {
    let title = rv::veil_str!("RustVeil workspace demo — ready", 42);
    println!("{title}: {}", calculate_total(150, 12));
}
