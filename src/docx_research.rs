//! 研究报告 → docx pipeline。
//!
//! 视觉布局对齐 `resources/research/md2tex.cls` 与 `template.tex`：
//! - 第 1 页 封面：左上"公开"（黑体4号）+ vfill + 居中一号小标宋标题 + vfill +
//!   居中三号黑体单位 + 16pt 间距 + 居中四号粗体日期；末尾翻页
//! - 第 2 页 目录：居中二号黑体"目录" + `TableOfContents`(dirty) + 翻页
//! - 之后 版本变更记录（不进 TOC，由 `<!-- [版本变更记录] -->` 触发）
//! - 之后 摘要 / 正文 / 附录，按 `<!-- [...] -->` 标记切换 emitter 模式
//!
//! 章节编号：H2 → "第X章 Y"、H3 → "X.Y Z"、H4 → "X.Y.Z W"，附录章节
//! 切换为 "附录 A / 附录 B / ..."。
//!
//! Heading1/2/3 样式在 styles.xml 中显式注册，并设 outline_lvl 0/1/2，让
//! Word 打开时 `TOC` 字段（`TOC \o "1-3"`）能扫描到全部条目；否则单独给段落
//! 挂 `pStyle="Heading1"` 但 styles.xml 里没有该样式定义，TOC 会是空的。

use anyhow::{Context, Result};
use chrono::Local;
use docx_rs::*;
use std::fs::File;
use std::path::Path;

use crate::common::ast::{Block, Inline, MarkerKind};
use crate::common::numbering::{int_to_roman, number_to_uppercase_letter};
use crate::parser;

// ===== 字体（与 LaTeX md2tex.cls 一致；用户须装相应字体，否则 Word 端字体回退） =====
const FONT_TITLE: &str = "FZXiaoBiaoSong-B05"; // 方正小标宋简体
const FONT_HEAD: &str = "FZHei-B01"; // 方正黑体
const FONT_BODY: &str = "FZShuSong-Z01"; // 方正书宋（仿宋类）
const FONT_KAI: &str = "FZKai-Z03"; // 方正楷体（行内强调用）

// ===== 字号（半磅 half-points，1pt = 2hp） =====
// LaTeX 模板中常用字号：
//   一号 = 26pt = 52hp（封面标题）
//   二号 = 22pt = 44hp（章 / 目录标题）
//   三号 = 16pt = 32hp（节标题 / 单位）
//   四号 = 14pt = 28hp（子节标题 / 日期 / "公开"）
//   小四 ≈ 12pt = 24hp
//   normalsize 14bp ≈ 14pt = 28hp（正文）
const SIZE_COVER_TITLE: usize = 52;
const SIZE_HEAD1: usize = 44; // chapter / 摘要 / 附录 / 目录 / 版本变更记录
const SIZE_HEAD2: usize = 32; // section
const SIZE_HEAD3: usize = 28; // subsection / 日期 / 公开
const SIZE_BODY: usize = 28; // 正文，对齐 LaTeX 14bp
const SIZE_COVER_INSTITUTION: usize = 32;
const SIZE_COVER_DATE: usize = 28;

// ===== 行距 / 间距 (单位 twips，1pt = 20twips) =====
// LaTeX 正文 24pt 行距 → 480twips；line_rule=AtLeast 保大字符不溢出
const LINE_BODY: i32 = 480;
const LINE_HEAD: i32 = 600; // 30pt，章标题留白
                            // 段前/段后 (twips)
const SPACING_BEFORE_CHAPTER: u32 = 480;
const SPACING_AFTER_CHAPTER: u32 = 480;
const SPACING_BEFORE_SECTION: u32 = 240;
const SPACING_AFTER_SECTION: u32 = 120;

// ===== 页面边距 (twips, 1mm = 56.6929 twips) =====
// LaTeX geometry: top=37mm, left=28mm, width=156mm, height=225mm（A4 = 210×297mm）
const PAGE_TOP: i32 = 2098; // 37 mm
const PAGE_BOTTOM: i32 = 1984; // 35 mm
const PAGE_LEFT: i32 = 1587; // 28 mm
const PAGE_RIGHT: i32 = 1474; // 26 mm

