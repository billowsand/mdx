# mdx 工具技术报告

<!-- [版本变更记录] -->

| 版本 | 日期 | 修订人 | 说明 |
|---|---|---|---|
| 1.0 | 2026-01 | 张三 | 初版，覆盖公文 docx/tex |
| 1.1 | 2026-03 | 李四 | 补充研究报告 docx 输出与评估章节 |
| 1.2 | 2026-04 | 王五 | 新增 `<!-- [参考文献] -->` 标记与 longtblr 表格 |
| 2.0 | 2026-05 | 王五 | 研究报告 tex 改为纯 Rust 实现，不再依赖 pandoc |

<!-- [摘要] -->

本文介绍 mdx 统一 markdown 转换工具的设计与实现。该工具将 Markdown 源文件统一转换为 `docx` 与 `tex` 两种目标格式，并原生支持中文公文（GB/T 9704—2012）与研究报告两套排版样式，覆盖 4 种组合。我们采用 *IR-based* 架构（**parser → IR → emitter**），将解析与输出解耦：一套 Rust 原生解析器产出统一的中间表示 `Vec<Block>`，再由四个独立 emitter 消费输出。实验显示，转换一致性达 98.7%，且无需任何外部 pandoc 依赖。

<!-- [正文] -->

## 一、引言

文档转换是软件工程中的常见需求。现有工具如 *pandoc* 虽然功能强大，但对中文排版（公文格式、研究报告规范）的支持有限，且难以按需定制输出样式。

### 1.1 背景

在科研机构和政府部门，大量文档需要遵循严格的格式规范。例如：

- 公文需符合《党政机关公文格式》（GB/T 9704—2012），正文使用 **仿宋** 三号、标题使用 黑体/楷体 分级。
- 研究报告需要章节编号、目录字段以及带封面的版式。
- 上述两种样式同时存在 `docx` 与 `tex` 两种交付需求，共 **4 种组合**。

### 1.2 动机

目前缺乏一款轻量级、可定制、原生支持中文排版规范的文档转换工具。mdx 旨在填补这一空白，目标有三：

1. 一套 Markdown 子集解析器，纯 Rust 实现，转换阶段无需 pandoc；
2. 多 emitter 共享同一 IR，避免语法解析重复造轮子；
3. 公文 / 研报两套样式在 docx 与 tex 上视觉一致。

## 二、系统设计

mdx 采用管道式架构：parser 将 Markdown 解析为统一中间表示，emitter 消费 IR 生成目标格式。整体分为 CLI、parser、common、emitter 四层。

### 2.1 解析层

`parser::parse` 输入为已合并的源字符串，输出 `Vec<Block>`。处理流程顺序如下：

1. 全局执行 [`common::quotes::convert_quotes`] 将直引号正规化为中文圆引号，但跳过 fenced code block 内部；
2. 行扫描，按"区段标记 / 标题 / 表格 / 列表 / 代码块 / 空行 / 段落"顺序分派；
3. 标题文本经 [`common::heading::clean`] 去除旧编号（"一、"、"1.1"、"(1)"等）；
4. 段落与列表内容用 [`common::inline::parse`] 拆成 `Inline` 序列。

`Block` 中间表示的设计原则是 **尽量贴近 markdown 语义、不附加风格信息**，编号、字体、章节形态完全由 emitter 自决定：

Table: Block 枚举变体与对应 markdown 元素

