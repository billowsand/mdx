use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub const TEMPLATE_TEX: &str = include_str!("../../resources/research/template.tex");
pub const MD2TEX_CLS: &str = include_str!("../../resources/research/md2tex.cls");

/// Extract runtime resources required next to the generated `.tex` file.
pub fn extract(output_dir: &Path) -> Result<()> {
    fs::write(output_dir.join("md2tex.cls"), MD2TEX_CLS).with_context(|| "释放 md2tex.cls 失败")?;
    Ok(())
}