// ===== 圆圈数字（与公文一致，emitter 局部使用） =====
const CIRCLE_NUMBERS_1: &[&str] = &[
    "⑴", "⑵", "⑶", "⑷", "⑸", "⑹", "⑺", "⑻", "⑼", "⑽", "⑾", "⑿", "⒀", "⒁", "⒂", "⒃", "⒄", "⒅", "⒆",
    "⒇",
];
const CIRCLE_NUMBERS_2: &[&str] = &[
    "①", "②", "③", "④", "⑤", "⑥", "⑦", "⑧", "⑨", "⑩", "⑪", "⑫", "⑬", "⑭", "⑮", "⑯", "⑰", "⑱", "⑲",
    "⑳",
];

fn font_set(name: &str) -> RunFonts {
    RunFonts::new()
        .ascii(name)
        .hi_ansi(name)
        .east_asia(name)
        .cs(name)
}

/// 入口：研究报告 docx 转换。
pub fn run(input: &Path, output: Option<&Path>) -> Result<()> {
    let content = crate::input::collect(input)?;
    let output_path = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::input::default_output(input, "docx"));

    println!("正在转换: {}", input.display());

    let blocks = parser::parse(&content);
    let split = split_blocks(&blocks);

    let mut docx = base_docx();
    docx = register_styles(docx);
    docx = add_cover(docx, split.title.as_deref());
    docx = add_toc(docx);

    if !split.changelog.is_empty() {
        docx = add_changelog(docx, &split.changelog);
    }

    let mut emitter = MainEmitter::new();
    docx = emitter.emit_all(docx, &split.main);

    if let Some(dir) = output_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("创建输出目录 {} 失败", dir.display()))?;
    }
    let file = File::create(&output_path)
        .with_context(|| format!("创建输出文件 {} 失败", output_path.display()))?;
    docx.build()
        .pack(file)
        .with_context(|| format!("写入 docx {} 失败", output_path.display()))?;

    println!("[完成] 转换完成: {}", output_path.display());
    Ok(())
}

// ============================================================
// 切分：标题 + 版本变更记录 + 主体
// ============================================================

struct SplitBlocks {
    title: Option<String>,
    changelog: Vec<Block>,
    main: Vec<Block>,
}

#[derive(PartialEq, Eq, Copy, Clone)]
enum Bucket {
    Main,
    Changelog,
}

fn split_blocks(blocks: &[Block]) -> SplitBlocks {
    let mut title: Option<String> = None;
    let mut changelog = Vec::new();
    let mut main = Vec::new();
    let mut bucket = Bucket::Main;

    for b in blocks {
        match b {
            Block::Heading { level: 1, text } if title.is_none() => {
                title = Some(text.clone());
            }
            Block::Marker(MarkerKind::Changelog) => {
                bucket = Bucket::Changelog;
            }
            Block::Marker(MarkerKind::Body)
            | Block::Marker(MarkerKind::Abstract)
            | Block::Marker(MarkerKind::Appendix)
            | Block::Marker(MarkerKind::Reference) => {
                bucket = Bucket::Main;
                main.push(b.clone());
            }
            _ => match bucket {
                Bucket::Main => main.push(b.clone()),
                Bucket::Changelog => changelog.push(b.clone()),
            },
        }
    }
    SplitBlocks {
        title,
        changelog,
        main,
    }
}

// ============================================================
// 文档默认 / 页面 / 样式
// ============================================================

fn base_docx() -> Docx {
    Docx::new()
        .page_margin(
            PageMargin::new()
                .top(PAGE_TOP)
                .bottom(PAGE_BOTTOM)
                .left(PAGE_LEFT)
                .right(PAGE_RIGHT),
        )
        .default_fonts(font_set(FONT_BODY))
        .default_size(SIZE_BODY)
        .default_line_spacing(
            LineSpacing::new()
                .line(LINE_BODY)
                .line_rule(LineSpacingType::AtLeast),
        )
}

