//! 公共工具：被 parser、tex_official、docx_research 共享的纯逻辑。
//!
//! docx_official 第一版未接入 common（保持与原 md_to_docx_rust 的字节一致性）；
//! 后续阶段确认无回归后再迁移。
//!
//! 阶段 2 落库时，emitter（公文 tex / 研报 docx）尚未连接 common；用 `allow(dead_code)`
//! 抑制 unused 警告，等阶段 3/4 接通后这些函数会自然被引用。

#![allow(dead_code)]

pub mod ast;
pub mod crossref;
pub mod heading;
pub mod images;
pub mod inline;
pub mod markers;
pub mod numbering;
pub mod parts;
pub mod quotes;
pub mod table;
pub mod table_to_longtblr;