| 变体 | 含义 | 来源示例 |
|---|---|---|
| `Heading { level, text }` | 1..=6 级标题 | `## 标题` |
| `Paragraph(Vec<Inline>)` | 普通段落 | 纯文本 |
| `List { ordered, level, content }` | 单行列表项，`level` 由缩进推断 | `- a` / `1. b` |
| `Table { rows, caption }` | GFM 表格，首行为表头 | `\| a \| b \|` |
| `Marker(MarkerKind)` | 区段切换标记 | `<!-- [摘要] -->` |
| `CodeBlock { lang, content }` | 带语言标识的代码块 | ` ```rust ` |
| `Empty` | 空行 | — |

### 2.2 中间表示

`Inline` 仅识别 `Text` / `Bold` / `Italic` / `Code` / `Link` 五种类型，对应 markdown 行内语法子集：

```rust
pub enum Inline {
    Text(String),
    Bold(String),          // **加粗**
    Italic(String),        // *斜体*
    Code(String),          // `code`
    Link { text, url },    // [text](url)
}
```

`MarkerKind` 由 HTML 注释触发，区分中英文括号与拼写：

```rust
pub enum MarkerKind { Abstract, Appendix, Changelog, Body, Reference }
```

### 2.3 输出层

emitter 按 (格式, 样式) 两维分派，共四条 pipeline：

Table: 四种 emitter 与对应字号字体方案

| 样式 \\ 格式 | docx | tex |
|---|---|---|
| official 公文 | 仿宋三号正文 + 黑体/楷体分级标题 | `official.cls`，正文 `\zihao{3}` 仿宋 + 1.5 倍行距 |
| research 研报 | 封面 + 目录字段 + "第X章" 编号 | `md2tex.cls`（ctexbook），含 `longtblr` 智能表格 |

#### 2.3.1 公文样式

公文 tex 标题层级映射：

- `#` → 居中大标题，方正小标宋简体二号；
- `##` → "一、" 前缀，黑体；
- `###` → "（一）" 前缀，楷体；
- `####` → "1." 前缀，仿宋；
- `#####` → "(1)" 前缀，仿宋粗体。

#### 2.3.2 研究报告样式

研报 tex 标题层级映射：`#` → `\papertitle`、`##` → `\chapter{}`、`###` → `\section{}`、`####` → `\subsection{}`、`#####` → `\subsubsection{}`。

## 三、关键技术

### 3.1 标题旧编号清理

`common::heading::clean` 按优先级应用 12 条正则，覆盖公文与研报常见编号样式：

| 序号 | 模式 | 示例 |
|---|---|---|
| 1 | 第X章 / 节 / 条 / 部分 | `第二章 引言` |
| 2 | 全角括号中文数字 | `（一）目标` |
| 3 | 中文数字 + 顿号 / 点号 | `一、引言` |
| 4 | 全角括号阿拉伯数字 | `（1）说明` |
| 6 | 多级数字编号 | `1.1.1 细节` |
| 9 | 圆圈数字 ①②③ | `①附录` |
| 12 | 字母编号 | `A. 示例` |

经清理后，旧编号不会污染 emitter 重新计算的章节号，保证 *"一、"* 与 *"第一章"* 双套编号互不影响。

### 3.2 列表层级前缀

公文 6 级列表前缀循环在 `common::numbering::list_prefix` 表达为同一份规则表，由缩进宽度推断 `level`：

- 缩进 0 列 → level 1，前缀 ①②③；
- 缩进 1–4 列 → level 2，前缀 ⑴⑵⑶；
- 缩进 5–8 列 → level 3，前缀 a.b.c.；
- 缩进 9–12 列 → level 4，前缀 I.II.III.；
- 缩进 13–16 列 → level 5，前缀 (A)(B)；
- 缩进 ≥17 列 → level 6，前缀 1)2)。

研报 docx / tex 复用同一循环，但以 Word `pStyle` 或 LaTeX `\begin{enumerate}` 实现层级嵌套。

### 3.3 智能表格排版

研报 tex 通过 `common::table_to_longtblr` 将 `Block::Table` 转为 `longtblr` 环境，按列内容自动决定对齐与列宽：

1. 数字列（数值占比 ≥ 0.8）→ 居中 `c`；
2. 含句子标点或长文本（宽度 > 20）→ 靠左 `l`；
3. 其余按平均宽度居中 / 靠左；
4. 列宽比例 = `max(最大宽度, 平均宽度 × 1.2)`，并在 `[0.8, 4.0]` 区间内裁剪。

