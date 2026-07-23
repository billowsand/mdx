//! 公文 → tex pipeline。
//!
//! 直接拼 LaTeX 字符串，不调 pandoc。视觉与 docx_official 一致：
//! - 正文 仿宋 三号（zihao{3}）、1.5 倍行距、2em 首行缩进
//! - 标题 H1 居中 方正小标宋简体 二号；H2 黑体 + "一、"；H3 楷体 + "（一）"；
//!   H4 仿宋 + "1."；H5 仿宋粗体 + "(1)"
//! - 列表前缀循环 ①②③ → ⑴⑵⑶ → a.b.c. → I.II.III. → (A)(B) → 1)2)
//! - 表格简单 tabular 全边框
//!
//! 内嵌 `official.cls`，运行时复制到输出目录，供 `xelatex` 编译用。

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::common::ast::{Block, Inline};
use crate::common::numbering::{int_to_roman, number_to_chinese, number_to_uppercase_letter};
use crate::parser;

const OFFICIAL_CLS: &str = include_str!("../resources/official/official.cls");

const CIRCLE_NUMBERS_1: &[&str] = &[
    "⑴", "⑵", "⑶", "⑷", "⑸", "⑹", "⑺", "⑻", "⑼", "⑽", "⑾", "⑿", "⒀", "⒁", "⒂", "⒃", "⒄", "⒅", "⒆",
    "⒇",
];
const CIRCLE_NUMBERS_2: &[&str] = &[
    "①", "②", "③", "④", "⑤", "⑥", "⑦", "⑧", "⑨", "⑩", "⑪", "⑫", "⑬", "⑭", "⑮", "⑯", "⑰", "⑱", "⑲",
    "⑳",
];

/// 公文 tex 入口。
///
/// 输出布局：主 tex 文件 + `data/` 下的分节部件（主文件 `\input` 引用）+
/// `figures/` 下的图片（markdown 引用的本地图片复制至此并改写路径）。
pub fn run(input: &Path, output: Option<&Path>) -> Result<()> {
    let kind = crate::input::classify(input)?;
    let content = crate::input::collect(input)?;
    let output_path = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::input::default_output(input, "tex"));

    println!("正在转换: {}", input.display());

    let out_dir = output_path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(out_dir).with_context(|| format!("创建输出目录 {} 失败", out_dir.display()))?;
    let base_dir = match kind {
        crate::input::InputKind::Directory => input,
        crate::input::InputKind::File => input.parent().unwrap_or(Path::new(".")),
    };

    let mut blocks = parser::parse(&content);
    for (_, dst) in crate::common::images::relocate(&mut blocks, base_dir, out_dir) {
        println!("  图片已复制到 {}", dst.display());
    }

    let mut emitter = TexEmitter::new();
    emitter.emit_all(&blocks);

    let (body, parts) = emitter.finish();
    let tex = wrap_document(&body);

    fs::write(&output_path, tex).with_context(|| format!("写入 {} 失败", output_path.display()))?;
    crate::common::parts::write_parts(out_dir, &parts)
        .with_context(|| format!("写入分章部件到 {} 失败", out_dir.display()))?;

    let cls_dst = out_dir.join("official.cls");
    fs::write(&cls_dst, OFFICIAL_CLS)
        .with_context(|| format!("复制 official.cls 到 {} 失败", cls_dst.display()))?;
    println!("  已复制 official.cls 到 {}", cls_dst.display());

    crate::tex_compile::compile_pdf_if_available(&output_path)?;

    println!("[完成] 转换完成: {}", output_path.display());
    Ok(())
}

fn wrap_document(body: &str) -> String {
    let mut s = String::new();
    s.push_str("\\documentclass{official}\n\n");
    s.push_str("\\begin{document}\n\n");
    s.push_str(body);
    if !body.ends_with('\n') {
        s.push('\n');
    }
    s.push_str("\n\\end{document}\n");
    s
}

// ========== Emitter ==========

struct TexEmitter {
    /// 当前写入缓冲区（主文件段或当前部件段）
    out: String,
    /// 已定稿的主文件内容（含 \input 引用行）
    main: String,
    /// 已定稿的部件：(相对路径, 内容)
    parts: Vec<(String, String)>,
    /// 当前正在写入的部件路径；None 表示当前写入主文件
    current_part: Option<String>,
    /// data/sectionNN 序号
    data_idx: usize,
    h2: usize,
    h3: usize,
    h4: usize,
    h5: usize,
    l1: usize,
    l2: usize,
    l3: usize,
    l4: usize,
    l5: usize,
    l6: usize,
    in_list: bool,
    list_level: u8,
}

