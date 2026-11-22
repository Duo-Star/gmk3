// src/pakoo_forest/geo/dot/dot_pred.rs

use crate::pakoo_forest::calc::Complex;
use crate::pakoo_forest::geo::Dot;

impl Dot {
    // ------------------------------------------------------------------
    // 谓词 / 近似比较
    // ------------------------------------------------------------------

    /// 无效点 `(0 : 0 : 0)`。
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.p.is_zero()
    }

    /// 无穷远点（`z == 0` 且非无效点）。
    #[inline]
    pub fn is_at_infinity(&self) -> bool {
        self.p.v[2] == Complex::ZERO && !self.p.is_zero()
    }

    /// 所有分量均为实数（实点）。
    #[inline]
    pub fn is_real(&self) -> bool {
        self.p.v.iter().all(|x| x.is_real())
    }

    /// 所有分量均有限。
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.p.is_finite()
    }

    /// 射影相等（与整体缩放无关）：`|p × q| ≤ eps·(1 + |p|·|q|)`。
    /// `eps = 0` 时精确判定。
    ///
    /// 注意：派生的 `PartialEq` 是按分量精确比较的，跨缩放的比较请用本方法。
    #[inline]
    pub fn approx_eq(&self, other: &Dot, eps: f64) -> bool {
        let c = self.p.cross(other.p);
        c.norm() <= eps * (1.0 + self.p.norm() * other.p.norm())
    }
}
