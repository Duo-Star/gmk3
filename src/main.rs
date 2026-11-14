use gmk3::pakoo_forest::calc::Complex;
use gmk3::pakoo_forest::calc::Mat3C;

fn c(re: f64, im: f64) -> Complex {
    Complex::new(re, im)
}

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

    let m =  Mat3C::new([
        [c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)],
        [c(4.0, 0.0), c(5.0, 0.0), c(6.0, 0.0)],
        [c(7.0, 0.0), c(8.0, 0.0), c(10.0, 0.0)],
    ]);

    println!("{}", m);

}