impl TexEmitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            main: String::new(),
            parts: Vec::new(),
            current_part: None,
            data_idx: 0,
            h2: 0,
            h3: 0,
            h4: 0,
            h5: 0,
            l1: 0,
            l2: 0,
            l3: 0,
            l4: 0,
            l5: 0,
            l6: 0,
            in_list: false,
            list_level: 0,
        }
    }

    /// 收尾：把当前缓冲区定稿，返回 (主文件 body, 部件列表)。
    fn finish(mut self) -> (String, Vec<(String, String)>) {
        self.finalize_current();
        (self.main, self.parts)
    }

    /// 当前缓冲区定稿：归入当前部件或主文件。
    fn finalize_current(&mut self) {
        let buf = std::mem::take(&mut self.out);
        match self.current_part.take() {
            Some(path) => self.parts.push((path, buf)),
            None => self.main.push_str(&buf),
        }
    }

    /// 切出一个新部件；主文件中按序留下 \input 引用。
    fn start_data_part(&mut self) {
        self.finalize_current();
        self.data_idx += 1;
        let path = format!("data/section{:02}.tex", self.data_idx);
        self.main.push_str(&format!("\\input{{{}}}\n\n", path));
        self.current_part = Some(path);
    }

    fn emit_all(&mut self, blocks: &[Block]) {
        for b in blocks {
            self.emit_block(b);
        }
    }

    fn emit_block(&mut self, block: &Block) {
        match block {
            Block::Heading { level, text } => {
                self.reset_list();
                self.emit_heading(*level, text);
            }
            Block::Paragraph(inlines) => {
                self.reset_list();
                self.emit_paragraph(inlines);
            }
            Block::List {
                ordered: _,
                level,
                content,
            } => {
                self.emit_list_item(*level, content);
            }
            Block::Table { rows, .. } => {
                self.reset_list();
                self.emit_table(rows);
            }
            Block::Marker(_) => {
                // 公文路径不响应区段标记，原样忽略
            }
            Block::CodeBlock { .. } => {
                // 公文路径暂不支持代码块
            }
            Block::Empty => {
                // 空行：不主动产出多余空行；段落分隔已由其他 emit_* 末尾的 "\n\n" 处理
            }
        }
    }

    fn emit_heading(&mut self, level: u8, text: &str) {
        let escaped = escape_latex(text);
        match level {
            1 => {
                self.h2 = 0;
                self.h3 = 0;
                self.h4 = 0;
                self.h5 = 0;
                self.out.push_str(&format!(
                    "\\begin{{center}}{{\\xbsong\\zihao{{2}}{}}}\\end{{center}}\n\n",
                    escaped
                ));
            }
            2 => {
                self.h2 += 1;
                self.h3 = 0;
                self.h4 = 0;
                self.h5 = 0;
                // H2（"一、"级）是公文的顶级节，每节切出 data/ 部件
                self.start_data_part();
                let num = number_to_chinese(self.h2);
                self.out
                    .push_str(&format!("{{\\hei {}、{}}}\\par\n\n", num, escaped));
            }
            3 => {
                self.h3 += 1;
                self.h4 = 0;
                self.h5 = 0;
                let num = number_to_chinese(self.h3);
                self.out
                    .push_str(&format!("{{\\kai （{}）{}}}\\par\n\n", num, escaped));
            }
            4 => {
                self.h4 += 1;
                self.h5 = 0;
                self.out
                    .push_str(&format!("{}.{}\\par\n\n", self.h4, escaped));
            }
            5 => {
                self.h5 += 1;
                self.out
                    .push_str(&format!("\\textbf{{({}){}}}\\par\n\n", self.h5, escaped));
            }
            _ => {
                // 6 级以上原 docx 也直接忽略
            }
        }
    }

    fn emit_paragraph(&mut self, inlines: &[Inline]) {
        // 独占一段的图片（允许前后有空白文本）输出为 figure 环境
        if let Some((alt, url)) = sole_image(inlines) {
            self.emit_figure(alt, url);
            return;
        }
        let body = render_inlines(inlines);
        if body.trim().is_empty() {
            return;
        }
        self.out.push_str(&body);
        self.out.push_str("\\par\n\n");
    }

    fn emit_figure(&mut self, alt: &str, url: &str) {
        self.out.push_str("\\begin{figure}[htbp]\n\\centering\n");
        self.out.push_str(&format!(
            "\\includegraphics[width=\\textwidth]{{{}}}\n",
            escape_href_url(url)
        ));
        if !alt.is_empty() {
            self.out
                .push_str(&format!("\\caption{{{}}}\n", escape_latex(alt)));
        }
        self.out.push_str("\\end{figure}\n\n");
    }

    fn emit_list_item(&mut self, level: u8, content: &[Inline]) {
        let prefix = self.list_prefix(level);
        let body = render_inlines(content);
        // 首行缩进交给 cls（parindent=2em）；列表项作为普通段落
        self.out.push_str(&format!("{}{}\\par\n\n", prefix, body));
        self.in_list = true;
        self.list_level = level;
    }

    fn emit_table(&mut self, rows: &[Vec<String>]) {
        if rows.is_empty() {
            return;
        }
        let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if max_cols == 0 {
            return;
        }
        let col_spec: String = std::iter::repeat('l')
            .take(max_cols)
            .map(|c| format!("|{}", c))
            .collect::<String>()
            + "|";

        self.out.push_str("\\begin{center}\n");
        self.out
            .push_str(&format!("\\begin{{tabular}}{{{}}}\n\\hline\n", col_spec));

        for (row_idx, row) in rows.iter().enumerate() {
            let cells: Vec<String> = (0..max_cols)
                .map(|i| {
                    let raw = row.get(i).map(String::as_str).unwrap_or("");
                    // tabular 内 \footnote 的注释文字不会输出，降级为全角括号内联注释
                    let inlines: Vec<Inline> = crate::common::inline::parse(raw)
                        .into_iter()
                        .map(|ip| match ip {
                            Inline::Footnote(t) => Inline::Text(format!("（{}）", t)),
                            other => other,
                        })
                        .collect();
                    let body = render_inlines(&inlines);
                    if row_idx == 0 {
                        format!("\\textbf{{{}}}", body)
                    } else {
                        body
                    }
                })
                .collect();
            self.out.push_str(&cells.join(" & "));
            self.out.push_str(" \\\\\n\\hline\n");
        }

        self.out.push_str("\\end{tabular}\n\\end{center}\n\n");
    }

    fn reset_list(&mut self) {
        self.in_list = false;
        self.list_level = 0;
        self.l1 = 0;
        self.l2 = 0;
        self.l3 = 0;
        self.l4 = 0;
        self.l5 = 0;
        self.l6 = 0;
    }

    /// 镜像 docx_official::Converter::get_list_prefix 的行为，确保 6 级前缀循环
    /// 在 tex 输出中和 docx 完全一致。
    fn list_prefix(&mut self, level: u8) -> String {
        match level {
            1 => {
                if !self.in_list || self.list_level > 1 {
                    self.l2 = 0;
                    self.l3 = 0;
                    self.l4 = 0;
                    self.l5 = 0;
                    self.l6 = 0;
                }
                if !self.in_list {
                    self.l1 = 0;
                }
                self.l1 += 1;
                if self.l1 <= CIRCLE_NUMBERS_2.len() {
                    CIRCLE_NUMBERS_2[self.l1 - 1].to_string()
                } else {
                    format!("({})", self.l1)
                }
            }
            2 => {
                if !self.in_list || self.list_level != 2 {
                    if self.list_level > 2 {
                        self.l3 = 0;
                        self.l4 = 0;
                        self.l5 = 0;
                        self.l6 = 0;
                    }
                    if !self.in_list || self.list_level < 2 {
                        self.l2 = 0;
                        self.l3 = 0;
                        self.l4 = 0;
                        self.l5 = 0;
                        self.l6 = 0;
                    }
                }
                self.l2 += 1;
                if self.l2 <= CIRCLE_NUMBERS_1.len() {
                    CIRCLE_NUMBERS_1[self.l2 - 1].to_string()
                } else {
                    format!("({})", self.l2)
                }
            }
            3 => {
                if !self.in_list || self.list_level < 3 {
                    self.l3 = 0;
                    self.l4 = 0;
                    self.l5 = 0;
                    self.l6 = 0;
                }
                self.l3 += 1;
                let ch = (b'a' + ((self.l3 - 1) % 26) as u8) as char;
                format!("{}.", ch)
            }
            4 => {
                if !self.in_list || self.list_level < 4 {
                    self.l4 = 0;
                    self.l5 = 0;
                    self.l6 = 0;
                }
                self.l4 += 1;
                format!("{}.", int_to_roman(self.l4))
            }
            5 => {
                if !self.in_list || self.list_level < 5 {
                    self.l5 = 0;
                    self.l6 = 0;
                }
                self.l5 += 1;
                format!("({})", number_to_uppercase_letter(self.l5))
            }
            6 => {
                if !self.in_list || self.list_level < 6 {
                    self.l6 = 0;
                }
                self.l6 += 1;
                format!("{})", self.l6)
            }
            _ => String::new(),
        }
    }
}