/// 注册 Heading1/2/3 样式到 styles.xml。
///
/// `TOC \o "1-3"` 字段会扫描 styleId 为 Heading1/2/3 的段落，或 outlineLvl
/// 为 0/1/2 的段落。两者同时设置最稳妥。
fn register_styles(docx: Docx) -> Docx {
    let h1 = Style::new("Heading1", StyleType::Paragraph)
        .name("heading 1")
        .based_on("Normal")
        .next("Normal")
        .fonts(font_set(FONT_HEAD))
        .size(SIZE_HEAD1)
        .bold()
        .align(AlignmentType::Center)
        .line_spacing(
            LineSpacing::new()
                .before(SPACING_BEFORE_CHAPTER)
                .after(SPACING_AFTER_CHAPTER)
                .line(LINE_HEAD)
                .line_rule(LineSpacingType::AtLeast),
        )
        .outline_lvl(0)
        .ui_priority(9)
        .q_format(true);

    let h2 = Style::new("Heading2", StyleType::Paragraph)
        .name("heading 2")
        .based_on("Normal")
        .next("Normal")
        .fonts(font_set(FONT_HEAD))
        .size(SIZE_HEAD2)
        .bold()
        .align(AlignmentType::Left)
        // titlespacing{\section}{2em}{0pt}{0pt}：左缩进 2em
        .indent(None, None, None, Some(200))
        .line_spacing(
            LineSpacing::new()
                .before(SPACING_BEFORE_SECTION)
                .after(SPACING_AFTER_SECTION)
                .line(LINE_BODY)
                .line_rule(LineSpacingType::AtLeast),
        )
        .outline_lvl(1)
        .ui_priority(9)
        .q_format(true);

    let h3 = Style::new("Heading3", StyleType::Paragraph)
        .name("heading 3")
        .based_on("Normal")
        .next("Normal")
        .fonts(font_set(FONT_HEAD))
        .size(SIZE_HEAD3)
        .bold()
        .align(AlignmentType::Left)
        .indent(None, None, None, Some(200))
        .line_spacing(
            LineSpacing::new()
                .before(SPACING_BEFORE_SECTION)
                .after(SPACING_AFTER_SECTION)
                .line(LINE_BODY)
                .line_rule(LineSpacingType::AtLeast),
        )
        .outline_lvl(2)
        .ui_priority(9)
        .q_format(true);

    docx.add_style(h1).add_style(h2).add_style(h3)
}

// ============================================================
// 封面
// ============================================================

fn add_cover(mut docx: Docx, title: Option<&str>) -> Docx {
    let title_text = title.unwrap_or("研究报告");

    // 左上"公开"（黑体四号）
    docx = docx.add_paragraph(
        Paragraph::new()
            .align(AlignmentType::Left)
            .line_spacing(
                LineSpacing::new()
                    .line(LINE_BODY)
                    .line_rule(LineSpacingType::AtLeast),
            )
            .add_run(
                Run::new()
                    .add_text("公开")
                    .fonts(font_set(FONT_HEAD))
                    .size(SIZE_HEAD3)
                    .bold(),
            ),
    );

    // 上半 vfill：约 8 空行把标题推到中部
    for _ in 0..8 {
        docx = docx.add_paragraph(Paragraph::new());
    }

    // 标题（一号小标宋居中）
    docx = docx.add_paragraph(
        Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(
                LineSpacing::new()
                    .before(0)
                    .after(0)
                    .line(800)
                    .line_rule(LineSpacingType::AtLeast),
            )
            .add_run(
                Run::new()
                    .add_text(title_text)
                    .fonts(font_set(FONT_TITLE))
                    .size(SIZE_COVER_TITLE)
                    .bold(),
            ),
    );

    // 下半 vfill：约 9 空行把单位/日期推到底部
    for _ in 0..9 {
        docx = docx.add_paragraph(Paragraph::new());
    }

    // "某某单位"（黑体三号居中）
    docx = docx.add_paragraph(
        Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(
                LineSpacing::new()
                    .after(320) // 16pt vspace
                    .line(LINE_BODY)
                    .line_rule(LineSpacingType::AtLeast),
            )
            .add_run(
                Run::new()
                    .add_text("某某单位")
                    .fonts(font_set(FONT_HEAD))
                    .size(SIZE_COVER_INSTITUTION)
                    .bold(),
            ),
    );

    // 日期（黑体四号居中加粗，对齐 LaTeX `\zihao{4}\bfseries\dateofsubmit`）
    let now = Local::now();
    let date_str = format!(
        "{}年{}月{}日",
        now.format("%Y"),
        now.format("%m"),
        now.format("%d")
    );
    docx = docx.add_paragraph(
        Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(
                LineSpacing::new()
                    .line(LINE_BODY)
                    .line_rule(LineSpacingType::AtLeast),
            )
            .add_run(
                Run::new()
                    .add_text(&date_str)
                    .fonts(font_set(FONT_HEAD))
                    .size(SIZE_COVER_DATE)
                    .bold(),
            ),
    );

    // 封面后强制翻页
    docx.add_paragraph(Paragraph::new().page_break_before(true))
}

