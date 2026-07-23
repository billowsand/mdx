//! 中间表示：parser 输出，emitter 消费。
//!
//! 设计原则：尽量贴近 markdown 语义，不附加风格信息。具体编号、字体、章节
//! 形态由 emitter 自行决定，common 不预设公文 / 研报。

#[derive(Debug, Clone)]
pub enum Block {
    /// 标题。`level` 是原 markdown 中的 `#` 数量（1..=6）。
    /// `text` 为已去除旧编号、引号已正规化后的纯标题文本，未做行内格式拆分。
    Heading { level: u8, text: String },

    /// 普通段落。Inline 序列表示文本/粗体/斜体片段。
    Paragraph(Vec<Inline>),

    /// 列表项。每个 Markdown 列表行对应一个 Block::List；
    /// `level` 是缩进推断出的层级（1..=6），具体编号循环交给 emitter。
    List {
        ordered: bool,
        level: u8,
        content: Vec<Inline>,
    },

    /// 表格。`rows[0]` 视为表头；分隔行 `|---|` 已被剥离。
    /// `caption` 对应 pandoc table caption（如 `Table: 标题` 或表后 `: 标题`）。
    Table {
        rows: Vec<Vec<String>>,
        caption: Option<String>,
    },

    /// 区段切换标记，由 `<!-- [...] -->` 注释触发。emitter 据此切模式。
    Marker(MarkerKind),

    /// 代码块。`lang` 为语言标识（如 "rust", "python"），`content` 为代码内容。
    CodeBlock {
        lang: Option<String>,
        content: String,
    },

    /// 空行；多数 emitter 直接忽略。
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Bold(String),
    Italic(String),
    /// 行内代码。如 `code`。
    Code(String),
    /// 链接。如 [text](url)。
    Link {
        text: String,
        url: String,
    },
    /// 图片。如 ![alt](path)。tex emitter 对独占一段的图片输出 figure 环境。
    Image {
        alt: String,
        url: String,
    },
    /// 行内脚注。如 `[^1]:(注释内容)`，仅保存注释内容；编号由输出格式自行生成。
    Footnote(String),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MarkerKind {
    /// 摘要段：`<!-- [摘要] -->`
    Abstract,
    /// 附录段：`<!-- [附录] -->`
    Appendix,
    /// 版本变更记录段：`<!-- [版本变更记录] -->`
    Changelog,
    /// 正文段（恢复正常章节计数）：`<!-- [正文] -->`
    Body,
    /// 参考文献段：`<!-- [参考文献] -->`
    Reference,
}
