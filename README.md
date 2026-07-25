# mdx — 统一 markdown 转换器

把 markdown 转成 docx 或 tex，覆盖**公文**与**研究报告**两套样式，共 4 种组合。

## 4 种组合

```
mdx docx <input> --style official  -o out.docx   # 公文 docx
mdx docx <input> --style research  -o out.docx   # 研究报告 docx（带封面+目录+章节编号）
mdx tex  <input> --style official  -o out.tex    # 公文 tex；检测到 TeX 引擎时自动编译 PDF
mdx tex  <input> --style research  -o out.tex    # 研究报告 tex（纯 Rust + ctexbook）；自动编译 PDF
```

`<input>` 可以是单个 `.md` 文件，也可以是包含 `.md` 的目录（按文件名升序合并）。

## 编译

```bash
cargo build --release
# 二进制位于 target/release/mdx
```

## 二进制依赖

| 子命令 | 依赖 |
|---|---|
| `docx official` | 无 |
| `docx research` | 无 |
| `tex  official` | 转换时无外部依赖；PDF 编译需要 tectonic 或 xelatex，有参考文献时 xelatex 路径需要 biber |
| `tex  research` | 转换时无外部依赖；PDF 编译需要 tectonic 或 xelatex，有参考文献时 xelatex 路径需要 biber |

`tex research` 不再调用 pandoc。两种 tex 样式都支持 BibTeX 文献引用；转换阶段由
Rust 内生解析器检查 Bib 文件，不依赖外部程序。

## BibTeX 文献引用（tex）

在 Markdown 顶部声明一个 Bib 文件：

```markdown
---
bibliography: refs/library.bib
---

已有研究给出了相同结论 [@zhang2024]，也可同时引用多篇文献 [@li2023; @wang2022]。
```

Bib 路径相对于单个 Markdown 文件所在目录；目录输入时相对于输入目录。转换结果分别
使用 `\cite{zhang2024}`、`\cite{li2023,wang2022}`，并把 Bib 文件复制到输出目录，
统一命名为 `references.bib`。只支持上述方括号形式，不支持裸 `@key`、页码、前后缀
或多个 Bib 文件。

出现引用但未声明 `bibliography`、Bib 文件不可读或格式错误、存在重复 key、引用 key
不存在时，转换会在写出 tex/PDF、图片或分章文件前停止。Bib 中未引用的条目允许保留，
生成的参考文献表只包含正文实际引用的条目。

## tex 输出布局（分章 + 图片）

`tex` 子命令生成的是一个主 `.tex` 文件，正文按章拆分为子文件，主文件用 `\input{...}` 按序引用：

```
out/
├── main.tex                  # 主文件：封面/摘要/前置内容 + \input 引用
├── data/                     # 正文章节（research: chapterNN.tex；official: sectionNN.tex）
│   ├── chapter01.tex
│   └── chapter02.tex
├── appendix/                 # 附录章节（仅 research，<!-- [附录] --> 之后）
│   └── appendix01.tex
├── references.bib            # 有文献引用时，从 front matter 声明的 Bib 文件复制
└── figures/                  # markdown 引用的本地图片复制至此，引用路径自动改写
```

- research 按 `\chapter`（H1/H2）拆分；official 按 H2（"一、"级节）拆分。摘要、版本变更记录、参考文献等留在主文件。
- 图片语法 `![替代文本](路径)`：独占一段时输出 `figure` 环境（`\includegraphics[width=\textwidth]`，替代文本作为 `\caption`）；行内混排时输出裸 `\includegraphics`。本地图片复制到 `figures/` 并把引用改写为 `figures/<文件名>`（同名不同源自动加序号后缀）；远程 URL 与找不到的文件保持原样并告警。docx 与表格单元格内暂不支持插图，降级为替代文本。

TeX 输出完成后会自动检测 `tectonic`，没有则检测 `xelatex`。检测到可用引擎时会直接生成同名 `.pdf`，并清理同名 LaTeX 中间产物（如 `.aux`、`.log`、`.toc`、`.bcf`、`.run.xml` 等）。常规输出目录最终只需要保留 `.md`、`.tex`、`.cls` 和 `.pdf`；有文献引用时还会保留复制后的 `references.bib`。

## 字体

如果系统未安装下面任何字体，docx 端只是写入字体名（Word 端找不到时回退到默认字体）；tex 端通过 `\IfFontExistsTF` 宏自动尝试 fallback 字体。建议按需安装。

**公文样式**：

- 仿宋_GB2312（FangSong_GB2312） — 正文、段落、表格、列表
- 黑体（SimHei / FZHei-B01） — 二级标题
- 楷体_GB2312（KaiTi_GB2312 / KaiTi） — 三级标题
- 方正小标宋简体（FZXiaoBiaoSong-B05） — 一级标题（公文红头）

**研究报告样式**：

- 仿宋_GB2312 / FZShuSong-Z01 — 正文
- 黑体 / FZHei-B01 — 章节标题
- 方正小标宋简体 / FZXiaoBiaoSong-B05 — 封面标题

## Markdown 支持范围

扩展标记的完整使用约定见 [docs/markdown-extensions.md](docs/markdown-extensions.md)，撰写新文档时请遵循。

当前 Markdown 解析为 Rust 内生实现，覆盖本项目常用子集：标题、段落、基础行内格式、普通链接、图片、行内脚注、Pandoc 方括号文献引用、列表行、fenced code block、GFM 管道表格、pandoc 风格表格标题，以及 research 区段标记。

尚未等价覆盖 pandoc 全量语法，例如数学公式、引用式脚注（`[^id]` + 单独定义行）、blockquote、raw LaTeX/HTML、复杂嵌套列表、多段落列表、复杂表格和完整 inline 嵌套规则。