// ============================================================
// 目录
// ============================================================

fn add_toc(mut docx: Docx) -> Docx {
    // "目录"标题（不带 Heading 样式，避免自引用）
    docx = docx.add_paragraph(
        Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(
                LineSpacing::new()
                    .before(0)
                    .after(SPACING_AFTER_CHAPTER)
                    .line(LINE_HEAD)
                    .line_rule(LineSpacingType::AtLeast),
            )
            .add_run(
                Run::new()
                    .add_text("目  录")
                    .fonts(font_set(FONT_HEAD))
                    .size(SIZE_HEAD1)
                    .bold(),
            ),
    );

    docx = docx.add_table_of_contents(TableOfContents::new().heading_styles_range(1, 3).dirty());

    // 目录后翻页
    docx.add_paragraph(Paragraph::new().page_break_before(true))
}

// ============================================================
// 版本变更记录
// ============================================================

fn add_changelog(mut docx: Docx, blocks: &[Block]) -> Docx {
    docx = docx.add_paragraph(
        Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(
                LineSpacing::new()
                    .before(0)
                    .after(SPACING_AFTER_CHAPTER)
                    .line(LINE_HEAD)
                    .line_rule(LineSpacingType::AtLeast),
            )
            .add_run(
                Run::new()
                    .add_text("版本变更记录")
                    .fonts(font_set(FONT_HEAD))
                    .size(SIZE_HEAD1)
                    .bold(),
            ),
    );

    let mut emitter = ChangelogEmitter::new();
    for b in blocks {
        docx = emitter.emit(docx, b);
    }
    docx.add_paragraph(Paragraph::new().page_break_before(true))
}

// ============================================================
// 主 emitter（含模式：Body / Abstract / Appendix）
// ============================================================

#[derive(Copy, Clone, Eq, PartialEq)]
enum Mode {
    Body,
    Abstract,
    Appendix,
    Reference,
}

struct MainEmitter {
    mode: Mode,
    chapter: usize,
    section: usize,
    subsection: usize,
    appendix_idx: usize, // 0..N，对应附录 A..
    ref_counter: usize,  // 参考文献条目计数器 [1] [2] ...
    list: ListState,
}

#[derive(Default)]
struct ListState {
    in_list: bool,
    level: u8,
    l1: usize,
    l2: usize,
    l3: usize,
    l4: usize,
    l5: usize,
    l6: usize,
}

impl MainEmitter {
    fn new() -> Self {
        Self {
            mode: Mode::Body,
            chapter: 0,
            section: 0,
            subsection: 0,
            appendix_idx: 0,
            ref_counter: 0,
            list: ListState::default(),
        }
    }

    fn emit_all(&mut self, mut docx: Docx, blocks: &[Block]) -> Docx {
        for b in blocks {
            docx = self.emit(docx, b);
        }
        docx
    }

