/// 画布渲染场景：只包含点、直线、圆锥曲线三种图元（使用实部齐次坐标/矩阵）。
#[derive(Clone, Debug, Default)]
pub struct CanvasScene {
    /// 点齐次坐标 [x, y, z]
    pub dots: Vec<[f64; 3]>,
    /// 直线齐次协变坐标 [a, b, c] (ax + by + cz = 0)
    pub lines: Vec<[f64; 3]>,
    /// 圆锥曲线对称矩阵 [[A, B, D], [B, C, E], [D, E, F]]
    pub conics: Vec<[[f64; 3]; 3]>,
}

impl CanvasScene {
    /// 验证画布用的初始几何场景（默认无射影变换时直接可见）
    pub fn demo() -> Self {
        Self {
            dots: vec![
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [-1.0, -1.0, 1.0],
                [2.0, 1.0, 1.0],
            ],
            lines: vec![
                [0.0, 1.0, 0.0],   // y = 0
                [1.0, 0.0, 0.0],   // x = 0
                [1.0, -1.0, 1.0],  // x - y + 1 = 0
                [0.0, 0.0, 1.0],   // 无穷远直线 l_inf（默认不可见，开启射影变换后显形）
            ],
            conics: vec![
                // 单位圆 x^2 + y^2 - 1 = 0
                [
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [0.0, 0.0, -1.0],
                ],
                // 双曲线 x^2 - y^2 - 1 = 0
                [
                    [1.0, 0.0, 0.0],
                    [0.0, -1.0, 0.0],
                    [0.0, 0.0, -1.0],
                ],
                // 抛物线 y - x^2 = 0
                [
                    [-1.0, 0.0, 0.0],
                    [0.0, 0.0, 0.5],
                    [0.0, 0.5, 0.0],
                ],
            ],
        }
    }

    pub fn clear(&mut self) {
        self.dots.clear();
        self.lines.clear();
        self.conics.clear();
    }

    pub fn push_dot(&mut self, dot: [f64; 3]) {
        self.dots.push(dot);
    }

    pub fn push_line(&mut self, line: [f64; 3]) {
        self.lines.push(line);
    }

    pub fn push_conic(&mut self, conic: [[f64; 3]; 3]) {
        self.conics.push(conic);
    }

    // 后续接入 pakoo_forest::geo::{Dot, Line, Conic} 时，只需按如下形式取实部填入：
    // pub fn push_geo_dot(&mut self, d: &Dot) {
    //     if d.is_real() { self.dots.push([d.x().re, d.y().re, d.z().re]); }
    // }
}