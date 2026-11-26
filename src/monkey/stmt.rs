// src/monkey/stmt.rs
//! Monkey 语句：对象唯一 ID、实参与构造语句。

use crate::monkey::kinds::CONSTANTS;
use crate::monkey::method::MethodId;
use crate::monkey::style::StyleSpec;

/// 对象唯一 ID。
///
/// `0..CONSTANTS.len()` 保留给内置常量，用户语句从 [`FIRST_USER_ID`] 起单调分配。
/// DSL 文本只按名字引用对象，名字解析为 ID 后内存中一律以 ID 引用。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ObjectId(pub u64);

/// 首个用户对象 ID。
pub const FIRST_USER_ID: u64 = CONSTANTS.len() as u64;

/// 实参：对象引用或实数字面量。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Arg {
    /// 引用先前语句或内置常量。
    Id(ObjectId),
    /// 实数字面量（f64）。
    Lit(f64),
}

/// 一条构造语句：`名字 = 方法(实参…)`，附绘图属性。
///
/// 固定形式、单条语句只产出一个对象；实参只允许引用先前语句（由解析器保证），
/// 因此整个程序是一条无环的 SSA 构造序列。
#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub id: ObjectId,
    pub name: String,
    pub method: MethodId,
    pub args: Vec<Arg>,
    /// 绘图属性（可序列化，进 DSL）。
    pub style: StyleSpec,
    /// 选中态：运行时 UI 状态，**不进 DSL**（导入时复位为 `false`）。
    pub selected: bool,
}