    fn emit(&mut self, docx: Docx, b: &Block) -> Docx {
        match b {
            Block::Marker(kind) => {
                self.handle_marker(*kind);
                if matches!(kind, MarkerKind::Abstract) {
                    return self.emit_abstract_header(docx);
                }
                if matches!(kind, MarkerKind::Reference) {
                    return self.emit_reference_header(docx);
                }
                docx
            }
            Block::Heading { level, text } => {
                self.list.reset();
                self.emit_heading(docx, *level, text)
            }
            Block::Paragraph(inlines) => {
                self.list.reset();
                add_body_paragraph(docx, |p| add_inlines(p, inlines))
            }
            Block::List {
                ordered: _,
                level,
                content,
            } => {
                let prefix = if self.mode == Mode::Reference && *level == 1 {
                    self.ref_counter += 1;
                    format!("[{}] ", self.ref_counter)
                } else {
                    self.list.next_prefix(*level)
                };
                add_list_paragraph(docx, *level, &prefix, content)
            }
            Block::Table { rows, .. } => {
                self.list.reset();
                add_table(docx, rows)
            }
            Block::CodeBlock { .. } => {
                // 研究报告 docx 暂不支持代码块
                docx
            }
            Block::Empty => docx,
            Block::Label(_) => {
                // docx 暂不支持交叉引用锚点，忽略
                docx
            }
        }
    }

    fn handle_marker(&mut self, kind: MarkerKind) {
        match kind {
            MarkerKind::Abstract => self.mode = Mode::Abstract,
            MarkerKind::Appendix => {
                self.mode = Mode::Appendix;
                self.appendix_idx = 0;
            }
            MarkerKind::Body => self.mode = Mode::Body,
            MarkerKind::Reference => {
                self.mode = Mode::Reference;
                self.appendix_idx = 0;
                self.ref_counter = 0;
            }
            MarkerKind::Changelog => {
                self.mode = Mode::Body;
            }
        }
    }

    /// 摘要标题：与章同级（出现在 TOC 中），但文字固定为"摘要"。
    fn emit_abstract_header(&mut self, docx: Docx) -> Docx {
        docx.add_paragraph(heading_chapter_paragraph("摘要"))
    }

    /// 参考文献标题：与章同级（出现在 TOC 中），文字固定为"参考文献"。
    fn emit_reference_header(&mut self, docx: Docx) -> Docx {
        docx.add_paragraph(heading_chapter_paragraph("参考文献"))
    }

    fn emit_heading(&mut self, mut docx: Docx, level: u8, text: &str) -> Docx {
        match (level, self.mode) {
            (2, Mode::Body) => {
                self.chapter += 1;
                self.section = 0;
                self.subsection = 0;
                let label = format!("第{}章 {}", chinese_chapter(self.chapter), text);
                docx = page_break(docx);
                docx.add_paragraph(heading_chapter_paragraph(&label))
            }
            (3, Mode::Body) => {
                self.section += 1;
                self.subsection = 0;
                let label = format!("{}.{} {}", self.chapter, self.section, text);
                docx.add_paragraph(heading_section_paragraph(&label))
            }
            (4, Mode::Body) => {
                self.subsection += 1;
                let label = format!(
                    "{}.{}.{} {}",
                    self.chapter, self.section, self.subsection, text
                );
                docx.add_paragraph(heading_subsection_paragraph(&label))
            }
            (2, Mode::Appendix) => {
                self.appendix_idx += 1;
                let letter = (b'A' + (self.appendix_idx - 1) as u8) as char;
                let label = format!("附录 {} {}", letter, text);
                docx = page_break(docx);
                docx.add_paragraph(heading_chapter_paragraph(&label))
            }
            (3, Mode::Appendix) => docx.add_paragraph(heading_section_paragraph(text)),
            (4, Mode::Appendix) => docx.add_paragraph(heading_subsection_paragraph(text)),
            (2, Mode::Reference) => docx.add_paragraph(heading_section_paragraph(text)),
            (3, Mode::Reference) => docx.add_paragraph(heading_subsection_paragraph(text)),
            (_, Mode::Abstract) => {
                // 摘要里出现的子标题降级为加粗居中段落
                add_body_paragraph(docx, |p| {
                    p.align(AlignmentType::Center).add_run(
                        Run::new()
                            .add_text(text.to_string())
                            .fonts(font_set(FONT_HEAD))
                            .size(SIZE_HEAD2)
                            .bold(),
                    )
                })
            }
            (1, _) => {
                // 第二个 H1（在主文中再次出现）退化为加粗居中段落
                add_body_paragraph(docx, |p| {
                    p.align(AlignmentType::Center).add_run(
                        Run::new()
                            .add_text(text.to_string())
                            .fonts(font_set(FONT_TITLE))
                            .size(SIZE_HEAD1)
                            .bold(),
                    )
                })
            }
            _ => docx,
        }
    }
}

