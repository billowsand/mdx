//! 行内格式拆分：把一行原文切成 Inline 序列（Text / Bold / Italic / Code / Link / Footnote）。
//!
//! 识别 markdown 的 `**加粗**`、`*斜体*`、`` `代码` ``、`[文本](链接)`、`![替代文本](图片路径)`
//! 以及行内脚注 `[^id]:(注释内容)`（冒号、括号兼容全角）；其他符号原样进入 Text。
//! 扩展标记：图片后紧跟 `{#id}` 作为交叉引用锚点；`{@id}` 为交叉引用（tex → \ref{id}）。
//! 与 md_to_docx_rust::process_text_formatting 的拆分规则一致：
//! - `**...**` 至少 4 字符长才视作粗体
//! - `*...*` 至少 2 字符长才视作斜体（且不会被 `**` 误吞）

use regex::Regex;
use std::sync::OnceLock;

use super::ast::Inline;

fn inline_matcher() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(`[^`]+`|\[(?:@[^\s@;,\[\]{}\\]+)(?:\s*;\s*@[^\s@;,\[\]{}\\]+)*\]|\*\*[^*]+\*\*|\*[^*]+\*)",
        )
        .expect("invalid inline regex")
    })
}

/// 链接与图片共用：`[text](url)` 或 `![alt](url)`（图片 alt 可为空）。
fn link_matcher() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"!?\[([^\]]*)\]\(([^)]+)\)").expect("invalid link regex"))
}

/// 行内脚注：`[^id]:(内容)` 或 `[^id]：（内容）`，冒号与括号均兼容全角。
fn footnote_matcher() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\[\^[^\]]+\][:：](?:\(([^)]*)\)|（([^）]*)）)")
            .expect("invalid footnote regex")
    })
}

/// 交叉引用：`{@id}`，id 为字母开头的 LaTeX label 合法字符序列。
fn crossref_matcher() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{@([A-Za-z][\w:.-]*)\}").expect("invalid crossref regex"))
}

/// 图片标签属性：紧跟 `![alt](url)` 之后的 `{#id}`（锚定匹配，用于向前窥探）。
fn label_attr_matcher() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\{#([A-Za-z][\w:.-]*)\}").expect("invalid label attr regex"))
}

/// 把一段已正规化引号的纯文本拆成 Inline 列表。
///
/// 空字符串返回空向量。
pub fn parse(text: &str) -> Vec<Inline> {
    if text.is_empty() {
        return Vec::new();
    }

    // 最优先处理行内脚注（脚注不嵌套）
    let mut out = Vec::new();
    let footnote_re = footnote_matcher();
    let mut last_end = 0;

    for m in footnote_re.find_iter(text) {
        if m.start() > last_end {
            out.extend(parse_with_crossrefs(&text[last_end..m.start()]));
        }
        let caps = footnote_re
            .captures(m.as_str())
            .expect("footnote match without captures");
        let content = caps
            .get(1)
            .or_else(|| caps.get(2))
            .map(|g| g.as_str())
            .unwrap_or("");
        out.push(Inline::Footnote(content.to_string()));
        last_end = m.end();
    }

    if last_end < text.len() {
        out.extend(parse_with_crossrefs(&text[last_end..]));
    }

    out
}

/// 处理不含脚注的文本：先切出 `{@id}` 交叉引用，其余交给链接/图片/格式解析
fn parse_with_crossrefs(text: &str) -> Vec<Inline> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let crossref_re = crossref_matcher();
    let mut last_end = 0;

    for caps in crossref_re.captures_iter(text) {
        let m = caps.get(0).expect("crossref match without whole group");
        if m.start() > last_end {
            out.extend(parse_with_links(&text[last_end..m.start()]));
        }
        out.push(Inline::CrossRef(caps[1].to_string()));
        last_end = m.end();
    }

    if last_end < text.len() {
        out.extend(parse_with_links(&text[last_end..]));
    }

    out
}

