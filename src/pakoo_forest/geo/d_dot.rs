use crate::pakoo_forest::geo::{Dot, Line};

struct Ddot {
    p: Dot,
    q: Dot,
}

impl Ddot {
    #[inline]
    pub const fn new(p_:Dot, q_:Dot) -> Ddot {
        Ddot { p: p_, q: q_ }
    }

    /// 虚圆点  各向同性点
    pub const CYCLIC: Ddot = Ddot::new(Dot::I, Dot::J);

    ///
    pub fn line(&self) -> Line {
        Line::from_points(&self.p, &self.q)
    }


}