// ============================================================
// 版本变更记录的简化 emitter（无章节计数 / 无模式）
// ============================================================

struct ChangelogEmitter {
    list: ListState,
}

impl ChangelogEmitter {
    fn new() -> Self {
        Self {
            list: ListState::default(),
        }
    }

    fn emit(&mut self, docx: Docx, b: &Block) -> Docx {
        match b {
            Block::Heading { level, text } => {
                self.list.reset();
                let size = match level {
                    1 | 2 => SIZE_HEAD2,
                    3 => SIZE_HEAD3,
                    _ => SIZE_BODY,
                };
                add_body_paragraph(docx, |p| {
                    p.align(AlignmentType::Left).add_run(
                        Run::new()
                            .add_text(text.to_string())
                            .fonts(font_set(FONT_HEAD))
                            .size(size)
                            .bold(),
                    )
                })
            }
            Block::Paragraph(inlines) => {
                self.list.reset();
                add_body_paragraph(docx, |p| add_inlines(p, inlines))
            }
            Block::List {
                ordered: _,
                level,
                content,
            } => {
                let prefix = self.list.next_prefix(*level);
                add_list_paragraph(docx, *level, &prefix, content)
            }
            Block::Table { rows, .. } => {
                self.list.reset();
                add_table(docx, rows)
            }
            Block::Marker(_) | Block::Empty | Block::CodeBlock { .. } | Block::Label(_) => docx,
        }
    }
}

// ============================================================
// 列表前缀状态机（与 docx_official::Converter 完全一致）
// ============================================================

