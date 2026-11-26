// src/monkey/mod.rs
//! Monkey：中间表示的**主要类**，文本驱动几何构造的单一事实源。
//!
//! 每条语句 `名字 方法 实参… [# 属性…]` 固定产出且只产出一个对象；
//! 实参只能引用先前语句（解析器保证），因此程序是一条无环 SSA 构造序列，
//! 单遍解算即收敛。
//!
//! [`Monkey`] 的职责：
//! - 从 DSL 文本导入（[`Monkey::from_dsl`] / [`Monkey::import_dsl`]）、
//!   导出为 DSL 文本（[`Monkey::export_dsl`]）；
//! - 解算一帧（[`Monkey::solve`]）：产出几何数据，缓存在自身；
//! - 绘制一帧（[`Monkey::draw`]）：用缓存数据喂给 [`Plotter`]；
//! - 查询接口：按编号 / 名字读取解算值或样式值（缓存）。
//!
//! 内存中以 [`ObjectId`] 引用对象，[`Monkey`] 维护名字 ⇄ ID 双向查询。
//!
//! 依赖方向：`canvas ← monkey → pakoo_forest`；`app` / `gmk3_app` 只使用公开 API。

pub mod dsl;
pub mod kinds;
pub mod method;
pub mod solve;
pub mod stmt;
pub mod style;

use std::collections::HashMap;

use crate::monkey::kinds::{CONSTANTS, GeoKind};
use crate::monkey::method::MethodId;
use crate::monkey::solve::{EvalStatus, Solved};
use crate::monkey::stmt::{Arg, FIRST_USER_ID, Stmt};

pub use crate::monkey::stmt::ObjectId;

/// Monkey：构造语句序列 + 名字索引 + 解算缓存。
#[derive(Clone, Debug)]
pub struct Monkey {
    /// 按构造顺序排列的语句（事实源）。
    stmts: Vec<Stmt>,
    /// 名字 → ID（含内置常量名）。
    names: HashMap<String, ObjectId>,
    /// 下一个可分配的用户对象 ID。
    next_id: u64,
    /// 解算缓存（`solve` 后与 `stmts` 等长；帧内查询用）。
    solved: Vec<Solved>,
}

impl Default for Monkey {
    fn default() -> Self {
        Self::new()
    }
}

impl Monkey {
    /// 空程序：内置常量（`XA YA INF I J`）占用 ID `0..5`，用户对象从 5 起。
    pub fn new() -> Self {
        let mut names = HashMap::with_capacity(16);
        for (i, (name, _)) in CONSTANTS.iter().enumerate() {
            names.insert((*name).to_string(), ObjectId(i as u64));
        }
        Self {
            stmts: Vec::new(),
            names,
            next_id: FIRST_USER_ID,
            solved: Vec::new(),
        }
    }

    /// 由 DSL 文本构造。
    pub fn from_dsl(text: &str) -> Result<Self, Vec<dsl::Diagnostic>> {
        dsl::parse(text)
    }

    /// 用 DSL 文本整体替换当前程序（保留对象）。
    pub fn import_dsl(&mut self, text: &str) -> Result<(), Vec<dsl::Diagnostic>> {
        let parsed = dsl::parse(text)?;
        *self = parsed;
        Ok(())
    }

    /// 导出为 DSL 文本（含版本头）。
    pub fn export_dsl(&self) -> String {
        dsl::emit(self)
    }

    /// 解算一帧：顺序执行全部语句，结果缓存在自身。
    pub fn solve(&mut self) {
        solve::solve(self);
    }

    /// 绘制一帧：清空绘图器并提交本帧解算出的几何。
    ///
    /// 必须先调用 [`Monkey::solve`]（否则不提交任何内容）。
    pub fn draw(&self, plotter: &mut crate::canvas::plotter::Plotter, dark: bool) {
        plotter.clear();
        self.draw_into(plotter, dark);
    }

    /// 仅提交几何（不清空绘图器），供外部已清空缓冲的绘图器使用。
    pub fn draw_into(&self, plotter: &mut crate::canvas::plotter::Plotter, dark: bool) {
        solve::draw_into(self, plotter, dark);
    }

