// src/ir/style.rs
//! 绘图属性：色板 / 形状 / 可见性 / 可拖动 / 粗细乘子。
//!
//! DSL 中属性以 `#` 引导的 token 序列表示，顺序任意、缺省走默认值、
//! 规则外的 token 静默忽略、同域重复后者覆盖。

use crate::monkey::kinds::GeoKind;

/// 点的基准半径（px），粗细乘子 1 时的值。
pub const DOT_BASE_RADIUS_PX: f32 = 3.8;
/// 直线 / 圆锥曲线的基准线宽（px），粗细乘子 1 时的值。
pub const LINE_BASE_WIDTH_PX: f32 = 1.6;

/// 色板。全部**动态读主题**：色板随日间/夜间模式自动微调；
/// `Atom` 完全自适应（暗底白 / 亮底黑）。不允许用户自定义 RGB。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Palette {
    Atom,
    Red,
    Sun,
    Amber,
    Ube,
    Cyan,
    Forest,
    Navy,
    Sky,
    Witch,
}

impl Default for Palette {
    fn default() -> Self {
        Palette::Atom
    }
}

impl Palette {
    /// DSL 属性 token → 色板。
    pub fn from_token(tok: &str) -> Option<Self> {
        match tok {
            "atom" => Some(Palette::Atom),
            "red" => Some(Palette::Red),
            "sun" => Some(Palette::Sun),
            "amber" => Some(Palette::Amber),
            "ube" => Some(Palette::Ube),
            "cyan" => Some(Palette::Cyan),
            "forest" => Some(Palette::Forest),
            "navy" => Some(Palette::Navy),
            "sky" => Some(Palette::Sky),
            "witch" => Some(Palette::Witch),
            _ => None,
        }
    }

    /// 色板 → DSL 属性 token。
    pub fn token(self) -> &'static str {
        match self {
            Palette::Atom => "atom",
            Palette::Red => "red",
            Palette::Sun => "sun",
            Palette::Amber => "amber",
            Palette::Ube => "ube",
            Palette::Cyan => "cyan",
            Palette::Forest => "forest",
            Palette::Navy => "navy",
            Palette::Sky => "sky",
            Palette::Witch => "witch",
        }
    }

    /// 按当前主题解析为 RGBA（sRGB，不透明）。
    ///
    /// M1 使用内置色板表；主题微调后续打磨（M4）。
    pub fn resolve(self, dark: bool) -> [f32; 4] {
        let rgb: [f32; 3] = match (self, dark) {
            (Palette::Atom, true) => [0.95, 0.95, 0.95],
            (Palette::Atom, false) => [0.08, 0.08, 0.10],
            (Palette::Red, true) => [0.95, 0.42, 0.44],
            (Palette::Red, false) => [0.78, 0.15, 0.18],
            (Palette::Sun, true) => [0.96, 0.86, 0.30],
            (Palette::Sun, false) => [0.75, 0.58, 0.05],
            (Palette::Amber, true) => [0.98, 0.65, 0.32],
            (Palette::Amber, false) => [0.80, 0.42, 0.08],
            (Palette::Ube, true) => [0.66, 0.57, 0.92],
            (Palette::Ube, false) => [0.45, 0.35, 0.75],
            (Palette::Cyan, true) => [0.35, 0.87, 0.90],
            (Palette::Cyan, false) => [0.10, 0.55, 0.55],
            (Palette::Forest, true) => [0.35, 0.72, 0.45],
            (Palette::Forest, false) => [0.10, 0.45, 0.25],
            (Palette::Navy, true) => [0.42, 0.58, 0.90],
            (Palette::Navy, false) => [0.15, 0.28, 0.55],
            (Palette::Sky, true) => [0.55, 0.78, 0.98],
            (Palette::Sky, false) => [0.15, 0.42, 0.70],
            (Palette::Witch, true) => [0.68, 0.44, 0.82],
            (Palette::Witch, false) => [0.42, 0.16, 0.52],
        };
        [rgb[0], rgb[1], rgb[2], 1.0]
    }
}

/// 形状。语义按对象类型展开：
///
/// | 形状     | 点               | 直线 / 圆锥曲线 |
/// |----------|------------------|----------------|
/// | `solid`  | 实心圆           | 实线           |
/// | `dashed` | 十字交叉点型     | 虚线           |
/// | `dotted` | 更小的实心点     | 点划线         |
/// | `hollow` | 空心圆环         | 空心描边       |
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Shape {
    Solid,
    Dashed,
    Dotted,
    Hollow,
}

impl Default for Shape {
    fn default() -> Self {
        Shape::Solid
    }
}

impl Shape {
    pub fn from_token(tok: &str) -> Option<Self> {
        match tok {
            "solid" => Some(Shape::Solid),
            "dashed" => Some(Shape::Dashed),
            "dotted" => Some(Shape::Dotted),
            "hollow" => Some(Shape::Hollow),
            _ => None,
        }
    }

    pub fn token(self) -> &'static str {
        match self {
            Shape::Solid => "solid",
            Shape::Dashed => "dashed",
            Shape::Dotted => "dotted",
            Shape::Hollow => "hollow",
        }
    }
}