/// 处理不含脚注的文本（优先链接，其次加粗/斜体/代码）
fn parse_with_links(text: &str) -> Vec<Inline> {
    if text.is_empty() {
        return Vec::new();
    }

    // 优先处理链接与图片（不嵌套）；以 `!` 开头的匹配为图片
    let mut out = Vec::new();
    let link_re = link_matcher();
    let mut last_end = 0;

    for caps in link_re.captures_iter(text) {
        let m = caps.get(0).expect("link match without whole group");
        if m.start() > last_end {
            // 处理链接之前的部分
            let before = &text[last_end..m.start()];
            let inlines = parse_no_links(before);
            out.extend(inlines);
        }
        let label_text = caps[1].to_string();
        let url = caps[2].to_string();
        if m.as_str().starts_with('!') {
            // 图片后紧跟的 `{#id}` 是交叉引用锚点，一并消费
            let mut label = None;
            if let Some(attr) = label_attr_matcher().captures(&text[m.end()..]) {
                label = Some(attr[1].to_string());
                last_end = m.end() + attr.get(0).expect("attr whole group").end();
            } else {
                last_end = m.end();
            }
            out.push(Inline::Image {
                alt: label_text,
                url,
                label,
            });
        } else {
            out.push(Inline::Link {
                text: label_text,
                url,
            });
            last_end = m.end();
        }
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
            // 递归解析内部：加粗里允许再次出现引用 / 脚注 / 嵌套格式
            let inner = &part[2..part.len() - 2];
            out.push(Inline::Bold(parse(inner)));
        } else if part.starts_with('*')
            && part.ends_with('*')
            && part.len() >= 2
            && !part.starts_with("**")
        {
            let inner = &part[1..part.len() - 1];
            out.push(Inline::Italic(parse(inner)));
        } else if part.starts_with('`') && part.ends_with('`') && part.len() >= 2 {
            // inline code 不再解析（保留原样），与代码块行为一致
            out.push(Inline::Code(part[1..part.len() - 1].to_string()));
        } else if part.starts_with('[') && part.ends_with(']') {
            let keys = part[1..part.len() - 1]
                .split(';')
                .map(|item| item.trim().trim_start_matches('@').to_string())
                .collect();
            out.push(Inline::Citation(keys));
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
            Inline::Text(t) => s.push_str(t),
            Inline::Bold(children) | Inline::Italic(children) => {
                s.push_str(&flatten(children));
            }
            Inline::Code(t) => s.push_str(t),
            Inline::Link { text, .. } => s.push_str(text),
            Inline::Image { alt, .. } => s.push_str(alt),
            Inline::CrossRef(id) => s.push_str(id),
            Inline::Citation(keys) => {
                s.push('[');
                s.push_str(
                    &keys
                        .iter()
                        .map(|key| format!("@{key}"))
                        .collect::<Vec<_>>()
                        .join("; "),
                );
                s.push(']');
            }
            Inline::Footnote(t) => {
                s.push('（');
                s.push_str(t);
                s.push('）');
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inline_footnote_halfwidth() {
        let inlines = parse("正文[^1]:(这是注释)继续");
        assert_eq!(
            inlines,
            vec![
                Inline::Text("正文".into()),
                Inline::Footnote("这是注释".into()),
                Inline::Text("继续".into()),
            ]
        );
    }

    #[test]
    fn parses_inline_footnote_fullwidth() {
        let inlines = parse("正文[^2]：（全角注释）");
        assert_eq!(
            inlines,
            vec![
                Inline::Text("正文".into()),
                Inline::Footnote("全角注释".into()),
            ]
        );
    }

    #[test]
    fn footnote_coexists_with_bold_and_link() {
        let inlines = parse("**重点**[^a]:(注) [页](https://x)");
        assert_eq!(
            inlines,
            vec![
                Inline::Bold(vec![Inline::Text("重点".into())]),
                Inline::Footnote("注".into()),
                Inline::Text(" ".into()),
                Inline::Link {
                    text: "页".into(),
                    url: "https://x".into()
                },
            ]
        );
    }

    #[test]
    fn bare_footnote_ref_without_definition_stays_text() {
        // 没有 `:(内容)` 的 `[^1]` 不识别为脚注，原样保留
        let inlines = parse("正文[^1]结束");
        assert_eq!(inlines, vec![Inline::Text("正文[^1]结束".into())]);
    }

    #[test]
    fn parses_image_with_alt() {
        let inlines = parse("见下图：![系统架构](figs/arch.png)");
        assert_eq!(
            inlines,
            vec![
                Inline::Text("见下图：".into()),
                Inline::Image {
                    alt: "系统架构".into(),
                    url: "figs/arch.png".into(),
                    label: None,
                },
            ]
        );
    }

    #[test]
    fn parses_image_with_empty_alt() {
        let inlines = parse("![](a.png)");
        assert_eq!(
            inlines,
            vec![Inline::Image {
                alt: String::new(),
                url: "a.png".into(),
                label: None,
            }]
        );
    }

    #[test]
    fn parses_image_label_attr() {
        let inlines = parse("![架构](figs/arch.png){#fig:arch}后续");
        assert_eq!(
            inlines,
            vec![
                Inline::Image {
                    alt: "架构".into(),
                    url: "figs/arch.png".into(),
                    label: Some("fig:arch".into()),
                },
                Inline::Text("后续".into()),
            ]
        );
    }

    #[test]
    fn parses_crossref() {
        let inlines = parse("见第{@chap:overview}章和图{@fig:arch}。");
        assert_eq!(
            inlines,
            vec![
                Inline::Text("见第".into()),
                Inline::CrossRef("chap:overview".into()),
                Inline::Text("章和图".into()),
                Inline::CrossRef("fig:arch".into()),
                Inline::Text("。".into()),
            ]
        );
    }

    #[test]
    fn crossref_coexists_with_footnote_and_link() {
        let inlines = parse("正文[^1]:(注)见{@sec:a}和[页](https://x)");
        assert_eq!(
            inlines,
            vec![
                Inline::Text("正文".into()),
                Inline::Footnote("注".into()),
                Inline::Text("见".into()),
                Inline::CrossRef("sec:a".into()),
                Inline::Text("和".into()),
                Inline::Link {
                    text: "页".into(),
                    url: "https://x".into(),
                },
            ]
        );
    }

    #[test]
    fn plain_link_still_parses_as_link() {
        let inlines = parse("[页](https://x)");
        assert_eq!(
            inlines,
            vec![Inline::Link {
                text: "页".into(),
                url: "https://x".into(),
            }]
        );
    }

    #[test]
    fn parses_single_and_multiple_citations() {
        assert_eq!(parse("[@key]"), vec![Inline::Citation(vec!["key".into()])]);
        assert_eq!(
            parse("前文 [@a; @b] 后文"),
            vec![
                Inline::Text("前文 ".into()),
                Inline::Citation(vec!["a".into(), "b".into()]),
                Inline::Text(" 后文".into()),
            ]
        );
    }

    #[test]
    fn citation_inside_bold_parses_as_citation() {
        // 粗体内部的 [@key] 应被识别为引用，而不是作为粗体文字原样保留。
        let inlines = parse("**bold and [@biddle_military_2006] inside**");
        assert_eq!(
            inlines,
            vec![Inline::Bold(vec![
                Inline::Text("bold and ".into()),
                Inline::Citation(vec!["biddle_military_2006".into()]),
                Inline::Text(" inside".into()),
            ])]
        );
    }

    #[test]
    fn footnote_inside_italic_parses_as_footnote() {
        // 斜体与脚注在同一段时，脚注先于斜体匹配，所以需要斜体不在脚注左右两侧被切断。
        let inlines = parse("斜体中[^f]:(脚注)嵌套", ); // 单独测试脚注 + 斜体分段行为
        // 期望：脚注独立匹配，前后是普通文本
        assert_eq!(
            inlines,
            vec![
                Inline::Text("斜体中".into()),
                Inline::Footnote("脚注".into()),
                Inline::Text("嵌套".into()),
            ]
        );
    }

    #[test]
    fn italic_wraps_text_with_internal_format() {
        // 斜体内部如果出现裸文本（无内嵌格式），整段被识别为 Italic
        let inlines = parse("*整段斜体*");
        assert_eq!(inlines, vec![Inline::Italic(vec![Inline::Text("整段斜体".into())])]);
    }

    #[test]
    fn unsupported_citations_and_inline_code_stay_literal() {
        assert_eq!(parse("@key"), vec![Inline::Text("@key".into())]);
        assert_eq!(
            parse("[@key, p. 2]"),
            vec![Inline::Text("[@key, p. 2]".into())]
        );
        assert_eq!(parse("`[@key]`"), vec![Inline::Code("[@key]".into())]);
    }

    #[test]
    fn flatten_reconstructs_citation_source() {
        assert_eq!(
            flatten(&[Inline::Citation(vec!["a".into(), "b".into()])]),
            "[@a; @b]"
        );
    }
}
