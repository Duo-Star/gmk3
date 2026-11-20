//! 计算基础：纯线性代数数值类型（与几何解耦）。
//!
//! - [`Complex`]：复数标量；
//! - [`Vec3C`]：3 分量复列向量（逆变）；
//! - [`Cov3C`]：3 分量复行向量（协变）；
//! - [`Mat3C`]：3×3 复矩阵。
//!
//! 四者构成闭合的乘法代数：
//!
//! | 运算 | 结果 |
//! |---|---|
//! | `Mat3C * Vec3C` | `Vec3C`（线性映射作用） |
//! | `Vec3C * Cov3C` | `Mat3C`（外积） |
//! | `Cov3C * Vec3C` | `Complex`（对偶配对） |
//! | `Cov3C * Mat3C` | `Cov3C`（行向量变换） |
//!
//! 几何对象（点 / 直线 / 圆锥曲线等）以后以 Newtype 封装这些类型。

pub mod complex;
pub mod cov3c;
pub mod mat3c;
pub mod mat3f64;
pub mod vec3c;

//
pub use complex::Complex;
//
pub use cov3c::Cov3C;
//
pub use mat3c::Mat3C;
//
pub use vec3c::Vec3C;