/// 绘图属性（`#` 之后的部分）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StyleSpec {
    pub color: Palette,
    /// 粗细乘子：实际尺寸 = 类型基准 × 倍率。默认 1。
    pub width: f32,
    pub shape: Shape,
    pub visible: bool,
    /// 名字标签是否显示。默认 `true`。
    pub label_visible: bool,
    /// 可拖动性；`None` = 不适用（直线 / 曲线只能通过上游对象改变）。
    /// 点类默认 `Some(true)`。
    pub draggable: Option<bool>,
}

impl StyleSpec {
    /// 按对象类型取默认属性。
    pub fn default_for(kind: GeoKind) -> Self {
        Self {
            color: Palette::Atom,
            width: 1.0,
            shape: Shape::Solid,
            visible: true,
            label_visible: true,
            draggable: match kind {
                GeoKind::Dot | GeoKind::Ddot | GeoKind::Qdot => Some(true),
                GeoKind::Line | GeoKind::Conic | GeoKind::XLine | GeoKind::DXLine => None,
            },
        }
    }

    /// 应用一个属性 token；规则外的 token 静默忽略，同域重复后者覆盖。
    pub fn apply_token(&mut self, tok: &str) {
        if let Some(p) = Palette::from_token(tok) {
            self.color = p;
            return;
        }
        if let Some(s) = Shape::from_token(tok) {
            self.shape = s;
            return;
        }
        match tok {
            "visual" => {
                self.visible = true;
                return;
            }
            "hidden" => {
                self.visible = false;
                return;
            }
            "label_visible" => {
                self.label_visible = true;
                return;
            }
            "label_hidden" => {
                self.label_visible = false;
                return;
            }
            "draggable" => {
                self.draggable = Some(true);
                return;
            }
            "fixed" => {
                self.draggable = Some(false);
                return;
            }
            _ => {}
        }
        // 唯一的数字属性：粗细乘子
        if let Ok(w) = tok.parse::<f32>() {
            if w.is_finite() && w > 0.0 {
                self.width = w;
            }
        }
    }
}

/// 名字是否与属性关键字冲突（属性 token 是保留字，不可作对象名）。
pub fn is_reserved_token(tok: &str) -> bool {
    Palette::from_token(tok).is_some()
        || Shape::from_token(tok).is_some()
        || matches!(
            tok,
            "visual" | "hidden" | "label_visible" | "label_hidden" | "draggable" | "fixed"
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_per_kind() {
        let dot = StyleSpec::default_for(GeoKind::Dot);
        assert_eq!(dot.color, Palette::Atom);
        assert_eq!(dot.width, 1.0);
        assert_eq!(dot.shape, Shape::Solid);
        assert!(dot.visible);
        assert!(dot.label_visible);
        assert_eq!(dot.draggable, Some(true));

        let line = StyleSpec::default_for(GeoKind::Line);
        assert_eq!(line.draggable, None);
    }

    #[test]
    fn apply_tokens_override_and_ignore() {
        let mut s = StyleSpec::default_for(GeoKind::Dot);
        s.apply_token("red");
        s.apply_token("fixed");
        s.apply_token("1.5");
        s.apply_token("hollow");
        s.apply_token("hollow"); // 同域重复：后者覆盖
        s.apply_token("not_a_token"); // 静默忽略
        s.apply_token("-2"); // 非法数字：忽略
        assert_eq!(s.color, Palette::Red);
        assert_eq!(s.draggable, Some(false));
        assert_eq!(s.width, 1.5);
        assert_eq!(s.shape, Shape::Hollow);

        s.apply_token("hidden");
        assert!(!s.visible);
        s.apply_token("visual");
        assert!(s.visible);

        s.apply_token("solid");
        s.apply_token("dashed");
        assert_eq!(s.shape, Shape::Dashed);

        s.apply_token("label_hidden");
        assert!(!s.label_visible);
        s.apply_token("label_visible");
        assert!(s.label_visible);
    }

    #[test]
    fn palette_tokens_round_trip() {
        for (p, tok) in [
            (Palette::Atom, "atom"),
            (Palette::Red, "red"),
            (Palette::Sun, "sun"),
            (Palette::Amber, "amber"),
            (Palette::Ube, "ube"),
            (Palette::Cyan, "cyan"),
            (Palette::Forest, "forest"),
            (Palette::Navy, "navy"),
            (Palette::Sky, "sky"),
            (Palette::Witch, "witch"),
        ] {
            assert_eq!(Palette::from_token(tok), Some(p));
            assert_eq!(p.token(), tok);
        }
    }

    #[test]
    fn shape_tokens_round_trip() {
        for (s, tok) in [
            (Shape::Solid, "solid"),
            (Shape::Dashed, "dashed"),
            (Shape::Dotted, "dotted"),
            (Shape::Hollow, "hollow"),
        ] {
            assert_eq!(Shape::from_token(tok), Some(s));
            assert_eq!(s.token(), tok);
        }
    }

    #[test]
    fn reserved_tokens() {
        assert!(is_reserved_token("red"));
        assert!(is_reserved_token("fixed"));
        assert!(is_reserved_token("hidden"));
        assert!(is_reserved_token("label_hidden"));
        assert!(!is_reserved_token("d1"));
    }
}
