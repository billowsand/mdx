use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub const TEMPLATE_TEX: &str = include_str!("../../resources/research/template.tex");
pub const MD2TEX_CLS: &str = include_str!("../../resources/research/md2tex.cls");

/// Extract runtime resources required next to the generated `.tex` file.
pub fn extract(output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir).with_context(|| "创建输出目录失败")?;
    fs::write(output_dir.join("md2tex.cls"), MD2TEX_CLS).with_context(|| "释放 md2tex.cls 失败")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_class_uses_separate_latin_and_cjk_code_fonts() {
        assert!(MD2TEX_CLS.contains("\\newcommand{\\codefont}{JetBrains Mono}"));
        assert!(MD2TEX_CLS.contains("\\newcommand{\\codeCJKfont}{FZKai-Z03}"));
        assert!(MD2TEX_CLS.contains("\\newcommand{\\maintitle}{FZXiaoBiaoSong-B05S}"));
        assert!(MD2TEX_CLS.contains("\\setmonofont{\\codefont}"));
        assert!(MD2TEX_CLS.contains("\\setCJKfamilyfont{code}{\\codeCJKfont}"));
        assert!(!MD2TEX_CLS.contains("LXGW Bright Code"));
    }
}
