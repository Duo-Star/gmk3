//! 射影几何层：以 `calc` 的线性代数类型为内核的几何 Newtype。
//!
//! # 对象与对偶
//!
//! - [`Dot`]：齐次点 `(x : y : z)`（`Vec3C` 的 Newtype，逆变）；
//! - [`Line`]：齐次直线 `(a : b : c)`，点 `(x:y:z)` 在线上当且仅当
//!   `a·x + b·y + c·z = 0`（`Cov3C` 的 Newtype，协变）。
//!
//! 点与直线互为对偶：两点连线 [`Dot::join`] 与两线交点 [`Line::meet`]
//! 都是叉积；关联关系是协变/逆变的对偶配对 `Line × Dot → Complex`
//! （`l * p == 0` ⇔ 点在线上）。
//!
//! # 尺度约定
//!
//! 齐次坐标只定义到整体非零比例。本模块**不**在构造时归一化（保持叉积的精确性），
//! 所有关联/相等判定均与尺度无关；需要单位代表元时显式调用 `normalize()`。
//!
//! # 绝对与无穷远
//!
//! - [`Line::INF`]：无穷远直线 `(0 : 0 : 1)`；
//! - [`Dot::I`] / [`Dot::J`]：两个虚圆点 `(1 : ±i : 0)`——欧氏度量的"绝对"。
//!   平行判定 [`Line::is_pall`] 由无穷远直线定义，垂直判定 [`Line::is_vert`]
//!   由两个虚圆点定义（无穷远点与虚圆点构成调和四点）。

pub mod conic;
pub mod d_dot;
pub mod d_x_line;
pub mod dot;
pub mod line;
pub mod q_dot;
pub mod x_line;

pub use dot::Dot;
pub use line::Line;
