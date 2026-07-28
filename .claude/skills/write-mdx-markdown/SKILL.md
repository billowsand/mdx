---
name: write-mdx-markdown
description: Author Markdown that converts cleanly through the mdx tool (`mdx docx|tex --style official|research`). Use whenever writing or fixing a `.md` source meant to become a 公文 (official) or 研究报告 (research) docx/tex/PDF — covers headings, front matter cover fields, research section markers (摘要/正文/附录/参考文献/版本变更记录), figures with anchors, cross-references `{#id}`/`{@id}`, BibTeX citations `[@key]`, tables and captions, inline footnotes, lists, code blocks, and the syntax mdx explicitly does NOT support.
---

# 撰写符合 mdx 标准的 Markdown

mdx 用 Rust 内生解析器把 Markdown 转成 docx / tex，覆盖**公文（official）**与**研究报告（research）**两套样式，4 种组合：

```bash
mdx docx <input> --style official   # 公文 docx
mdx docx <input> --style research   # 研究报告 docx（封面+目录+章节编号）
mdx tex  <input> --style official   # 公文 tex（检测到 TeX 引擎自动编 PDF）
mdx tex  <input> --style research   # 研究报告 tex（自动编 PDF）
```

`<input>` 可为单个 `.md` 或含 `.md` 的目录（按**文件名升序**合并，建议 `01-xxx.md`、`02-xxx.md`）。

**权威规范是 [docs/markdown-extensions.md](../../../docs/markdown-extensions.md)** —— 本 skill 是可直接照用的操作清单。撰写前先确认目标样式（official / research）和格式（docx / tex），因为不同组合下同一标记行为不同。

## 铁律（最常见的错误）

1. **绝不手写编号。** 标题里的 `一、`、`1.1`、`（一）`、`第X章`，表题里的 `表1：`，列表项的 `1.`/`①` 前缀——编号全部由输出端自动生成。手写的编号会被剥除或造成重复。
2. **一行一个列表项**，层级用前导空格缩进（见 §列表），不要写多段落 / 嵌套子列表。
3. **不要使用不支持的语法**（见最后一节）——数学公式、blockquote、引用式脚注、raw LaTeX/HTML、复杂/嵌套列表、多段落列表——输出不可预期。
4. tex 转换会**先校验**锚点、引用和 Bib 文件，任一硬错误会**停止转换、不产出任何文件**。先把引用关系写对。

## 标题层级

| markdown | official（公文） | research（研究报告） |
|---|---|---|
| `#`      | 红头大标题（居中） | 文档标题/封面（每个输入仅取第一个 H1，且从正文移除） |
| `##`     | 一、二、三…（黑体） | 第 X 章（`\chapter`，拆到 `data/chapterNN.tex`） |
| `###`    | （一）（二）…（楷体） | X.Y（`\section`） |
| `####`   | 1. 2. 3. | X.Y.Z（`\subsection`） |
| `#####`  | (1) (2) (3) | `\subsubsection` |
| `######` | 忽略 | 忽略 |

- 标题只写**纯标题文字**，如 `## 概述`，不写 `## 二、概述` 或 `## 第二章 概述`。
- research 一份文档内统一一种开章写法（都用 `##` 起章，或附录段都用 `#`），不要混用。

## 封面字段（research，front matter）

文件**首行**起用 `---` 包围，`键: 值` 每行一条（冒号中英文皆可）：

```markdown
---
密级: 机密
年限: 5年
文件类型: 技术报告
文件编号: XX-2026-001
版本: V1.0
撰写单位: 某研究所
撰写时间: 2026-07
文件名称: ××系统研制报告
bibliography: refs/library.bib
---
```