    /// 生成对象名标签（本轮仅点），与坐标轴标签合并进同一缓冲。
    pub fn collect_labels(
        &self,
        plotter: &crate::canvas::plotter::Plotter,
        viewport_px: [f32; 2],
        dark: bool,
        out: &mut Vec<crate::canvas::labels::CanvasLabel>,
    ) {
        solve::collect_labels(self, plotter, viewport_px, dark, out);
    }

    // ------------------------------------------------------------------
    // 查询接口（读缓存 / 事实源）
    // ------------------------------------------------------------------

    /// 追加一条语句。名字必须唯一（与常量 / 保留字 / 已有名字不冲突）。
    ///
    /// 实参引用需已定义（调用方负责，解析器已校验）。
    pub fn push(
        &mut self,
        name: String,
        method: MethodId,
        args: Vec<Arg>,
        style: style::StyleSpec,
    ) -> Result<ObjectId, &'static str> {
        if self.names.contains_key(&name) {
            return Err("名字已存在");
        }
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        self.names.insert(name.clone(), id);
        self.stmts.push(Stmt {
            id,
            name,
            method,
            args,
            style,
            selected: false,
        });
        Ok(id)
    }

    /// 名字 → ID（含内置常量）。
    pub fn id_of(&self, name: &str) -> Option<ObjectId> {
        self.names.get(name).copied()
    }

    /// ID → 名字（内置常量名或语句名）。
    pub fn name_of(&self, id: ObjectId) -> &str {
        if (id.0 as usize) < CONSTANTS.len() {
            CONSTANTS[id.0 as usize].0
        } else {
            &self.stmts[(id.0 - FIRST_USER_ID) as usize].name
        }
    }

    /// ID → 语句（内置常量返回 `None`）。
    pub fn stmt_of(&self, id: ObjectId) -> Option<&Stmt> {
        if (id.0 as usize) < CONSTANTS.len() {
            return None;
        }
        self.stmts.get((id.0 - FIRST_USER_ID) as usize)
    }

    /// ID → 解算值（几何对象；内置常量总是有效，用户对象须已 `solve`）。
    pub fn value_of(&self, id: ObjectId) -> Option<&crate::monkey::kinds::GeoObject> {
        if (id.0 as usize) < CONSTANTS.len() {
            return Some(&CONSTANTS[id.0 as usize].1);
        }
        self.solved
            .get((id.0 - FIRST_USER_ID) as usize)
            .and_then(|s| s.object.as_ref())
    }

    /// ID → 解算状态（内置常量返回 `None`）。
    pub fn status_of(&self, id: ObjectId) -> Option<&EvalStatus> {
        if (id.0 as usize) < CONSTANTS.len() {
            return None;
        }
        self.solved
            .get((id.0 - FIRST_USER_ID) as usize)
            .map(|s| &s.status)
    }

    /// ID → 绘图属性（内置常量返回 `None`）。
    pub fn style_of(&self, id: ObjectId) -> Option<&style::StyleSpec> {
        self.stmt_of(id).map(|s| &s.style)
    }

    /// 全部语句（对象列表等 UI 直接消费）。
    pub fn stmts(&self) -> &[Stmt] {
        &self.stmts
    }

    /// 语句数量（不含内置常量）。
    pub fn len(&self) -> usize {
        self.stmts.len()
    }

    /// 是否无语句。
    pub fn is_empty(&self) -> bool {
        self.stmts.is_empty()
    }

    /// 设置对象选中态（运行时 UI 状态，不进 DSL）。
    pub fn set_selected(&mut self, id: ObjectId, selected: bool) {
        if (id.0 as usize) >= CONSTANTS.len() {
            if let Some(stmt) = self.stmts.get_mut((id.0 - FIRST_USER_ID) as usize) {
                stmt.selected = selected;
            }
        }
    }

    // ------------------------------------------------------------------
    // crate 内部（dsl / solve 使用）
    // ------------------------------------------------------------------

    pub(crate) fn contains_name(&self, name: &str) -> bool {
        self.names.contains_key(name)
    }

    pub(crate) fn kind_of(&self, id: ObjectId) -> GeoKind {
        if (id.0 as usize) < CONSTANTS.len() {
            CONSTANTS[id.0 as usize].1.kind()
        } else {
            self.stmts[(id.0 - FIRST_USER_ID) as usize].method.def().out
        }
    }
}
