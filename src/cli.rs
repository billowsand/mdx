use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mdx")]
#[command(version, about = "Unified markdown converter: docx/tex × official/research")]
#[command(long_about = r#"统一 markdown 转换工具，覆盖 (docx | tex) × (official | research) 四种组合。

输入可以是单个 .md 文件，也可以是目录（按文件名升序合并所有 .md）。

样式说明：
  official  公文风格（仿宋/黑体/楷体；"一、（一）1."编号；公文页边距）
  research  研究报告风格（ctexbook 论文模板；"第X章 / X.Y / X.Y.Z"编号；含封面、目录）

示例：
  mdx docx report.md --style official -o report.docx
  mdx docx ./chapters --style research -o paper.docx
  mdx tex  report.md --style official -o report.tex
  mdx tex  ./chapters --style research -o paper.tex
"#)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Format,
}

#[derive(Subcommand)]
pub enum Format {
    /// 输出 .docx
    Docx {
        /// 输入 .md 文件或包含 .md 的目录
        input: PathBuf,
        /// 文档样式
        #[arg(short, long, value_enum)]
        style: Style,
        /// 输出 .docx 路径（默认按输入名生成）
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// 输出 .tex
    Tex {
        /// 输入 .md 文件或包含 .md 的目录
        input: PathBuf,
        /// 文档样式
        #[arg(short, long, value_enum)]
        style: Style,
        /// 输出 .tex 路径（默认按输入名生成）
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// 仅 research：可选自定义 LaTeX 模板路径（覆盖内置）
        #[arg(short, long)]
        template: Option<PathBuf>,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Style {
    /// 公文风格
    Official,
    /// 研究报告风格
    Research,
}