## 行内脚注（`tex`）

正文中写 `[^id]:(注释内容)`，冒号与括号均兼容全角（`：` / `（）`）：

```markdown
这是一段正文[^1]:(第一条注释)继续正文。
```

- `tex official` / `tex research`：转为 `\footnote{...}`，页脚自动编号，`id` 仅作占位、不参与编号。
- 表格单元格内（`tabular` / `longtblr` 中 `\footnote` 不生效）降级为全角括号内联注释 `（注释内容）`。
- `docx`：暂不生成 Word 脚注部件，同样降级为全角括号内联注释。
- 只有 `[^id]` 而没有 `:(内容)` 时不识别为脚注，按普通文字原样输出。

## 区段标记（`research`）

研究报告 docx / tex 支持 markdown 行内 HTML 注释作为区段切换标记：

```markdown
<!-- [摘要] -->        # 进入摘要段；此后段落不计入章节计数
<!-- [版本变更记录] --> # 进入版本变更记录段；提取到目录前单独输出
<!-- [正文] -->        # 恢复正常 "第X章" 编号
<!-- [参考文献] -->     # 进入参考文献段；不编号
<!-- [附录] -->        # 进入附录段；后续 H2 编号切换为 "附录 A / B / ..."
                      # （若附录用 H1 开章，如 "# 附录A ..."，则 H1 为附录章，H2 起下移为节）
```

如果文档不带任何标记，所有 H2 按 `第一章 / 第二章 / ...` 顺序编号。

## 表格标题（`tex research`）

研究报告 tex 支持 pandoc 常用表格标题语法：

```markdown
Table: 实验结果

| 指标 | 数值 |
|---|---|
| A | 1.0 |
```

也支持表后标题：

```markdown
| 指标 | 数值 |
|---|---|
| A | 1.0 |

: 实验结果
```

中文写法 `表: 标题` / `表：标题` 也可识别。输出会生成 `longtblr` 的 `caption={...}`。
带编号的标记（`表1：标题`、`表E.1：标题`、`Table 1: 标题`）以及表题开头残留的
编号（`: 4.6 标题`）会被剥除，编号统一由 LaTeX 表格计数器自动生成。

## 封面字段（`tex research`）

研究报告 tex 的封面包含：密级、文件类型、文件编号、文件版本号、文件名称、
撰写单位、撰写时间（到月）。在 markdown 文件（目录模式为排序后的第一个文件）
顶部用 front matter 提供：

```markdown
---
密级: 内部
文件类型: 技术报告
文件编号: XX-2026-001
版本: V2.1
撰写单位: 某研究所
撰写时间: 2026-07
---
```

键名支持中英文冒号；`版本`/`单位`/`时间`/`标题` 可替代 `文件版本号`/`撰写单位`/
`撰写时间`/`文件名称`。`标题`（或 `文件名称`）会覆盖第一个 `#` 标题作为文件名称。
缺省值：密级"公开"、文件类型"研究报告"、版本"V1.0"、单位"某某单位"、撰写时间取
编译时的当前年月；`撰写时间` 支持 `2026-07` / `2026/7` / `2026年7月` 写法。

## 标题映射

| markdown | 公文 docx / tex | 研究报告 docx | 研究报告 tex |
|---|---|---|---|
| `#` | 居中大标题（方正小标宋简体） | 封面标题 | 封面标题 `\papertitle` |
| `##` | "一、X"（黑体） | "第X章 X"（Heading1） | `\chapter{}` |
| `###` | "（一）X"（楷体） | "X.Y X"（Heading2） | `\section{}` |
| `####` | "1.X"（仿宋） | "X.Y.Z X"（Heading3） | `\subsection{}` |
| `#####` | "(1)X"（仿宋粗体） | — | `\subsubsection{}` |

公文 4 种编号格式（"一、（一）1.(1)"）以及 6 级列表前缀循环
（①②③ → ⑴⑵⑶ → a.b.c. → I.II.III. → (A)(B) → 1)2)）在 docx / tex 两个 emitter 中输出一致。

## 示例

```bash
# 公文
mdx docx 通知.md --style official -o 通知.docx
mdx tex  通知.md --style official -o 通知.tex
# 若系统存在 tectonic 或 xelatex，会自动生成 通知.pdf

# 研究报告（单文件）
mdx docx 报告.md --style research -o 报告.docx
mdx tex  报告.md --style research -o 报告.tex
# 若系统存在 tectonic 或 xelatex，会自动生成 报告.pdf

# 研究报告（多文件，按文件名排序合并）
mdx docx ./chapters --style research -o 报告.docx
mdx tex  ./chapters --style research -o 报告.tex

# 研究报告 tex 自定义模板
mdx tex 报告.md --style research --template my_template.tex -o 报告.tex
```

研究报告 docx 包含原生 Word 目录字段，**Word 中打开后**右键目录 →
"更新整个目录"即可填出条目。

## 项目结构

```
mdx/
├── Cargo.toml
├── README.md
├── resources/
│   ├── official/official.cls          # 公文 LaTeX 类
│   └── research/                       # 研究报告 LaTeX 资源
│       ├── template.tex
│       └── md2tex.cls
└── src/
    ├── main.rs cli.rs input.rs parser.rs
    ├── common/         # IR + 引号/标题/编号/表格/标记 共享逻辑
    ├── docx_official.rs
    ├── docx_research.rs
    ├── tex_official.rs
    ├── tex_research/   # research tex 资源/模板包装
    └── tex_research_emitter.rs
```

## 测试

```bash
cargo test
```

覆盖 parser、numbering、heading 清理、tex 转义、表格 caption、longtblr、模板渲染、biblatex 清理、research tex emitter、研报 docx 切分等。
