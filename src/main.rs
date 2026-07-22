use anyhow::Result;
use clap::Parser;

mod cli;
mod common;
mod docx_official;
mod docx_research;
mod input;
mod parser;
mod tex_compile;
mod tex_official;
mod tex_research;
mod tex_research_emitter;

use cli::{Cli, Format, Style};

fn main() {
    if let Err(e) = run() {
        eprintln!("[错误] {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Format::Docx { input, style, output } => match style {
            Style::Official => docx_official::run(&input, output.as_deref()),
            Style::Research => docx_research::run(&input, output.as_deref()),
        },
        Format::Tex {
            input,
            style,
            output,
            template,
        } => match style {
            Style::Official => {
                if template.is_some() {
                    eprintln!("[警告] 公文 → tex 不支持 --template 参数，已忽略");
                }
                tex_official::run(&input, output.as_deref())
            }
            Style::Research => {
                tex_research::run(&input, output.as_deref(), template.as_deref())
            }
        },
    }
}
