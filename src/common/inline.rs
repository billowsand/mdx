//! 行内格式拆分：把一行原文切成 Inline 序列（Text / Bold / Italic）。
//!
//! 仅识别 markdown 的 `**加粗**` 与 `*斜体*` 两种语法；其他符号原样进入 Text。
//! 与 md_to_docx_rust::process_text_formatting 的拆分规则一致：
//! - `**...**` 至少 4 字符长才视作粗体
//! - `*...*` 至少 2 字符长才视作斜体（且不会被 `**` 误吞）

use regex::Regex;
use std::sync::OnceLock;

use super::ast::Inline;

fn inline_matcher() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(`[^`]+`|\*\*[^*]+\*\*|\*[^*]+\*)").expect("invalid inline regex")
    })
}

fn link_matcher() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").expect("invalid link regex"))
}

/// 把一段已正规化引号的纯文本拆成 Inline 列表。
///
/// 空字符串返回空向量。
pub fn parse(text: &str) -> Vec<Inline> {
    if text.is_empty() {
        return Vec::new();
    }

    // 优先处理链接（链接不嵌套）
    let mut out = Vec::new();
    let link_re = link_matcher();
    let mut last_end = 0;

    for m in link_re.find_iter(text) {
        if m.start() > last_end {
            // 处理链接之前的部分
            let before = &text[last_end..m.start()];
            let inlines = parse_no_links(before);
            out.extend(inlines);
        }
        // 处理链接
        let link_text = &text[m.start() + 1..m.end() - 1];
        if let Some(sep) = link_text.find("](") {
            let text_part = &link_text[..sep];
            let url_part = &link_text[sep + 2..];
            out.push(Inline::Link {
                text: text_part.to_string(),
                url: url_part.to_string(),
            });
        } else {
            out.push(Inline::Text(m.as_str().to_string()));
        }
        last_end = m.end();
    }

    if last_end < text.len() {
        let remaining = &text[last_end..];
        let inlines = parse_no_links(remaining);
        out.extend(inlines);
    }

    out
}

/// 解析不含链接的文本（处理加粗、斜体、代码）
fn parse_no_links(text: &str) -> Vec<Inline> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let re = inline_matcher();
    let mut last_end = 0;

    for m in re.find_iter(text) {
        if m.start() > last_end {
            out.push(Inline::Text(text[last_end..m.start()].to_string()));
        }
        let part = m.as_str();
        if part.starts_with("**") && part.ends_with("**") && part.len() >= 4 {
            out.push(Inline::Bold(part[2..part.len() - 2].to_string()));
        } else if part.starts_with('*')
            && part.ends_with('*')
            && part.len() >= 2
            && !part.starts_with("**")
        {
            out.push(Inline::Italic(part[1..part.len() - 1].to_string()));
        } else if part.starts_with('`') && part.ends_with('`') && part.len() >= 2 {
            out.push(Inline::Code(part[1..part.len() - 1].to_string()));
        } else {
            out.push(Inline::Text(part.to_string()));
        }
        last_end = m.end();
    }

    if last_end < text.len() {
        out.push(Inline::Text(text[last_end..].to_string()));
    }
    out
}

/// 把 Inline 序列拼回纯字符串（emitter 在不需要格式时使用，比如表格 cell 简化）。
#[allow(dead_code)]
pub fn flatten(inlines: &[Inline]) -> String {
    let mut s = String::new();
    for ip in inlines {
        match ip {
            Inline::Text(t) | Inline::Bold(t) | Inline::Italic(t) | Inline::Code(t) => {
                s.push_str(t)
            }
            Inline::Link { text, .. } => s.push_str(text),
        }
    }
    s
}
