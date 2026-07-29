use anyhow::Result;
use clap::Parser;

mod cli;

use cli::{Cli, Format, Style};
use mdx::{ConvertRequest, DocumentStyle, OutputFormat};

fn main() {
    if let Err(e) = run() {
        eprintln!("[错误] {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Cli::parse();
    let request = match args.command {
        Format::Docx {
            input,
            style,
            output,
        } => ConvertRequest {
            input,
            output,
            format: OutputFormat::Docx,
            style: map_style(style),
            template: None,
            compile_pdf: false,
        },
        Format::Tex {
            input,
            style,
            output,
            template,
        } => ConvertRequest {
            input,
            output,
            format: OutputFormat::Tex,
            style: map_style(style),
            template,
            compile_pdf: true,
        },
    };

    mdx::convert_with_progress(request, |event| {
        if event.level == mdx::ProgressLevel::Warning {
            eprintln!("[警告] {}", event.message);
        }
    })?;
    Ok(())
}

fn map_style(style: Style) -> DocumentStyle {
    match style {
        Style::Official => DocumentStyle::Official,
        Style::Research => DocumentStyle::Research,
    }
}
