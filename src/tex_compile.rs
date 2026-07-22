use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy)]
enum TexEngine {
    Tectonic,
    Xelatex,
}

/// Compile a generated `.tex` file to PDF when a supported TeX engine exists.
///
/// This is best-effort for engine discovery: if neither `tectonic` nor `xelatex`
/// is available, conversion still succeeds and leaves the `.tex` for manual
/// compilation.
pub fn compile_pdf_if_available(tex_path: &Path) -> Result<()> {
    let Some(engine) = detect_engine() else {
        println!("  未检测到 tectonic 或 xelatex，跳过 PDF 编译");
        return Ok(());
    };

    match engine {
        TexEngine::Tectonic => compile_with_tectonic(tex_path)?,
        TexEngine::Xelatex => compile_with_xelatex(tex_path)?,
    }

    let pdf_path = tex_path.with_extension("pdf");
    if !pdf_path.exists() {
        anyhow::bail!("TeX 编译完成但未找到 PDF: {}", pdf_path.display());
    }

    cleanup_intermediates(tex_path)?;
    Ok(())
}

fn detect_engine() -> Option<TexEngine> {
    if command_available("tectonic") {
        return Some(TexEngine::Tectonic);
    }
    if command_available("xelatex") {
        return Some(TexEngine::Xelatex);
    }
    None
}

fn command_available(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn compile_with_tectonic(tex_path: &Path) -> Result<()> {
    let dir = output_dir(tex_path);
    let file_name = tex_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("无效的 tex 文件路径: {}", tex_path.display()))?;

    println!("  检测到 tectonic，正在编译 PDF...");
    run_command(
        Command::new("tectonic").current_dir(dir).arg(file_name),
        "tectonic",
    )?;
    println!("  PDF 编译完成");
    Ok(())
}

fn compile_with_xelatex(tex_path: &Path) -> Result<()> {
    let dir = output_dir(tex_path);
    let file_name = tex_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("无效的 tex 文件路径: {}", tex_path.display()))?;
    let stem = tex_path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or_else(|| anyhow::anyhow!("无效的 tex 文件名: {}", tex_path.display()))?;

    println!("  检测到 xelatex，正在编译 PDF...");
    run_xelatex(dir, file_name)?;

    if tex_uses_biblatex(tex_path) && command_available("biber") {
        println!("  检测到 biblatex 和 biber，正在处理参考文献...");
        run_command(Command::new("biber").current_dir(dir).arg(stem), "biber")?;
    }

    run_xelatex(dir, file_name)?;
    println!("  PDF 编译完成");
    Ok(())
}

fn run_xelatex(dir: &Path, file_name: &OsStr) -> Result<()> {
    run_command(
        Command::new("xelatex")
            .current_dir(dir)
            .arg("-interaction=nonstopmode")
            .arg("-halt-on-error")
            .arg(file_name),
        "xelatex",
    )
}

fn run_command(command: &mut Command, label: &str) -> Result<()> {
    let output = command
        .output()
        .with_context(|| format!("执行 {} 失败", label))?;
    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    anyhow::bail!(
        "{} 编译失败\nstdout:\n{}\nstderr:\n{}",
        label,
        stdout.trim(),
        stderr.trim()
    )
}

fn tex_uses_biblatex(tex_path: &Path) -> bool {
    fs::read_to_string(tex_path)
        .map(|content| content.contains("\\usepackage") && content.contains("{biblatex}"))
        .unwrap_or(false)
}

fn cleanup_intermediates(tex_path: &Path) -> Result<()> {
    let dir = output_dir(tex_path);
    let stem = tex_path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or_else(|| anyhow::anyhow!("无效的 tex 文件名: {}", tex_path.display()))?;

    let generated_extensions = [
        "aux",
        "bbl",
        "bcf",
        "blg",
        "fdb_latexmk",
        "fls",
        "log",
        "out",
        "run.xml",
        "synctex.gz",
        "toc",
        "xdv",
    ];

    for ext in generated_extensions {
        let path = generated_path(dir, stem, ext);
        if path.exists() {
            fs::remove_file(&path).with_context(|| format!("删除 {} 失败", path.display()))?;
        }
    }

    Ok(())
}

fn output_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

fn generated_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    if ext.contains('.') {
        dir.join(format!("{}.{}", stem, ext))
    } else {
        dir.join(stem).with_extension(ext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_dir_defaults_to_current_dir_for_bare_file_name() {
        assert_eq!(output_dir(Path::new("report.tex")), Path::new("."));
        assert_eq!(output_dir(Path::new("./report.tex")), Path::new("."));
        assert_eq!(output_dir(Path::new("out/report.tex")), Path::new("out"));
    }

    #[test]
    fn cleanup_removes_latex_sidecars_for_same_stem() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tex = dir.path().join("report.tex");
        fs::write(&tex, "").unwrap();
        fs::write(dir.path().join("report.aux"), "").unwrap();
        fs::write(dir.path().join("report.log"), "").unwrap();
        fs::write(dir.path().join("report.run.xml"), "").unwrap();
        fs::write(dir.path().join("other.aux"), "").unwrap();
        fs::write(dir.path().join("report.pdf"), "").unwrap();
        fs::write(dir.path().join("md2tex.cls"), "").unwrap();

        cleanup_intermediates(&tex).expect("cleanup");

        assert!(!dir.path().join("report.aux").exists());
        assert!(!dir.path().join("report.log").exists());
        assert!(!dir.path().join("report.run.xml").exists());
        assert!(dir.path().join("other.aux").exists());
        assert!(dir.path().join("report.tex").exists());
        assert!(dir.path().join("report.pdf").exists());
        assert!(dir.path().join("md2tex.cls").exists());
    }
}
