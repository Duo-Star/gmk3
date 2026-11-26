// src/ir/kinds.rs
//! IR 对象类型：内核几何类型的统一枚举与 DSL 类型码。

use crate::pakoo_forest::geo::{Conic, Ddot, Dot, DXLine, Line, Qdot, XLine};

/// 几何对象类型（IR 层面）。
///
/// 类型码（DSL 方法名 `输出类型_动词_参数类型` 使用）：
/// `d` 点、`l` 直线、`c` 圆锥曲线、`dd` 骈点、`q` 四点形、
/// `xl` 交叉直线、`dxl` 一对交叉直线、`f` 实数字面量（仅作参数类型）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum GeoKind {
    Dot,
    Line,
    Conic,
    Ddot,
    Qdot,
    XLine,
    DXLine,
}

impl GeoKind {
    /// DSL 类型码。
    pub fn code(self) -> &'static str {
        match self {
            GeoKind::Dot => "d",
            GeoKind::Line => "l",
            GeoKind::Conic => "c",
            GeoKind::Ddot => "dd",
            GeoKind::Qdot => "q",
            GeoKind::XLine => "xl",
            GeoKind::DXLine => "dxl",
        }
    }

    /// 由 DSL 类型码解析。
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "d" => Some(GeoKind::Dot),
            "l" => Some(GeoKind::Line),
            "c" => Some(GeoKind::Conic),
            "dd" => Some(GeoKind::Ddot),
            "q" => Some(GeoKind::Qdot),
            "xl" => Some(GeoKind::XLine),
            "dxl" => Some(GeoKind::DXLine),
            _ => None,
        }
    }

    /// 中文名（用于诊断信息与 UI）。
    pub fn cn(self) -> &'static str {
        match self {
            GeoKind::Dot => "点",
            GeoKind::Line => "直线",
            GeoKind::Conic => "圆锥曲线",
            GeoKind::Ddot => "骈点",
            GeoKind::Qdot => "四点形",
            GeoKind::XLine => "交叉直线",
            GeoKind::DXLine => "一对交叉直线",
        }
    }
}

/// 求值后的几何对象（可绘制，或仅参与构造）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeoObject {
    Dot(Dot),
    Line(Line),
    Conic(Conic),
    Ddot(Ddot),
    Qdot(Qdot),
    XLine(XLine),
    DXLine(DXLine),
}

impl GeoObject {
    #[inline]
    pub fn kind(&self) -> GeoKind {
        match self {
            GeoObject::Dot(_) => GeoKind::Dot,
            GeoObject::Line(_) => GeoKind::Line,
            GeoObject::Conic(_) => GeoKind::Conic,
            GeoObject::Ddot(_) => GeoKind::Ddot,
            GeoObject::Qdot(_) => GeoKind::Qdot,
            GeoObject::XLine(_) => GeoKind::XLine,
            GeoObject::DXLine(_) => GeoKind::DXLine,
        }
    }

    /// 无效对象（全零）：重合直线的交点、重合点的连线等退化情形。
    #[inline]
    pub fn is_zero(&self) -> bool {
        match self {
            GeoObject::Dot(d) => d.is_zero(),
            GeoObject::Line(l) => l.is_zero(),
            GeoObject::Conic(c) => c.is_zero(),
            GeoObject::Ddot(dd) => dd.p().is_zero() || dd.q().is_zero(),
            GeoObject::Qdot(q) => {
                let (d1, d2) = (q.ddot_1(), q.ddot_2());
                d1.p().is_zero() || d1.q().is_zero() || d2.p().is_zero() || d2.q().is_zero()
            }
            GeoObject::XLine(xl) => xl.is_zero(),
            GeoObject::DXLine(dxl) => dxl.xlm().is_zero() || dxl.xln().is_zero(),
        }
    }
}

/// 内置常量：保留名与预留 `ObjectId` 一一对应（顺序即 ID，勿改动）。
///
/// - `XA` / `YA`：世界坐标轴（`y = 0` / `x = 0`）；
/// - `INF`：无穷远直线 `(0 : 0 : 1)`；
/// - `I` / `J`：两个虚圆点（欧氏度量的绝对）。
pub const CONSTANTS: [(&str, GeoObject); 5] = [
    ("XA", GeoObject::Line(Line::X_AXIS)),
    ("YA", GeoObject::Line(Line::Y_AXIS)),
    ("INF", GeoObject::Line(Line::INF)),
    ("I", GeoObject::Dot(Dot::I)),
    ("J", GeoObject::Dot(Dot::J)),
];