// ========== 字符串工具 ==========

/// 段落内容去掉纯空白 Text 后只剩一张图片时，返回其 (alt, url)。
fn sole_image(inlines: &[Inline]) -> Option<(&str, &str)> {
    let meaningful: Vec<&Inline> = inlines
        .iter()
        .filter(|ip| !matches!(ip, Inline::Text(t) if t.trim().is_empty()))
        .collect();
    match meaningful.as_slice() {
        [Inline::Image { alt, url }] => Some((alt, url)),
        _ => None,
    }
}

fn render_inlines(inlines: &[Inline]) -> String {
    let mut s = String::new();
    for ip in inlines {
        match ip {
            Inline::Text(t) => s.push_str(&escape_latex(t)),
            Inline::Bold(t) => {
                s.push_str("\\textbf{");
                s.push_str(&escape_latex(t));
                s.push('}');
            }
            Inline::Italic(t) => {
                s.push_str("\\textit{");
                s.push_str(&escape_latex(t));
                s.push('}');
            }
            Inline::Code(t) => {
                s.push_str("\\texttt{");
                s.push_str(&escape_latex(t));
                s.push('}');
            }
            Inline::Link { text, url } => {
                s.push_str("\\href{");
                s.push_str(&escape_href_url(url));
                s.push_str("}{");
                s.push_str(&escape_latex(text));
                s.push('}');
            }
            // 行内图片（未独占一段）：直接插入 \includegraphics，宽度默认 \textwidth
            Inline::Image { url, .. } => {
                s.push_str("\\includegraphics[width=\\textwidth]{");
                s.push_str(&escape_href_url(url));
                s.push('}');
            }
            Inline::Footnote(t) => {
                s.push_str("\\footnote{");
                s.push_str(&escape_latex(t));
                s.push('}');
            }
        }
    }
    s
}

