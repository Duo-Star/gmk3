// src/ir/method.rs
//! 方法注册表：IR 与计算库（`pakoo_forest`）之间的唯一接口表。
//!
//! 方法名规范：`输出类型码_动词_参数类型码`（如 `d_xy_ff`、`l_join_dd`）。
//! DSL 只认规范名；UI 通过 `alias_zh` / `alias_en` 显示别名。
//! 交互层由本表反查"选中对象可参与的构造"。

use crate::monkey::kinds::{GeoKind, GeoObject};
use crate::pakoo_forest::geo::{Conic, Dot, Line};

/// 求值期实参（已由求值器解析）。
pub enum ArgValue<'a> {
    Ref(&'a GeoObject),
    Lit(f64),
}

/// 方法求值结果：成功返回几何对象，失败返回错误说明。
pub type EvalOutcome = Result<GeoObject, &'static str>;

/// 参数槽类型。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParamKind {
    /// 对象引用，限定类型。
    Ref(GeoKind),
    /// 实数字面量。
    Lit,
}

/// 方法定义：DSL 规范名 + 签名 + 求值函数 + UI 元数据。
pub struct MethodDef {
    pub name: &'static str,
    pub out: GeoKind,
    pub ins: &'static [ParamKind],
    pub alias_zh: &'static str,
    pub alias_en: &'static str,
    pub eval: fn(&[ArgValue<'_>]) -> EvalOutcome,
}

/// 注册表下标。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct MethodId(pub u16);

impl MethodId {
    #[inline]
    pub fn def(self) -> &'static MethodDef {
        &METHODS[self.0 as usize]
    }
}

/// 由规范名查方法。
pub fn method_by_name(name: &str) -> Option<MethodId> {
    METHODS
        .iter()
        .position(|m| m.name == name)
        .map(|i| MethodId(i as u16))
}

// ---------------------------------------------------------------------------
// 求值辅助：类型化下转型（解析期已校验签名，此处仅为防御）
// ---------------------------------------------------------------------------

#[inline]
fn lit(args: &[ArgValue<'_>], i: usize) -> Result<f64, &'static str> {
    match args[i] {
        ArgValue::Lit(x) if x.is_finite() => Ok(x),
        _ => Err("实参类型错误：此处需要实数"),
    }
}

#[inline]
fn dot(args: &[ArgValue<'_>], i: usize) -> Result<Dot, &'static str> {
    match args[i] {
        ArgValue::Ref(GeoObject::Dot(d)) => Ok(*d),
        _ => Err("实参类型错误：此处需要点"),
    }
}

#[inline]
fn line(args: &[ArgValue<'_>], i: usize) -> Result<Line, &'static str> {
    match args[i] {
        ArgValue::Ref(GeoObject::Line(l)) => Ok(*l),
        _ => Err("实参类型错误：此处需要直线"),
    }
}

#[inline]
fn conic(args: &[ArgValue<'_>], i: usize) -> Result<Conic, &'static str> {
    match args[i] {
        ArgValue::Ref(GeoObject::Conic(c)) => Ok(*c),
        _ => Err("实参类型错误：此处需要圆锥曲线"),
    }
}

use ParamKind::{Lit as PLit, Ref as PRef};
use GeoKind::{Conic as KC, Dot as KD, Line as KL};

// ---------------------------------------------------------------------------
// 注册表（M1 首批方法；新增方法在此追加，计算库函数需先存在）
// ---------------------------------------------------------------------------

pub static METHODS: &[MethodDef] = &[
    MethodDef {
        name: "d_xy_ff",
        out: KD,
        ins: &[PLit, PLit],
        alias_zh: "点",
        alias_en: "Point",
        eval: |args| {
            let x = lit(args, 0)?;
            let y = lit(args, 1)?;
            Ok(GeoObject::Dot(Dot::from_affine_f64(x, y)))
        },
    },
    MethodDef {
        name: "l_join_dd",
        out: KL,
        ins: &[PRef(KD), PRef(KD)],
        alias_zh: "过两点的直线",
        alias_en: "Line through two points",
        eval: |args| {
            let a = dot(args, 0)?;
            let b = dot(args, 1)?;
            Ok(GeoObject::Line(a.join(b)))
        },
    },
    MethodDef {
        name: "d_meet_ll",
        out: KD,
        ins: &[PRef(KL), PRef(KL)],
        alias_zh: "两直线交点",
        alias_en: "Intersection of two lines",
        eval: |args| {
            let a = line(args, 0)?;
            let b = line(args, 1)?;
            Ok(GeoObject::Dot(a.meet(b)))
        },
    },
    MethodDef {
        name: "c_circle_cp_dd",
        out: KC,
        ins: &[PRef(KD), PRef(KD)],
        alias_zh: "圆（圆心 + 圆上一点）",
        alias_en: "Circle (center & point)",
        eval: |args| {
            let center = dot(args, 0)?;
            let on = dot(args, 1)?;
            Ok(GeoObject::Conic(Conic::circle_from_center_point(
                center, on,
            )))
        },
    },
    MethodDef {
        name: "c_circle_3d_ddd",
        out: KC,
        ins: &[PRef(KD), PRef(KD), PRef(KD)],
        alias_zh: "三点定圆",
        alias_en: "Circle through 3 points",
        eval: |args| {
            let p1 = dot(args, 0)?;
            let p2 = dot(args, 1)?;
            let p3 = dot(args, 2)?;
            Ok(GeoObject::Conic(Conic::circle_from_3_dots(p1, p2, p3)))
        },
    },
    MethodDef {
        name: "c_conic5d_ddddd",
        out: KC,
        ins: &[PRef(KD), PRef(KD), PRef(KD), PRef(KD), PRef(KD)],
        alias_zh: "五点圆锥曲线",
        alias_en: "Conic through 5 points",
        eval: |args| {
            let p0 = dot(args, 0)?;
            let p1 = dot(args, 1)?;
            let p2 = dot(args, 2)?;
            let p3 = dot(args, 3)?;
            let p4 = dot(args, 4)?;
            Ok(GeoObject::Conic(Conic::conic_from_5_dots(
                p0, p1, p2, p3, p4,
            )))
        },
    },
    MethodDef {
        name: "c_conic_cde_ddd",
        out: KC,
        ins: &[PRef(KD), PRef(KD), PRef(KD)],
        alias_zh: "共轭椭圆（中心 + 两点）",
        alias_en: "Ellipse from center & conjugate points",
        eval: |args| {
            let center = dot(args, 0)?;
            let p1 = dot(args, 1)?;
            let p2 = dot(args, 2)?;
            Ok(GeoObject::Conic(
                Conic::conic_from_center_conjugate_ellipse(center, p1, p2),
            ))
        },
    },
    MethodDef {
        name: "d_param_cf",
        out: KD,
        ins: &[PRef(KC), PLit],
        alias_zh: "曲线上参数点",
        alias_en: "Point on conic by parameter",
        eval: |args| {
            let c = conic(args, 0)?;
            let t = lit(args, 1)?;
            Ok(GeoObject::Dot(c.point_at_f64(t, 1e-12)))
        },
    },
    MethodDef {
        name: "d_mid_dd",
        out: KD,
        ins: &[PRef(KD), PRef(KD)],
        alias_zh: "中点",
        alias_en: "Midpoint",
        eval: |args| {
            let a = dot(args, 0)?;
            let b = dot(args, 1)?;
            Ok(GeoObject::Dot(a.mid(&b)))
        },
    },
    MethodDef {
        name: "l_polar_dc",
        out: KL,
        ins: &[PRef(KD), PRef(KC)],
        alias_zh: "极线",
        alias_en: "Polar",
        eval: |args| {
            let p = dot(args, 0)?;
            let c = conic(args, 1)?;
            Ok(GeoObject::Line(c.polar(&p)))
        },
    },
    MethodDef {
        name: "l_tangent_dc",
        out: KL,
        ins: &[PRef(KD), PRef(KC)],
        alias_zh: "切线",
        alias_en: "Tangent",
        eval: |args| {
            let p = dot(args, 0)?;
            let c = conic(args, 1)?;
            c.tangent_or_not(&p, 1e-12)
                .map(GeoObject::Line)
                .ok_or("该点不在曲线上或为奇异点：无切线")
        },
    },
    MethodDef {
        name: "dd_ins_cl",
        out: GeoKind::Ddot,
        ins: &[PRef(KC), PRef(KL)],
        alias_zh: "直线与圆锥曲线交点",
        alias_en: "Intersections of conic & line",
        eval: |args| {
            let c = conic(args, 0)?;
            let l = line(args, 1)?;
            Ok(GeoObject::Ddot(c.intersect_line(&l)))
        },
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_names_unique() {
        for (i, m) in METHODS.iter().enumerate() {
            for other in &METHODS[i + 1..] {
                assert_ne!(m.name, other.name, "方法名重复：{}", m.name);
            }
        }
    }

    #[test]
    fn method_names_match_signatures() {
        for m in METHODS {
            let codes: String = m
                .ins
                .iter()
                .map(|p| match p {
                    ParamKind::Ref(k) => k.code().to_string(),
                    ParamKind::Lit => "f".to_string(),
                })
                .collect();
            let prefix = format!("{}_", m.out.code());
            let suffix = format!("_{codes}");
            assert!(
                m.name.starts_with(&prefix),
                "方法 {} 应以 {prefix} 开头（输出类型码）",
                m.name
            );
            assert!(
                m.name.ends_with(&suffix),
                "方法 {} 应以 {suffix} 结尾（参数类型码）",
                m.name
            );
        }
    }

    #[test]
    fn aliases_present() {
        for m in METHODS {
            assert!(!m.alias_zh.is_empty(), "{} 缺少中文别名", m.name);
            assert!(!m.alias_en.is_empty(), "{} 缺少英文别名", m.name);
        }
    }
}
