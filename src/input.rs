use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use walkdir::WalkDir;

/// 校验输入路径并区分单文件/目录
pub fn classify(input: &Path) -> Result<InputKind> {
    if !input.exists() {
        anyhow::bail!("找不到输入路径 '{}'", input.display());
    }
    if input.is_dir() {
        return Ok(InputKind::Directory);
    }
    if input.is_file()
        && input
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("md"))
            .unwrap_or(false)
    {
        return Ok(InputKind::File);
    }
    anyhow::bail!("'{}' 必须是目录或 .md 文件", input.display());
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InputKind {
    File,
    Directory,
}

/// 把输入路径合并成单一字符串（目录则按文件名升序拼接所有 .md）
pub fn collect(input: &Path) -> Result<String> {
    let raw = match classify(input)? {
        InputKind::File => fs::read_to_string(input)
            .with_context(|| format!("读取文件 {} 失败", input.display()))?,
        InputKind::Directory => merge_dir(input)?,
    };
    Ok(strip_horizontal_rules(&raw))
}

/// 删除整行只剩 `---` / `----` …… 的水平分隔线。
///
/// 表格分隔行（如 `|---|---|`）以 `|` 起始，正则不会命中；不影响表格解析。
fn strip_horizontal_rules(content: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^\s*-{3,}\s*$").expect("invalid hr regex"));
    content
        .lines()
        .filter(|line| !re.is_match(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn merge_dir(dir: &Path) -> Result<String> {
    let mut md_files: Vec<PathBuf> = WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("md"))
                    .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    if md_files.is_empty() {
        anyhow::bail!("在目录 '{}' 中未找到任何 .md 文件", dir.display());
    }

    md_files.sort_by(|a, b| {
        let na = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let nb = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
        na.cmp(nb)
    });

    println!("找到 {} 个 markdown 文件:", md_files.len());
    for f in &md_files {
        println!("  - {}", f.file_name().unwrap_or_default().to_string_lossy());
    }

    let mut merged = String::new();
    for path in &md_files {
        let content = fs::read_to_string(path)
            .with_context(|| format!("读取文件 {} 失败", path.display()))?;
        merged.push_str(&content);
        merged.push_str("\n\n");
    }
    Ok(merged)
}

/// 计算默认输出路径（输入是目录则用目录名，单文件则用 stem）
pub fn default_output(input: &Path, ext: &str) -> PathBuf {
    let kind = classify(input).unwrap_or(InputKind::File);
    let name = match kind {
        InputKind::Directory => input
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("output"),
        InputKind::File => input
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("output"),
    };
    PathBuf::from(format!("{}.{}", name, ext))
}
