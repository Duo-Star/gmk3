// src/pakoo_forest/geo/conic/tools.rs

/// Schur 补 `S = F − (C·D² − 2·B·D·E + A·E²)/(A·C − B²)`。
///
/// 仅当 `Δ∞ = B² − AC < 0`（`Q∞` 正定或负定）时有意义；
/// 调用方需先按 `Q∞ > 0` 归一化（`A < 0` 时整体乘 `−1`）。
#[inline]
pub fn schur_complement(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> f64 {
    f - (c * d * d - 2.0 * b * d * e + a * e * e) / (a * c - b * b)
}