- 键名（含英文别名）：`密级`/`security`、`年限`|`保密年限`|`保密期限`/`years`、`文件类型`|`类型`/`doctype`、`文件编号`|`编号`/`number`、`文件版本号`|`版本`/`version`、`撰写单位`|`单位`/`institution`、`撰写时间`|`时间`|`日期`/`date`、`文件名称`|`标题`/`title`、`bibliography`。
- `密级` 与 `年限` 分列两个字段，封面排为 `密级：机密★5年`（tex 用 pifont 的 `\ding{72}`）；只写 `密级` 时无星号，`年限` 请自带单位（写 `5年` 而非 `5`）。
- `撰写时间` 接受 `2026-07`/`2026/7`/`2026.07`/`2026年7月`，统一输出"YYYY 年 M 月"；缺省取编译当月。
- **`---` 必须闭合**，否则整块按普通正文处理。封面字段仅 research 生效；`bibliography` 两种 tex 样式都读。目录输入时 front matter 写在排序后的第一个文件。

## 区段标记（research，独占一行的 HTML 注释）

```markdown
<!-- [摘要] -->            进入摘要段（汇入 abstract；不计章节编号）
<!-- [正文] -->            回到正文（章节计数重置为第一章）
<!-- [版本变更记录] -->     不编号章（\chapter*），留在主 tex
<!-- [参考文献] -->        不编号；列表项自动加 [1] [2] 前缀
<!-- [附录] -->            \appendix，之后章拆到 appendix/appendixNN.tex
```

- official 路径忽略这些标记。BibTeX 自动引用**不需要** `<!-- [参考文献] -->`；该标记只用于手写参考文献段。
- 不带任何标记时，所有 H2 按"第一章 / 第二章…"顺序编号。

## 图片

```markdown
![系统架构图](pics/arch.png){#fig:arch}
```

- **独占一段**才输出 `figure` 环境，替代文本作为 `\caption`；行内混排只输出裸图。**要被引用的图必须独占一段且有替代文本**，否则 `\label` 不输出。
- 本地图片复制到输出 `figures/` 并改写路径；远程 URL 与找不到的文件保持原样并告警。docx 与表格单元格内不支持插图，降级为替代文本。

## 交叉引用（tex）

被引对象尾部加 `{#id}` 打锚点，正文用 `{@id}` 引用；tex 编译为 `\label`/`\ref`，编号由 LaTeX 自动生成。

```markdown
## 概述 {#chap:overview}

![系统架构图](pics/arch.png){#fig:arch}

| 名称 | 数量 |
|------|------|
: 产品清单 {#tbl:products}

详见第 {@chap:method} 章，架构如图 {@fig:arch} 所示，清单见表 {@tbl:products}。
```

- 锚点位置：标题行尾、图片语法之后、表题行尾。
- `{@id}` 只输出编号；"第 X 章 / 图 X / 表 X" 的中文措辞由作者围绕引用手写。
- id 规则：字母开头，可含字母、数字、`:`、`.`、`_`、`-`；建议前缀 `chap:`/`sec:`/`fig:`/`tbl:`（约定，不强制）。
- **生效范围**：research 完整支持（章/节/图/表）；official 仅图片锚点有效；docx 不支持，降级为 id 文本。
- **硬错误（停止转换）**：引用未定义/当前样式无效的锚点、锚点重复定义、锚点无法生效（未挂到标题/表格、图缺替代文本、图未独占一段、official 下的章节/表格锚点）。锚点已定义但未引用仅警告。

## 文献引用（tex，BibTeX）

front matter 声明**一个** Bib 文件，正文用方括号引用：

```markdown
---
bibliography: refs/library.bib
---

已有研究给出了相同结论 [@zhang2024]。两种方法可结合 [@li2023; @wang2022]。
```

- `[@key]` → `\cite{key}`；`[@a; @b]` → `\cite{a,b}`。支持段落、列表项、表格单元格；**标题、行内代码、代码块内不解析**。
- Bib 路径相对单文件所在目录（目录输入时相对输入目录）。两种 tex 都用 `biblatex` 的 `gb7714-2015` 样式；docx 保留 `[@key]` 原文。
- **只支持**上述两种方括号形式和**一个** Bib 文件。不支持裸 `@key`、`[-@key]`、页码、前后缀、多 Bib 文件。
- **硬错误（停止转换）**：正文有 citation 但未声明 `bibliography`、Bib 路径不存在/不可读、格式错/重复 key、引用 key 不存在。声明了 Bib 即使正文无引用也会校验；未引用条目不报错。

