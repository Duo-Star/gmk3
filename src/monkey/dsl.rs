// src/monkey/dsl.rs
//! DSL 文本 ⇄ Monkey 程序：解析与序列化。
//!
//! # 行语法
//!
//! ```text
//! <名字> <方法名> <实参>... [# <属性>...] [// 行注释]
//! ```
//!
//! - 文件首行 `// gmk3 <版本>` 为版本头（本身是注释，导入容忍缺失）；
//! - `#` 引导绘图属性（token 序列，顺序任意），`//` 到行尾为行注释；
//! - 整行 `#` 起头的行视为注释，空行跳过；
//! - 实参：实数（`-1.5`、`2e-3`）| 名字引用 | 内置常量（`XA YA INF I J`）；
//! - 实参只能引用先前语句：禁止前向引用 → 程序天然无环；
//! - 每条语句固定产出且只产出一个对象，不支持嵌套组合。

use crate::monkey::kinds::{CONSTANTS, GeoKind};
use crate::monkey::method::{method_by_name, ParamKind};
use crate::monkey::stmt::Arg;
use crate::monkey::style::{is_reserved_token, StyleSpec};
use crate::monkey::Monkey;

/// DSL 版本（文件首行 `// gmk3 0.0.5`）。
pub const DSL_NAME: &str = "gmk3";
pub const DSL_VERSION: &str = "0.0.5";

/// 带位置的解析诊断。
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    /// 1 基行号。
    pub line: usize,
    /// 1 基列号（token 起始，按字符计）。
    pub col: usize,
    pub message: String,
}

/// 解析失败时返回全部诊断（不遇错即停，一次给出所有问题，便于 AI 生成文件修复）。
pub fn parse(text: &str) -> Result<Monkey, Vec<Diagnostic>> {
    let mut program = Monkey::new();
    let mut diags: Vec<Diagnostic> = Vec::new();

    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        let (head, attrs) = tokenize_line(raw);
        // 整行注释（`#` 或 `//` 起头、或纯属性行）：跳过
        if head.is_empty() {
            continue;
        }

        // ------------------------------------------------------------
        // 1. 名字
        // ------------------------------------------------------------
        let name_tok = &head[0];
        let mut ok = true;

        if !valid_ident(&name_tok.text) {
            diags.push(diag(line_no, name_tok.col, format!("非法名字「{}」", name_tok.text)));
            ok = false;
        } else if is_reserved_name(&name_tok.text) {
            diags.push(diag(
                line_no,
                name_tok.col,
                format!("名字「{}」与保留字冲突", name_tok.text),
            ));
            ok = false;
        } else if program.contains_name(&name_tok.text) {
            diags.push(diag(line_no, name_tok.col, format!("名字「{}」重复定义", name_tok.text)));
            ok = false;
        }

        // ------------------------------------------------------------
        // 2. 方法名
        // ------------------------------------------------------------
        let method_tok = head.get(1);
        let Some(mid) = method_tok.and_then(|t| method_by_name(&t.text)) else {
            let (line, col, msg) = match method_tok {
                Some(t) => (line_no, t.col, format!("未知方法「{}」", t.text)),
                None => (line_no, 1, "缺少方法名".to_string()),
            };
            diags.push(diag(line, col, msg));
            continue;
        };
        let def = mid.def();

        // ------------------------------------------------------------
        // 3. 实参：个数与类型逐一校验
        // ------------------------------------------------------------
        let given = head.len().saturating_sub(2);
        if given != def.ins.len() {
            diags.push(diag(
                line_no,
                method_tok.map_or(1, |t| t.col),
                format!(
                    "方法「{}」需要 {} 个实参，得到 {given} 个",
                    def.name,
                    def.ins.len()
                ),
            ));
            continue;
        }

        let mut args: Vec<Arg> = Vec::with_capacity(def.ins.len());
        for (slot, tok) in head.iter().skip(2).enumerate() {
            match def.ins[slot] {
                ParamKind::Lit => match tok.text.parse::<f64>() {
                    Ok(x) if x.is_finite() => args.push(Arg::Lit(x)),
                    _ => {
                        diags.push(diag(
                            line_no,
                            tok.col,
                            format!("此处需要实数，得到「{}」", tok.text),
                        ));
                        ok = false;
                    }
                },
                ParamKind::Ref(kind) => match program.id_of(&tok.text) {
                    Some(id) => {
                        let got = program.kind_of(id);
                        if got == kind {
                            args.push(Arg::Id(id));
                        } else {
                            diags.push(diag(
                                line_no,
                                tok.col,
                                format!(
                                    "类型不符：方法「{}」此处需要{}，而「{}」是{}",
                                    def.name,
                                    kind.cn(),
                                    tok.text,
                                    got.cn()
                                ),
                            ));
                            ok = false;
                        }
                    }
                    None => {
                        diags.push(diag(
                            line_no,
                            tok.col,
                            format!("未知名字「{}」（未定义或前向引用）", tok.text),
                        ));
                        ok = false;
                    }
                },
            }
        }
        if !ok {
            continue;
        }

        // ------------------------------------------------------------
        // 4. 属性：默认值 + token 覆盖
        // ------------------------------------------------------------
        let mut style = StyleSpec::default_for(def.out);
        for tok in &attrs {
            style.apply_token(tok);
        }

        // 5. 入表（重名已查，理论上不失败）
        let _ = program.push(name_tok.text.clone(), mid, args, style);
    }

    if diags.is_empty() {
        Ok(program)
    } else {
        Err(diags)
    }
}

