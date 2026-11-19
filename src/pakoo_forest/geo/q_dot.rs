//
use crate::pakoo_forest::geo::{Dot, Line};

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
}