## 表格与表题

GFM 管道表格；表题三选一：**表前一行** `Table: 标题`/`表: 标题`/`表：标题`，或**表后一行** `: 标题`。

```markdown
| 列A | 列B |
|-----|-----|
| 1   | 2   |
: 产品清单 {#tbl:products}
```

- 带编号的写法（`表1：标题`、`Table 1: 标题`）可识别但**编号被剥除**，勿手写编号。
- research 输出 `longtblr`（单元格支持 `**粗体**`、`*斜体*`，脚注降级为全角括号内联）；official 输出全边框 `longtable`（可跨页、表头续页重复）。要被引用的表**必须有表题**（无表题计数器不递增）。

## 行内格式

| 写法 | 输出 |
|---|---|
| `**加粗**` | 粗体（**至少 4 字符**才识别） |
| `*斜体*` | 斜体（中文用楷体） |
| `` `代码` `` | 等宽 |
| `[文字](url)` | tex：`\href`；docx：纯文字 |
| `[^id]:(注释内容)` | 行内脚注，自动编号；冒号、括号兼容全角 `：`/`（）` |

- 脚注写在正文中间：`这是一段正文[^1]:(第一条注释)继续正文。`
- 裸 `[^id]`（无 `:(内容)`）**不识别**，按普通文字输出——引用式脚注不支持。
- 表格单元格内与 docx 中脚注降级为全角括号内联 `（注释内容）`。
- 中英文直引号会被全局正规化为中文圆引号；代码块内保持原样。

## 列表

```markdown
- 无序项
* 亦可
1. 有序项
```

- **每行一个列表项**，不写多段落或嵌套子列表。编号/符号前缀由输出端按层级循环生成，**不要手写编号**。
- 层级由前导空白推断：0→1 级，1–4 空格→2 级，5–8→3 级，9–12→4 级，13–16→5 级，更多→6 级。
- tex 输出**包成 `paralist` 环境**（公文和研究报告两种样式一致）：
  - 1 级 → `\begin{asparaenum}`，每项独占一段（首行缩进 `一二` 字宽）
  - 2..6 级 → `\begin{inparaenum}`，所有项紧排在一段里
- tex 标签循环：L1 ⑴⑵⑶ → L2 ①②③ → L3 (A)(B) → L4 (a)(b) → L5 I.II. → L6 i.ii.。
- docx 仍按层级输出前缀字符（公文 ①②⑴，研报 1./(1)/a.）。

## 代码块

````markdown
```rust
fn main() {}
```
````

- 信息串第一个词作语言（research tex 映射 listings；未知语言按纯文本）。official tex **不支持代码块（忽略）**。

## 明确不支持的语法（勿用）

数学公式、引用式脚注（`[^id]` + 单独定义行）、blockquote（`>`）、raw LaTeX/HTML、嵌套子列表、多段落列表项、复杂表格（合并单元格等）、完整 inline 嵌套。使用这些的输出不可预期。

## 写完后的自检清单

- [ ] 标题、表题、列表里**没有手写编号**。
- [ ] research 的 front matter `---` 已闭合、写在首个文件顶部。
- [ ] 每个 `{@id}` 都有对应 `{#id}`；被引图独占一段且有替代文本；被引表有表题。
- [ ] 有 `[@key]` 时 front matter 声明了 `bibliography`，且 key 在 Bib 中存在。
- [ ] 没有多段落/嵌套列表、blockquote、数学公式、raw HTML/LaTeX。
- [ ] 交叉引用与文献引用是 tex 专属；若目标是 docx，知道它们会降级。
