// src/pakoo_forest/geo/q_dot.rs
// ----------------------------------
//          p  pn-mq : xl1   dp[1]
//          |
//          |
// m ----------------- n
//          | \center
//          |                dp[1]-dp[2] : line
//          q  pm-nq : xl2   dp[2]
// ----------------------------------

use crate::pakoo_forest::geo::d_x_line::DXLine;
use crate::pakoo_forest::geo::x_line::XLine;
use crate::pakoo_forest::geo::{Ddot, Dot, Line};


// 一对"一对点"
struct Qdot {
    p: Dot,
    q: Dot,
    m: Dot,
    n: Dot,
}

impl Qdot {
    #[inline]
    pub const fn new(p_: Dot, q_: Dot, m_: Dot, n_: Dot) -> Qdot {
        Qdot {
            p: p_,
            q: q_,
            m: m_,
            n: n_,
        }
    }

    /// 从两个骈点构造
    #[inline]
    pub const fn from_ddot2(ddot_1: Ddot, ddot_2: Ddot) -> Qdot {
        Qdot {
            p: ddot_1.p(),
            q: ddot_1.q(),
            m: ddot_2.p(),
            n: ddot_2.q(),
        }
    }

    /// 获得其中一个骈点 1
    #[inline]
    pub fn ddot_1(&self) -> Ddot {
        Ddot::new(self.p, self.q)
    }
    /// 获得其中一个骈点 2
    #[inline]
    pub fn ddot_2(&self) -> Ddot {
        Ddot::new(self.m, self.n)
    }

    /// 对顶点连线 1
    #[inline]
    pub fn l1(&self) -> Line {
        self.ddot_1().line()
    }
    /// 对顶点连线 2
    #[inline]
    pub fn l2(&self) -> Line {
        self.ddot_2().line()
    }

    /// 心点
    #[inline]
    pub fn center(&self) -> Dot {
        self.l1().meet(self.l2())
    }

    /// 衍骈点
    #[inline]
    pub fn d_dot(&self) -> Ddot {
        Ddot::new(
            self.p.join(self.n).meet(self.m.join(self.q)),
            self.p.join(self.m).meet(self.n.join(self.q)),
        )
    }

    /// 衍线
    #[inline]
    pub fn line(&self) -> Line {
        self.d_dot().line()
    }


    /// 骈交叉直线
    #[inline]
    pub fn dxl(&self) -> DXLine {
        DXLine::new(
            XLine::from_d4(self.p, self.n, self.m, self.q),
            XLine::from_d4(self.p, self.m, self.n, self.q),
        )
    }
}
