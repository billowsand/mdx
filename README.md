<div align="center">

# mdx

**面向中文公文与研究报告的 Markdown → DOCX / TeX 转换器**

一次写作，四种专业排版组合。提供桌面界面与命令行，纯 Rust 解析，无需 Pandoc。

[![CI](https://github.com/billowsand/mdx/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/billowsand/mdx/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/billowsand/mdx?display_name=tag&sort=semver)](https://github.com/billowsand/mdx/releases/latest)
[![License](https://img.shields.io/badge/license-MIT-2ea44f.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-dea584?logo=rust)](https://www.rust-lang.org/)

[快速开始](#快速开始) · [功能概览](#功能概览) · [语法文档](docs/markdown-extensions.md) · [示例](examples/) · [参与贡献](CONTRIBUTING.md)

</div>

## 为什么选择 mdx？

通用转换器擅长覆盖大量格式，但中文公文和研究报告往往需要固定的字体、字号、页边距、章节编号、封面与目录。mdx 将这些规则固化为两套可直接使用的样式，并让 DOCX 与 TeX 输出共享同一套 Markdown 语义。

| | `official` 公文 | `research` 研究报告 |
|---|---|---|
| **DOCX** | 公文字体与“四级标题”编号 | 封面、目录、章节编号 |
| **TeX / PDF** | 内置公文 LaTeX 类 | `ctexbook` 研报模板、分章输出 |

- **四种输出组合**：`docx/tex × official/research`。
- **桌面与命令行双入口**：`mdx-gui` 提供文件选择、拖放和任务日志，`mdx` 适合脚本与自动化。
- **纯 Rust 转换**：转换阶段不依赖 Pandoc；DOCX 输出无运行时外部依赖。
- **面向中文排版**：内置公文和研究报告常用字体、字号及编号规则。
- **结构化长文档**：支持目录输入、章节拆分、附录、封面字段和版本记录。
- **专业写作能力**：支持图片、表格、行内脚注、交叉引用与 BibTeX 文献引用。
- **谨慎失败**：文献或交叉引用校验失败时，在写出不完整 TeX/PDF 前停止。

## 快速开始

### 安装

#### 下载预编译版本（推荐）

从 [GitHub Releases](https://github.com/billowsand/mdx/releases/latest) 下载与你的平台对应的压缩包。压缩包包含：

- `mdx-gui`（Windows 为 `mdx-gui.exe`）：双击运行桌面界面；
- `mdx`（Windows 为 `mdx.exe`）：命令行程序，可将其目录加入 `PATH`。

发布工作流提供 Windows x86_64、Linux x86_64/ARM64、macOS Intel/Apple Silicon 构建及 `SHA256SUMS.txt`。

#### 从源码安装

需要 Rust 1.85 或更高版本：

```bash
git clone https://github.com/billowsand/mdx.git
cd mdx
cargo install --path . --locked
mdx --version
```

也可在项目根目录直接构建：

```bash
# 只构建轻量 CLI
cargo build --release --locked

# 同时构建 CLI 与桌面界面
cargo build --release --locked --all-features
```

二进制位于 `target/release/`。GUI 使用内嵌的 `font/sfss.ttf` 显示中文。

### 第一次转换

#### 桌面界面

运行 `mdx-gui`，选择或拖入 `.md` 文件/目录，然后选择输出格式和文档样式并点击
“开始转换”。TeX 模式可选择是否调用系统中的 XeLaTeX/Tectonic 同时生成 PDF；
研究报告 TeX 还可指定自定义模板。转换在后台执行，完成后可直接打开输出位置。

从源码直接启动界面：

```bash
cargo run --release --bin mdx-gui --features gui
```

#### 命令行

```bash
# 中文公文 DOCX
mdx docx examples/official.md --style official -o output/official.docx

# 研究报告 DOCX
mdx docx examples/research.md --style research -o output/research.docx

# 中文公文 TeX；检测到 TeX 引擎时同时生成 PDF
mdx tex examples/official.md --style official -o output/official.tex

# 研究报告 TeX / PDF
mdx tex examples/research.md --style research -o output/research.tex
```

`<input>` 可以是单个 `.md` 文件，也可以是目录。目录模式会读取顶层 Markdown，并按文件名升序合并；推荐用 `01-intro.md`、`02-body.md` 控制顺序。

省略 `-o` 时，TeX 输出会连同 `data/`、`figures/`、`.cls`、`.bib` 与 PDF 一起收进一个单独的目录，不再散落在当前目录：

| 输入 | 默认 TeX 输出 | 默认 DOCX 输出 |
|---|---|---|
| `report.md` | `report/report.tex` | `report.docx` |
| `chapters/` | `chapters-tex/chapters.tex` | `chapters.docx` |

目录输入的输出目录带 `-tex` 后缀，避免与输入目录同名冲突；DOCX 是自包含单文件，仍直接生成在当前目录。

## 命令行速查

```text
mdx docx <input> --style <official|research> [-o <output.docx>]
mdx tex  <input> --style <official|research> [-o <output.tex>] [--template <template.tex>]
```

| 参数 | 说明 |
|---|---|
| `<input>` | Markdown 文件，或包含 Markdown 的目录 |
| `--style official` | 使用中文公文样式 |
| `--style research` | 使用研究报告样式 |
| `-o, --output` | 指定输出路径；省略时按输入名称生成（TeX 另建同名目录收纳配套文件） |
| `--template` | 仅 `tex research`：覆盖内置 LaTeX 模板 |
| `-h, --help` | 查看帮助 |
| `-V, --version` | 查看版本 |

## 功能概览

### 中文公文（official）

- H1 为居中大标题，H2–H5 自动生成“一、”“（一）”“1.”“(1)”编号。
- 正文、标题、列表与表格应用仿宋、黑体、楷体、小标宋等公文字体约定。
- DOCX 直接生成；TeX 使用内置 `official.cls`。

### 研究报告（research）

- 从 front matter 读取密级、文件类型、编号、版本、单位、日期与文档名称。
- 自动生成封面、目录、“第 X 章 / X.Y / X.Y.Z”编号和附录字母编号。
- TeX 正文章节拆分到 `data/`，附录拆分到 `appendix/`，便于维护大型文档。
- DOCX 包含原生 Word 目录字段；打开文档后右键目录并选择“更新整个目录”。

### 表格、图片、引用与脚注

- GFM 管道表格，以及 `Table:`、`表：`、表后 `: 标题`形式的表题。
- 本地图片会复制到 `figures/`，同名文件自动去重；远程或缺失图片保留原路径并告警。
- TeX 支持 `{#fig:id}` / `{@fig:id}` 形式的图、表、章节交叉引用。
- TeX 支持 front matter 声明单个 BibTeX 文件，以及 `[@key]` / `[@a; @b]` 文献引用。
- `[^id]:(注释内容)` 生成 TeX 脚注；DOCX 和表格单元格降级为括号内联注释。

完整写法、边界条件和不支持项请查阅 [Markdown 扩展标记使用说明](docs/markdown-extensions.md)。

## TeX / PDF 依赖

生成 `.tex` 本身不需要外部转换工具。完成 TeX 输出后，mdx 会按以下顺序尝试生成 PDF：

1. 优先检测 `xelatex`；
2. 不可用时回退到 `tectonic`；
3. 两者都不存在时保留 `.tex`，转换仍成功。

有 BibTeX 引用且使用 XeLaTeX 时，系统还应提供 `biber`。TeX 样式依赖常见中文字体；若指定字体不存在，内置类会尝试 fallback 字体。

| 输出 | 转换阶段依赖 | 可选 PDF 依赖 |
|---|---|---|
| `docx official` | 无 | — |
| `docx research` | 无 | — |
| `tex official` | 无 | XeLaTeX 或 Tectonic；文献引用建议 Biber |
| `tex research` | 无 | XeLaTeX 或 Tectonic；文献引用建议 Biber |

## Markdown 支持范围

mdx 使用面向本项目场景的 Rust 原生解析器，而不是完整 CommonMark/Pandoc 实现。

已覆盖：

- 标题、段落、基础行内格式、链接与图片；
- 单行列表、fenced code block、GFM 管道表格；
- 行内脚注、交叉引用、Pandoc 方括号文献引用；
- research 区段标记、封面字段与目录输入合并。

尚未完整覆盖：

- 数学公式、引用式脚注、blockquote、raw LaTeX/HTML；
- 多段落或复杂嵌套列表、合并单元格等复杂表格；
- Pandoc/CommonMark 的全部行内嵌套规则。

在生产文档中使用前，请以 [语法文档](docs/markdown-extensions.md) 为准，并先转换一个最小样例确认版式。

## TeX 输出结构

主文件与全部配套文件都在同一个目录内，可整体拷走或交付：

```text
report/                      # 默认目录名：文件输入取文件名，目录输入取“目录名-tex”
├── report.tex               # 主文件：封面、摘要、目录等
├── md2tex.cls               # research 样式资源
├── data/                    # 正文章节
│   ├── chapter01.tex
│   └── chapter02.tex
├── appendix/                # 附录章节（research）
│   └── appendix01.tex
├── references.bib           # 使用文献引用时生成
├── figures/                 # 本地图片副本
└── report.pdf               # 检测到可用 TeX 引擎时生成
```

## 开发与验证

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release --locked --all-features
```

测试覆盖解析、编号、TeX 转义、表格、文献与交叉引用校验、资源渲染、章节拆分、
DOCX 生成及 GUI 路径处理逻辑。GitHub Actions 会在 Linux、Windows 和 macOS 上构建
CLI 与 GUI，并在 Linux 上执行完整质量检查。

## 路线图

- [ ] 数学公式与更完整的脚注语法
- [ ] 复杂嵌套列表和多段落列表项
- [ ] 更丰富的 DOCX 图片及交叉引用支持
- [ ] 可复现的视觉回归样例与输出预览

欢迎先提交 [Feature request](https://github.com/billowsand/mdx/issues/new/choose) 讨论设计与兼容性。

## 参与项目

- 问题与功能建议：使用仓库中的 Issue 表单；
- 代码贡献：阅读 [CONTRIBUTING.md](CONTRIBUTING.md)；
- 安全漏洞：遵循 [SECURITY.md](SECURITY.md) 私下报告；
- 版本变化：参见 [CHANGELOG.md](CHANGELOG.md)。

## 许可证

本项目基于 [MIT License](LICENSE) 开源。
