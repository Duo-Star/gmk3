use gmk3::pakoo_forest::calc::Complex;

fn main() {
    let z = Complex::new(3.0, 4.0);
    println!("z          = {z}");
    println!("conj(z)    = {}", z.conj());
    println!("|z|        = {}", z.abs());
    println!("z·conj(z)  = {}", z * z.conj());
    println!("√-1        = {}", Complex::new(-1.0, 0.0).sqrt());
    println!(
        "e^(iπ) + 1 = {}",
        (Complex::I * std::f64::consts::PI).exp() + 1.0
    );
    println!("i^i        = {}", Complex::I.pow_complex(Complex::I));
    println!("ln(1+i)    = {}", Complex::new(1.0, 1.0).ln());
}