/// 序列化为 DSL 文本（含版本头；属性按规范顺序输出非默认项）。
pub fn emit(program: &Monkey) -> String {
    let mut out = String::with_capacity(program.stmts.len() * 24 + 16);
    out.push_str("// ");
    out.push_str(DSL_NAME);
    out.push(' ');
    out.push_str(DSL_VERSION);
    out.push('\n');

    for stmt in &program.stmts {
        out.push_str(&stmt.name);
        out.push(' ');
        out.push_str(stmt.method.def().name);
        for arg in &stmt.args {
            out.push(' ');
            match arg {
                Arg::Lit(x) => out.push_str(&fmt_f64(*x)),
                Arg::Id(id) => out.push_str(program.name_of(*id)),
            }
        }
        let attrs = format_attrs(&stmt.style, stmt.method.def().out);
        if !attrs.is_empty() {
            out.push_str(" # ");
            out.push_str(&attrs);
        }
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// 内部工具
// ---------------------------------------------------------------------------

/// 行内 token：文本 + 1 基起始列。
struct Tok {
    text: String,
    col: usize,
}

enum Stage {
    Head,
    Attrs,
}

/// 把一行切成（头部 token，属性 token）。
/// `//` 起即行注释（此后一律忽略）；`#` 起即属性区。
fn tokenize_line(line: &str) -> (Vec<Tok>, Vec<String>) {
    let chars: Vec<char> = line.chars().collect();
    let mut head: Vec<Tok> = Vec::new();
    let mut attrs: Vec<String> = Vec::new();
    let mut stage = Stage::Head;
    let mut i = 0;

    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        // 行注释
        if chars[i] == '/' && chars.get(i + 1) == Some(&'/') {
            break;
        }
        // 属性区
        if chars[i] == '#' {
            stage = Stage::Attrs;
            i += 1;
            continue;
        }
        let col = i + 1;
        let mut tok = String::new();
        while i < chars.len() && !chars[i].is_whitespace() {
            if chars[i] == '#' {
                break;
            }
            if chars[i] == '/' && chars.get(i + 1) == Some(&'/') {
                break;
            }
            tok.push(chars[i]);
            i += 1;
        }
        match stage {
            Stage::Head => head.push(Tok { text: tok, col }),
            Stage::Attrs => attrs.push(tok),
        }
    }
    (head, attrs)
}

fn diag(line: usize, col: usize, message: String) -> Diagnostic {
    Diagnostic { line, col, message }
}

/// Unicode 标识符：字母 / 下划线开头，其后字母数字下划线（中文等非 ASCII 字母可用）。
fn valid_ident(s: &str) -> bool {
    let mut cs = s.chars();
    match cs.next() {
        Some(c) if c.is_alphabetic() || c == '_' => {}
        _ => return false,
    }
    cs.all(|c| c.is_alphanumeric() || c == '_')
}

/// 名字是否与内置常量 / 方法名 / 属性关键字冲突。
fn is_reserved_name(name: &str) -> bool {
    CONSTANTS.iter().any(|(n, _)| *n == name)
        || method_by_name(name).is_some()
        || is_reserved_token(name)
}

/// 最短可往返的 f64 文本（Rust `Display` 即最短往返表示）。
fn fmt_f64(x: f64) -> String {
    format!("{x}")
}

/// 规范属性序列（只输出与默认值不同的项）：
/// 颜色 → 形状 → 粗细 → 可见性 → 标签可见性 → 拖动。
fn format_attrs(style: &StyleSpec, kind: GeoKind) -> String {
    let default = StyleSpec::default_for(kind);
    let mut parts: Vec<String> = Vec::with_capacity(6);

    if style.color != default.color {
        parts.push(style.color.token().to_string());
    }
    if style.shape != default.shape {
        parts.push(style.shape.token().to_string());
    }
    if style.width != default.width {
        parts.push(fmt_f64(style.width as f64));
    }
    if style.visible != default.visible {
        parts.push(if style.visible { "visual" } else { "hidden" }.to_string());
    }
    if style.label_visible != default.label_visible {
        parts.push(if style.label_visible {
            "label_visible"
        } else {
            "label_hidden"
        }
        .to_string());
    }
    match style.draggable {
        Some(false) => parts.push("fixed".to_string()),
        Some(true) if default.draggable != Some(true) => parts.push("draggable".to_string()),
        _ => {}
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monkey::stmt::ObjectId;
    use crate::monkey::style::{Palette, Shape};

    const SAMPLE: &str = include_str!("../../examples/sample.gmk3");

    #[test]
    fn sample_parses_seven_statements() {
        let program = parse(SAMPLE).expect("样例应解析成功");
        assert_eq!(program.stmts.len(), 7);
        assert_eq!(program.stmts[0].name, "d1");
        assert_eq!(program.stmts[0].method.def().name, "d_xy_ff");
        assert_eq!(program.stmts[0].args, vec![Arg::Lit(1.0), Arg::Lit(1.0)]);

        // d2 的属性
        let d2 = &program.stmts[1];
        assert_eq!(d2.style.color, Palette::Red);
        assert_eq!(d2.style.draggable, Some(false));
        assert_eq!(d2.style.width, 1.5);
        assert_eq!(d2.style.shape, Shape::Hollow);

        // c 引用 d1、d2
        assert_eq!(program.stmts[2].method.def().name, "c_circle_cp_dd");
        assert_eq!(
            program.stmts[2].args,
            vec![Arg::Id(ObjectId(5)), Arg::Id(ObjectId(6))]
        );

        // x 引用常量 XA（ID 0），y 引用常量 YA（ID 1）
        assert_eq!(program.stmts[5].args[1], Arg::Id(ObjectId(0)));
        assert_eq!(program.stmts[6].args[1], Arg::Id(ObjectId(1)));
    }

    #[test]
    fn round_trip_stable() {
        let p1 = parse(SAMPLE).expect("解析成功");
        let text1 = emit(&p1);
        let p2 = parse(&text1).expect("再解析成功");
        let text2 = emit(&p2);
        assert_eq!(text1, text2, "两次序列化应一致");
        assert_eq!(p1.stmts, p2.stmts, "往返后语句应一致");
    }

    #[test]
    fn comments_and_attributes() {
        let text = "\
// gmk3 0.0.5
# 这是整行注释
a d_xy_ff 1 2 // 行尾注释
b d_xy_ff -1.5 2e-3 # red fixed 1.5 hollow // 属性后再注释
";
        let program = parse(text).expect("解析成功");
        assert_eq!(program.stmts.len(), 2);
        assert_eq!(program.stmts[1].args, vec![Arg::Lit(-1.5), Arg::Lit(0.002)]);
        assert_eq!(program.stmts[1].style.color, Palette::Red);
    }

    #[test]
    fn duplicate_name_rejected() {
        let err = parse("a d_xy_ff 1 1\na d_xy_ff 2 2\n").unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].line, 2);
        assert!(err[0].message.contains("重复"));
    }

    #[test]
    fn unknown_method_rejected() {
        let err = parse("a d_zzz_ff 1 1\n").unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err[0].message.contains("未知方法"));
    }

    #[test]
    fn wrong_arity_rejected() {
        let err = parse("a d_xy_ff 1\n").unwrap_err();
        assert!(err[0].message.contains("实参"));
    }

    #[test]
    fn forward_reference_rejected() {
        let err = parse("l l_join_dd a b\na d_xy_ff 1 1\nb d_xy_ff 2 2\n").unwrap_err();
        assert_eq!(err.len(), 2); // a、b 均未知
        assert!(err.iter().all(|d| d.message.contains("未知名字")));
    }

    #[test]
    fn reserved_name_rejected() {
        for name in ["XA", "red", "d_xy_ff", "fixed"] {
            let err = parse(&format!("{name} d_xy_ff 1 1\n")).unwrap_err();
            assert!(
                err.iter().any(|d| d.message.contains("保留字")),
                "{name} 应触发保留字诊断：{err:?}"
            );
        }
    }

    #[test]
    fn invalid_identifier_rejected() {
        for name in ["1a", "a-b"] {
            let err = parse(&format!("{name} d_xy_ff 1 1\n")).unwrap_err();
            assert!(
                err.iter().any(|d| d.message.contains("非法名字")),
                "{name} 应触发非法名字诊断：{err:?}"
            );
        }
    }

    #[test]
    fn chinese_names_allowed() {
        let program = parse("原点 d_xy_ff 0 0\n点A d_xy_ff 1 1\n").expect("中文名应合法");
        assert_eq!(program.stmts[0].name, "原点");
        assert_eq!(program.stmts[1].name, "点A");
    }

    #[test]
    fn wrong_arg_type_rejected() {
        let err = parse("a d_xy_ff 1 1\nl l_join_dd a a\nc c_circle_cp_dd a l\n").unwrap_err();
        // 第 3 行：c_circle_cp_dd 第 2 个实参需要点，得到直线 l
        assert_eq!(err.len(), 1);
        assert!(err[0].message.contains("类型不符"), "{err:?}");
        assert_eq!(err[0].line, 3);
    }

    #[test]
    fn literal_where_ref_expected_rejected() {
        let err = parse("c c_circle_cp_dd 1 2\n").unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("未知名字")));
    }

    #[test]
    fn ref_where_literal_expected_rejected() {
        let err = parse("a d_xy_ff 1 1\np d_param_cf a 2\nq d_param_cf p XA\n").unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("需要实数")));
    }

    #[test]
    fn label_tokens_round_trip() {
        let text = "a d_xy_ff 1 1 # label_hidden\nb d_xy_ff 2 2 # red label_visible\n";
        let p1 = parse(text).expect("解析成功");
        assert!(!p1.stmts[0].style.label_visible);
        assert!(p1.stmts[1].style.label_visible);
        let out = emit(&p1);
        // 非默认项才输出：a 输出 label_hidden；b 的 label_visible 是默认值，不输出
        assert!(out.contains("a d_xy_ff 1 1 # label_hidden\n"), "{out}");
        assert!(out.contains("b d_xy_ff 2 2 # red\n"), "{out}");
        let p2 = parse(&out).expect("再解析成功");
        assert_eq!(p1.stmts, p2.stmts, "往返后语句应一致");
    }

    #[test]
    fn unknown_attr_tokens_silently_ignored() {
        let program = parse("a d_xy_ff 1 1 # foo bar 2.5 baz\n").expect("解析成功");
        assert_eq!(program.stmts[0].style.width, 2.5); // 数字 token 生效
        // foo/bar/baz 静默忽略，不产生诊断
    }

    #[test]
    fn version_header_tolerated_when_missing() {
        let program = parse("a d_xy_ff 1 1\n").expect("无版本头也应成功");
        assert_eq!(program.stmts.len(), 1);
    }

    #[test]
    fn emit_contains_header_and_names() {
        let text = emit(&parse(SAMPLE).unwrap());
        assert!(text.starts_with("// gmk3 0.0.5\n"));
        // 属性按规范顺序输出（颜色 → 形状 → 粗细 → 可见性 → 标签 → 拖动）
        assert!(text.contains("d1 d_xy_ff 1 1 # navy dashed 2\n"));
        assert!(text.contains("d2 d_xy_ff 1 0 # red hollow 1.5 fixed\n"));
        assert!(text.contains("p d_param_cf c -3\n"));
        assert!(text.contains("x d_meet_ll l XA # label_hidden\n"));
        assert!(text.contains("y d_meet_ll l YA # ube dotted 2\n"));
    }

    #[test]
    fn default_attrs_not_emitted() {
        let text = emit(&parse("a d_xy_ff 1 1\nb l_join_dd a a\n").unwrap());
        assert!(!text.contains(" # "), "全默认属性不应输出：{text}");
    }
}