impl ListState {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn next_prefix(&mut self, level: u8) -> String {
        let prefix = match level {
            1 => {
                if !self.in_list || self.level > 1 {
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
                if !self.in_list || self.level != 2 {
                    if self.level > 2 {
                        self.l3 = 0;
                        self.l4 = 0;
                        self.l5 = 0;
                        self.l6 = 0;
                    }
                    if !self.in_list || self.level < 2 {
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
                if !self.in_list || self.level < 3 {
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
                if !self.in_list || self.level < 4 {
                    self.l4 = 0;
                    self.l5 = 0;
                    self.l6 = 0;
                }
                self.l4 += 1;
                format!("{}.", int_to_roman(self.l4))
            }
            5 => {
                if !self.in_list || self.level < 5 {
                    self.l5 = 0;
                    self.l6 = 0;
                }
                self.l5 += 1;
                format!("({})", number_to_uppercase_letter(self.l5))
            }
            6 => {
                if !self.in_list || self.level < 6 {
                    self.l6 = 0;
                }
                self.l6 += 1;
                format!("{})", self.l6)
            }
            _ => String::new(),
        };
        self.in_list = true;
        self.level = level;
        prefix
    }
}

// ============================================================
// 段落构造工具
// ============================================================

/// 章 / 摘要 / 附录 / 目录标题：用 Heading1 样式，居中、二号黑体。
fn heading_chapter_paragraph(text: &str) -> Paragraph {
    Paragraph::new()
        .style("Heading1")
        .align(AlignmentType::Center)
        .add_run(
            Run::new()
                .add_text(text)
                .fonts(font_set(FONT_HEAD))
                .size(SIZE_HEAD1)
                .bold(),
        )
}

/// 节标题：用 Heading2 样式，左缩进 2 字符、三号黑体。
fn heading_section_paragraph(text: &str) -> Paragraph {
    Paragraph::new()
        .style("Heading2")
        .align(AlignmentType::Left)
        .indent(None, None, None, Some(200))
        .add_run(
            Run::new()
                .add_text(text)
                .fonts(font_set(FONT_HEAD))
                .size(SIZE_HEAD2)
                .bold(),
        )
}

/// 子节标题：用 Heading3 样式，左缩进 2 字符、四号黑体。
fn heading_subsection_paragraph(text: &str) -> Paragraph {
    Paragraph::new()
        .style("Heading3")
        .align(AlignmentType::Left)
        .indent(None, None, None, Some(200))
        .add_run(
            Run::new()
                .add_text(text)
                .fonts(font_set(FONT_HEAD))
                .size(SIZE_HEAD3)
                .bold(),
        )
}

/// 普通正文段落：两端对齐、首行缩进 2 字符、AtLeast 24pt 行距。
fn add_body_paragraph<F>(docx: Docx, build: F) -> Docx
where
    F: FnOnce(Paragraph) -> Paragraph,
{
    // 首行缩进 2 字符 ≈ 2×14pt = 560 twips（与 ctex 的 \parindent=2em 等效）
    let p = Paragraph::new()
        .align(AlignmentType::Both)
        .indent(Some(0), Some(SpecialIndentType::FirstLine(560)), None, None)
        .line_spacing(
            LineSpacing::new()
                .line(LINE_BODY)
                .line_rule(LineSpacingType::AtLeast),
        );
    docx.add_paragraph(build(p))
}

/// 列表段落：根据层级设置左缩进，首行不再额外缩进。
fn add_list_paragraph(docx: Docx, level: u8, prefix: &str, content: &[Inline]) -> Docx {
    // 每加深一层缩进 1 个中文字符（200 hundredths-of-char = 2 char base + level）
    let chars = (level.saturating_sub(1) as i32) * 100; // 0/100/200/300...
    let p = Paragraph::new().align(AlignmentType::Both).line_spacing(
        LineSpacing::new()
            .line(LINE_BODY)
            .line_rule(LineSpacingType::AtLeast),
    );
    let p = if chars > 0 {
        p.indent(None, None, None, Some(chars))
    } else {
        p
    };
    let p = p.add_run(
        Run::new()
            .add_text(prefix.to_string())
            .fonts(font_set(FONT_BODY))
            .size(SIZE_BODY),
    );
    let p = add_inlines(p, content);
    docx.add_paragraph(p)
}

fn inline_run_style(ip: &Inline) -> (String, bool, bool) {
    match ip {
        Inline::Text(t) => (t.clone(), false, false),
        // docx 简化处理：粗体 / 斜体的子节点降级为纯文本（嵌套格式丢失）
        Inline::Bold(children) => (crate::common::inline::flatten(children), true, false),
        Inline::Italic(children) => (crate::common::inline::flatten(children), false, true),
        Inline::Code(t) => (t.clone(), false, false),
        Inline::Link { text, .. } => (text.clone(), false, false),
        // docx 暂不支持插图，降级为替代文本
        Inline::Image { alt, .. } => (alt.clone(), false, false),
        // docx 暂不支持交叉引用，降级为 id 文本
        Inline::CrossRef(id) => (id.clone(), false, false),
        // docx 暂不生成原生文献引用，保留 Pandoc 方括号标记
        Inline::Citation(keys) => (
            format!(
                "[{}]",
                keys.iter()
                    .map(|key| format!("@{key}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            false,
            false,
        ),
        // docx 暂不生成脚注部件，降级为全角括号内联注释
        Inline::Footnote(t) => (format!("（{}）", t), false, false),
    }
}

fn add_inlines(mut p: Paragraph, inlines: &[Inline]) -> Paragraph {
    for ip in inlines {
        let (text, bold, italic) = inline_run_style(ip);
        let mut run = Run::new().add_text(&text).size(SIZE_BODY);
        // 斜体在中文里用楷体表达（与 LaTeX `ItalicFont={FZKai-Z03}` 一致）
        if italic {
            run = run.fonts(font_set(FONT_KAI)).italic();
        } else {
            run = run.fonts(font_set(FONT_BODY));
        }
        if bold {
            run = run.bold();
        }
        p = p.add_run(run);
    }
    p
}

fn page_break(docx: Docx) -> Docx {
    docx.add_paragraph(Paragraph::new().page_break_before(true))
}

fn add_table(docx: Docx, rows: &[Vec<String>]) -> Docx {
    if rows.is_empty() {
        return docx;
    }
    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if max_cols == 0 {
        return docx;
    }

    let mut table_rows = Vec::new();
    for (row_idx, row) in rows.iter().enumerate() {
        let mut cells = Vec::new();
        for col_idx in 0..max_cols {
            let cell_data = row.get(col_idx).map(String::as_str).unwrap_or("");
            let align = AlignmentType::Center;
            // 表头整行黑体加粗；表体解析 cell 内的 **加粗**/*斜体* 行内格式。
            let is_header = row_idx == 0;
            let mut p = Paragraph::new().align(align);
            for ip in crate::common::inline::parse(cell_data) {
                let (text, bold, italic) = inline_run_style(&ip);
                let mut run = Run::new().add_text(&text).size(SIZE_BODY);
                if is_header {
                    run = run.fonts(font_set(FONT_HEAD)).bold();
                } else if italic {
                    // 斜体在中文里用楷体表达（与正文 add_inlines 一致）
                    run = run.fonts(font_set(FONT_KAI)).italic();
                } else {
                    run = run.fonts(font_set(FONT_BODY));
                }
                if bold && !is_header {
                    run = run.bold();
                }
                p = p.add_run(run);
            }
            cells.push(TableCell::new().add_paragraph(p));
        }
        table_rows.push(TableRow::new(cells));
    }

    let table = Table::new(table_rows).set_borders(
        TableBorders::new()
            .set(TableBorder::new(TableBorderPosition::Top).size(4))
            .set(TableBorder::new(TableBorderPosition::Left).size(4))
            .set(TableBorder::new(TableBorderPosition::Bottom).size(4))
            .set(TableBorder::new(TableBorderPosition::Right).size(4))
            .set(TableBorder::new(TableBorderPosition::InsideH).size(4))
            .set(TableBorder::new(TableBorderPosition::InsideV).size(4)),
    );
    docx.add_table(table)
}

// ============================================================
// 中文章节序号
// ============================================================

fn chinese_chapter(num: usize) -> String {
    use crate::common::numbering::number_to_chinese;
    number_to_chinese(num)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ast::Inline;

    #[test]
    fn split_extracts_title_and_changelog() {
        let blocks = vec![
            Block::Heading {
                level: 1,
                text: "测试报告".into(),
            },
            Block::Marker(MarkerKind::Changelog),
            Block::Paragraph(vec![Inline::Text("v1.0 初版".into())]),
            Block::Marker(MarkerKind::Body),
            Block::Heading {
                level: 2,
                text: "引言".into(),
            },
            Block::Paragraph(vec![Inline::Text("正文".into())]),
        ];
        let split = split_blocks(&blocks);
        assert_eq!(split.title.as_deref(), Some("测试报告"));
        assert_eq!(split.changelog.len(), 1);
        assert!(split
            .main
            .iter()
            .any(|b| matches!(b, Block::Heading { level: 2, .. })));
    }

    #[test]
    fn main_emitter_numbers_chapters() {
        let mut e = MainEmitter::new();
        let docx = Docx::new();
        let docx = e.emit(
            docx,
            &Block::Heading {
                level: 2,
                text: "引言".into(),
            },
        );
        let docx = e.emit(
            docx,
            &Block::Heading {
                level: 3,
                text: "背景".into(),
            },
        );
        let _ = e.emit(
            docx,
            &Block::Heading {
                level: 4,
                text: "动机".into(),
            },
        );
        assert_eq!(e.chapter, 1);
        assert_eq!(e.section, 1);
        assert_eq!(e.subsection, 1);
    }

    #[test]
    fn main_emitter_appendix_letters() {
        let mut e = MainEmitter::new();
        e.handle_marker(MarkerKind::Appendix);
        let docx = Docx::new();
        let docx = e.emit(
            docx,
            &Block::Heading {
                level: 2,
                text: "数据集".into(),
            },
        );
        let _ = e.emit(
            docx,
            &Block::Heading {
                level: 2,
                text: "代码".into(),
            },
        );
        assert_eq!(e.appendix_idx, 2);
    }

    #[test]
    fn docx_fallback_reconstructs_citation_source() {
        let (text, bold, italic) =
            inline_run_style(&Inline::Citation(vec!["a".into(), "b".into()]));
        assert_eq!(text, "[@a; @b]");
        assert!(!bold);
        assert!(!italic);
    }
}