表标题支持 pandoc 风格前置 `Table: 标题`、表后 `: 标题`，以及中文 `表：标题`，最终汇入 `caption={...}`。

## 四、评估

我们在真实语料上对四条 pipeline 做端到端验证，记录如下：

| 类别 | 数量 | 平均页数 | 转换一致性 |
|---|---|---|---|
| 通知类公文 | 20 | 2 | 99.2% |
| 报告类公文 | 30 | 8 | 98.5% |
| 研究报告 | 30 | 35 | 98.4% |

: 真实语料转换结果

行内格式抽检：**加粗** 与 *斜体* 均正确映射到 docx `Run` 的 `b` / `i` 属性与 tex `\textbf{}` / `\emph{}`；行内代码 `xcb \\bitblt` 输出为 `\texttt{}`；普通链接 [mdx 仓库](https://example.com/mdx) 渲染为蓝色下划线文字。

常见失败模式集中在：

- 复杂嵌套列表（多段落 list item）— 当前子集未覆盖；
- 行内数学公式与脚注 — 未支持；
- 复杂表格（合并单元格）— 退化为普通 `tabular`。

## 五、编号关系小结

本章仅作示范，展示 markdown 标题层级在公文 emitter 中的逐级编号：

### （一）解析层指标

正文段落与列表共覆盖 7 类 Block。

#### 1. 类型数量

`Inline` 5 类，`MarkerKind` 5 类，`Block` 7 类。

##### (1) 命名一致性

所有变体字段命名与 README 描述保持一致，避免 emitter 间零字段对齐差异。

## 六、讨论

mdx 在 4 种组合上已达成视觉一致；后续将补齐嵌套列表、图片与数学公式，并将公文 docx 迁移至 `common` 路径以统一代码风格。

<!-- [参考文献] -->

1. 张三, 李四. Markdown 到 LaTeX 的转换框架研究[J]. 计算机学报, 2024, 47(3): 456-470.
2. Smith J, Brown A. A Survey of Document Conversion Tools[C]. Proceedings of DocEng, 2023: 112-119.
3. 王五. GB/T 9704 公文格式自动化排版实践[J]. 软件学报, 2025, 36(2): 301-315.
4. 国家标准化管理委员会. GB/T 9704—2012 党政机关公文格式[S]. 北京: 中国标准出版社, 2012.

<!-- [附录] -->

## 附录 A 命令行接口参考

```bash
# 公文 docx / tex
mdx docx notice.md --style official -o notice.docx
mdx tex  notice.md --style official -o notice.tex

# 研究报告 docx / tex（单文件）
mdx docx report.md --style research -o report.docx
mdx tex  report.md --style research -o report.tex

# 研究报告 tex，自定义模板
mdx tex report.md --style research --template my_template.tex -o report.tex

# 多文件合并（按文件名升序）
mdx docx ./chapters --style research -o paper.docx
```

`<input>` 既可以是单个 `.md`，也可以是目录（按文件名升序合并其中所有 `.md`）。目录模式由 `input::merge_dir` 实现，并会先剔除整行 `---` 形式的水平分隔线，避免破坏后续表格分隔行。

## 附录 B 测试语料明细

Table: 各类别语料规模

| 类别 | 数量 | 平均页数 |
|---|---|---|
| 通知类公文 | 20 | 2 |
| 报告类公文 | 30 | 8 |
| 研究报告 | 30 | 35 |

## 附录 C 编译依赖

```bash
cargo build --release      # 产物 target/release/mdx
cargo test                 # parser / numbering / 表格 / emitter 单测
```

PDF 编译依赖任一可用 TeX 引擎：优先 `xelatex`，否则回退 `tectonic`；含参考文献时 `xelatex` 还需配合 `biber`。