use crate::pakoo_forest::geo::Conic;

impl Conic {
    // ------------------------------------------------------------------
    // 分类
    // ------------------------------------------------------------------

    /// 复射影分类。`eps = 0` 精确判零；`eps > 0` 归一化后容差判零。
    ///
    /// 输出："零型" | "二重直线" | "平行直线型" | "交叉直线型" |
    /// "圆" | "抛物线" | "横截型" | "无效型"（含 NaN/无穷）。
    pub fn classify_complex(&self, eps: f64) -> &'static str {
        if !self.is_finite() {
            return "无效型";
        }
        let q = if eps == 0.0 {
            *self
        } else {
            self.normalize().unwrap_or(*self)
        };
        let (a, b, c) = (q.a(), q.b(), q.c());
        let delta = b * b - a * c;
        let circle_like = (a - c).abs() <= eps && b.abs() <= eps;
        match self.rank(eps) {
            0 => "零型",
            1 => "二重直线",
            2 => {
                if delta.abs() <= eps {
                    "平行直线型"
                } else {
                    "交叉直线型"
                }
            }
            _ => {
                if circle_like {
                    "圆"
                } else if delta.abs() <= eps {
                    "抛物线"
                } else {
                    "横截型"
                }
            }
        }
    }

    /// 实系数分类（调用方保证系数为实或虚部可忽略；[`Conic::classify`] 自动分派）。
    ///
    /// "实圆" | "虚圆" |
    /// "实椭圆" | "虚椭圆" |
    /// "实抛物线" |
    /// "实双曲线" | "等轴双曲线" |
    /// "两条实平行线" | "两条实相交线" |
    /// "两条实垂直相交线" | "单点" | "点圆" | "仿射空集（两共轭复平行线）" |
    /// "二重实直线" | "零型" | "无效型"。
    pub fn classify_real(&self, eps: f64) -> &'static str {
        if !self.is_finite() {
            return "无效型";
        }
        let q = if eps == 0.0 {
            *self
        } else {
            self.normalize().unwrap_or(*self)
        };
        let (a, b, c) = (q.a().re, q.b().re, q.c().re);
        let (d, e, f) = (q.d().re, q.e().re, q.f().re);

        let delta = b * b - a * c;
        let parallel = delta.abs() <= eps;
        let circle_like = (a - c).abs() <= eps && b.abs() <= eps;
        let ac_zero = (a + c).abs() <= eps;

        match self.rank(eps) {
            0 => "零型",
            1 => "二重实直线",
            2 => {
                // 主 2×2 子式之和 = 两个非零特征值之积：
                // 负 → 不定 (1,1,1)；正 → 定号 (2,0,1)/(0,2,1)
                let m = (a * c - b * b) + (a * f - d * d) + (c * f - e * e);
                if m < -eps {
                    // 不定：两条实直线
                    if parallel {
                        "两条实平行线"
                    } else if ac_zero {
                        "两条实垂直相交线"
                    } else {
                        "两条实相交线"
                    }
                } else if parallel {
                    // 定号且核在无穷远直线上：两共轭复平行线，实射影为空
                    "仿射空集（两共轭复平行线）"
                } else if circle_like {
                    "点圆"
                } else {
                    "单点"
                }
            }
            _ => {
                // 秩 3
                if circle_like {
                    let (a2, b2, c2, d2, e2, f2) = if a < 0.0 {
                        (-a, -b, -c, -d, -e, -f)
                    } else {
                        (a, b, c, d, e, f)
                    };
                    let s = crate::pakoo_forest::geo::conic::schur_complement(a2, b2, c2, d2, e2, f2);
                    if s < -eps {
                        "实圆"
                    } else if s > eps {
                        "虚圆"
                    } else {
                        "圆" // S ≈ 0 的临界情形（秩 3 时理论上不出现）
                    }
                } else if parallel {
                    "实抛物线"
                } else if delta < -eps {
                    // Δ∞ < 0：椭圆族
                    let (a2, b2, c2, d2, e2, f2) = if a < 0.0 {
                        (-a, -b, -c, -d, -e, -f)
                    } else {
                        (a, b, c, d, e, f)
                    };
                    let s = crate::pakoo_forest::geo::conic::schur_complement(a2, b2, c2, d2, e2, f2);
                    if s < -eps {
                        "实椭圆"
                    } else if s > eps {
                        "虚椭圆"
                    } else {
                        "椭圆"
                    }
                } else if ac_zero {
                    // Δ∞ > 0 且 A + C = 0
                    "等轴双曲线"
                } else {
                    "实双曲线"
                }
            }
        }
    }

    /// 自动分派：实系数（`eps = 0` 时精确判定，否则按容差）走实分类，
    /// 否则走复射影分类。
    pub fn classify(&self, eps: f64) -> &'static str {
        let real = if eps == 0.0 {
            self.is_real()
        } else {
            let q = self.normalize().unwrap_or(*self);
            q.q.m.iter().flatten().all(|x| x.im.abs() <= eps)
        };
        if real {
            self.classify_real(eps)
        } else {
            self.classify_complex(eps)
        }
    }
}