fn escape_href_url(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\textbackslash{}"),
            '%' => out.push_str("\\%"),
            '#' => out.push_str("\\#"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            other => out.push(other),
        }
    }
    out
}

/// LaTeX 9 个特殊字符的转义 + 反斜杠。
/// 必须 char 流处理，否则 `\\` 与后续替换互相干扰。
fn escape_latex(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\textbackslash{}"),
            '&' => out.push_str("\\&"),
            '%' => out.push_str("\\%"),
            '$' => out.push_str("\\$"),
            '#' => out.push_str("\\#"),
            '_' => out.push_str("\\_"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '~' => out.push_str("\\textasciitilde{}"),
            '^' => out.push_str("\\textasciicircum{}"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ast::{Block, Inline};

    /// 测试辅助：主文件与全部部件按序拼接，便于对整体内容做断言
    fn test_body(e: TexEmitter) -> String {
        let (main, parts) = e.finish();
        let mut s = main;
        for (_, content) in parts {
            s.push_str(&content);
        }
        s
    }

    #[test]
    fn escapes_special_chars() {
        assert_eq!(
            escape_latex("a&b%c$d#e_f{g}h~i^j"),
            "a\\&b\\%c\\$d\\#e\\_f\\{g\\}h\\textasciitilde{}i\\textasciicircum{}j"
        );
    }

    #[test]
    fn inline_bold_italic_emit() {
        let inlines = vec![
            Inline::Text("一段".into()),
            Inline::Bold("重点".into()),
            Inline::Text("和".into()),
            Inline::Italic("斜体".into()),
        ];
        assert_eq!(
            render_inlines(&inlines),
            "一段\\textbf{重点}和\\textit{斜体}"
        );
    }

    #[test]
    fn h2_uses_chinese_numbering() {
        let mut e = TexEmitter::new();
        e.emit_block(&Block::Heading {
            level: 2,
            text: "引言".into(),
        });
        e.emit_block(&Block::Heading {
            level: 2,
            text: "方法".into(),
        });
        let body = test_body(e);
        assert!(body.contains("一、引言"), "got {}", body);
        assert!(body.contains("二、方法"), "got {}", body);
    }

    #[test]
    fn list_prefix_cycles() {
        let mut e = TexEmitter::new();
        e.emit_block(&Block::List {
            ordered: false,
            level: 1,
            content: vec![Inline::Text("a".into())],
        });
        e.emit_block(&Block::List {
            ordered: false,
            level: 1,
            content: vec![Inline::Text("b".into())],
        });
        e.emit_block(&Block::List {
            ordered: false,
            level: 2,
            content: vec![Inline::Text("c".into())],
        });
        let body = test_body(e);
        assert!(body.contains('①') && body.contains('②') && body.contains('⑴'));
    }

    #[test]
    fn table_emits_tabular() {
        let mut e = TexEmitter::new();
        e.emit_block(&Block::Table {
            rows: vec![
                vec!["列A".into(), "列B".into()],
                vec!["1".into(), "2".into()],
            ],
            caption: None,
        });
        let body = test_body(e);
        assert!(body.contains("\\begin{tabular}{|l|l|}"));
        assert!(body.contains("\\textbf{列A}"));
        assert!(body.contains("\\hline"));
    }

    #[test]
    fn table_cell_bold_italic() {
        let mut e = TexEmitter::new();
        e.emit_block(&Block::Table {
            rows: vec![
                vec!["列A".into(), "列B".into()],
                vec!["**重点**".into(), "*斜体*".into()],
            ],
            caption: None,
        });
        let body = test_body(e);
        assert!(body.contains("\\textbf{重点}"));
        assert!(body.contains("\\textit{斜体}"));
        assert!(!body.contains("**重点**"));
    }

    #[test]
    fn standalone_image_emits_figure_with_textwidth() {
        let mut e = TexEmitter::new();
        e.emit_block(&Block::Paragraph(vec![Inline::Image {
            alt: "系统架构".into(),
            url: "figs/arch.png".into(),
        }]));
        let body = test_body(e);
        assert!(body.contains("\\begin{figure}[htbp]"), "got {}", body);
        assert!(body.contains("\\centering"));
        assert!(
            body.contains("\\includegraphics[width=\\textwidth]{figs/arch.png}"),
            "got {}",
            body
        );
        assert!(body.contains("\\caption{系统架构}"));
        assert!(body.contains("\\end{figure}"));
    }

    #[test]
    fn image_without_alt_omits_caption() {
        let mut e = TexEmitter::new();
        e.emit_block(&Block::Paragraph(vec![Inline::Image {
            alt: String::new(),
            url: "a.png".into(),
        }]));
        let body = test_body(e);
        assert!(body.contains("\\includegraphics[width=\\textwidth]{a.png}"));
        assert!(!body.contains("\\caption"));
    }

    #[test]
    fn inline_image_stays_inline() {
        let rendered = render_inlines(&[
            Inline::Text("见".into()),
            Inline::Image {
                alt: "图".into(),
                url: "a.png".into(),
            },
        ]);
        assert_eq!(rendered, "见\\includegraphics[width=\\textwidth]{a.png}");
    }

    #[test]
    fn h2_sections_split_into_data_parts() {
        let mut e = TexEmitter::new();
        e.emit_block(&Block::Heading {
            level: 1,
            text: "标题".into(),
        });
        e.emit_block(&Block::Heading {
            level: 2,
            text: "引言".into(),
        });
        e.emit_block(&Block::Paragraph(vec![Inline::Text("引言正文".into())]));
        e.emit_block(&Block::Heading {
            level: 2,
            text: "方法".into(),
        });
        let (main, parts) = e.finish();

        // 主文件：标题保留，两节按序 \input 引用
        assert!(main.contains("标题"), "got {main}");
        assert!(main.contains("\\input{data/section01.tex}"));
        assert!(main.contains("\\input{data/section02.tex}"));
        assert!(!main.contains("一、引言"), "got {main}");

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].0, "data/section01.tex");
        assert!(parts[0].1.contains("一、引言"));
        assert!(parts[0].1.contains("引言正文"));
        assert_eq!(parts[1].0, "data/section02.tex");
        assert!(parts[1].1.contains("二、方法"));
    }
}
