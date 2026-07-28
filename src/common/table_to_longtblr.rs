//! 表格 → longtblr 转换。
//!
//! 智能表格处理：自动分析内容决定对齐方式和列宽分配。

use crate::common::ast::{Block, Inline};

/// 配置参数
const SHORT_TEXT_THRESHOLD: f64 = 8.0;
const LONG_TEXT_THRESHOLD: f64 = 20.0;
const NUMERIC_RATIO_THRESHOLD: f64 = 0.8;
const MIN_WIDTH_RATIO: f64 = 0.8;
const MAX_WIDTH_RATIO: f64 = 4.0;
const CJK_WIDTH_FACTOR: f64 = 1.8;
/// 窄数字列：表头以外全是不超过这么多位的纯数字（如序号列 1/2/…/99）
const NARROW_NUMERIC_MAX_DIGITS: usize = 2;
/// 窄数字列的固定列宽，不参与 X 列的按比例伸缩
const NARROW_NUMERIC_WIDTH: &str = "2em";

/// 计算字符串的显示宽度（考虑中文字符）
fn calc_display_width(s: &str) -> f64 {
    let mut width = 0.0;
    for c in s.chars() {
        if c.is_ascii() {
            width += 1.0;
        } else {
            width += CJK_WIDTH_FACTOR;
        }
    }
    width
}

/// 检测是否为数字内容（包括带单位的数字）
fn is_numeric_content(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // 清理空白和 LaTeX 命令
    let cleaned = s.replace(|c: char| c.is_whitespace(), "").replace("\\", "");

    let patterns: Vec<&str> = vec![
        r"^-?\d+\.?\d*$",            // 纯数字：123, 12.34, -5
        r"^-?\d+\.?\d*%$",           // 百分比：12.5%
        r"^-?\d+\.?\d*[万亿千百]+$", // 带中文单位：100万
        r"^\d+[/-]\d+[/-]?\d*$",     // 日期：2024/01/01, 2024-01
        r"^\d+:\d+:?\d*$",           // 时间：12:30, 12:30:00
        r"^[\d,]+.?\d*$",            // 带千分位：1,234,567
    ];

    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(&cleaned) {
                return true;
            }
        }
    }

    false
}

/// 检测是否包含句子标点（表示描述性文本）
fn has_sentence_punctuation(s: &str) -> bool {
    let short_text_punctuations = ['，', '、', ','];
    let long_text_punctuations = ['。', '；', '：', '.', ';'];

    let width = calc_display_width(s);
    let mut has_long_punct = false;
    let mut has_short_punct = false;

    for c in s.chars() {
        if long_text_punctuations.contains(&c) {
            has_long_punct = true;
        }
        if short_text_punctuations.contains(&c) {
            has_short_punct = true;
        }
    }

    if has_long_punct {
        return true;
    }
    // 短文本中的逗号/顿号不算描述性标点
    if has_short_punct && width > SHORT_TEXT_THRESHOLD {
        return true;
    }

    false
}

/// 是否为"不超过 NARROW_NUMERIC_MAX_DIGITS 位的纯数字"，如 `1`、`42`
fn is_short_digits(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty()
        && t.chars().count() <= NARROW_NUMERIC_MAX_DIGITS
        && t.chars().all(|c| c.is_ascii_digit())
}

#[derive(Debug, Clone)]
struct ColumnStats {
    total_width: f64,
    max_width: f64,
    count: usize,
    numeric_count: usize,
    has_long_text: bool,
    has_punctuation: bool,
    avg_width: f64,
    is_numeric: bool,
    /// 表头以外全是 2 位以内纯数字（序号列一类）：走固定列宽
    is_narrow_numeric: bool,
}

impl ColumnStats {
    fn new() -> Self {
        Self {
            total_width: 0.0,
            max_width: 0.0,
            count: 0,
            numeric_count: 0,
            has_long_text: false,
            has_punctuation: false,
            avg_width: 0.0,
            is_numeric: false,
            is_narrow_numeric: false,
        }
    }
}

/// 分析列内容特征
fn analyze_column(cells: &[String]) -> ColumnStats {
    let mut stats = ColumnStats::new();
    // 传入的 cells 不含表头，因此这里天然只看表体
    let mut all_short_digits = true;

    for cell_text in cells {
        if cell_text.is_empty() {
            continue;
        }

        if !is_short_digits(cell_text) {
            all_short_digits = false;
        }

        let width = calc_display_width(cell_text);

        stats.total_width += width;
        stats.count += 1;

        if width > stats.max_width {
            stats.max_width = width;
        }

        if is_numeric_content(cell_text) {
            stats.numeric_count += 1;
        }

        if width > LONG_TEXT_THRESHOLD {
            stats.has_long_text = true;
        }

        if has_sentence_punctuation(cell_text) {
            stats.has_punctuation = true;
        }
    }

    // 计算平均宽度
    stats.avg_width = if stats.count > 0 {
        stats.total_width / stats.count as f64
    } else {
        0.0
    };

    // 判断是否为数字列
    stats.is_numeric = stats.count > 0
        && (stats.numeric_count as f64 / stats.count as f64) >= NUMERIC_RATIO_THRESHOLD;

    // 窄数字列：表体全是 2 位以内纯数字（空单元格不计）
    stats.is_narrow_numeric = stats.count > 0 && all_short_digits;

    stats
}

/// 根据列统计决定对齐方式
fn determine_alignment(stats: &ColumnStats) -> char {
    // 数字列：居中
    if stats.is_numeric {
        return 'c';
    }

    // 有句子标点或长文本：靠左
    if stats.has_punctuation || stats.has_long_text {
        return 'l';
    }

    // 短文本或中等文本：居中
    if stats.avg_width <= SHORT_TEXT_THRESHOLD || stats.avg_width <= LONG_TEXT_THRESHOLD {
        return 'c';
    }

    // 默认靠左
    'l'
}

/// 计算列宽比例
fn calculate_width_ratios(columns_stats: &[ColumnStats]) -> Vec<f64> {
    let num_cols = columns_stats.len();
    if num_cols == 0 {
        return vec![];
    }

    // 计算每列的权重
    let mut weights: Vec<f64> = Vec::new();
    let mut total_weight = 0.0;

    for stats in columns_stats {
        // 权重 = max(最大宽度, 平均宽度 * 1.2)
        let weight = stats.max_width.max(stats.avg_width * 1.2).max(2.0);
        weights.push(weight);
        total_weight += weight;
    }

    // 归一化并应用约束
    let mut ratios: Vec<f64> = Vec::new();

    for weight in &weights {
        let mut ratio = (weight / total_weight) * num_cols as f64;

        // 应用最小/最大约束
        ratio = ratio.clamp(MIN_WIDTH_RATIO, MAX_WIDTH_RATIO);

        // 四舍五入到一位小数
        ratio = (ratio * 10.0 + 0.5).floor() / 10.0;

        ratios.push(ratio);
    }

    ratios
}

/// 生成智能列规格
fn generate_smart_colspec(columns_stats: &[ColumnStats]) -> String {
    let ratios = calculate_width_ratios(columns_stats);
    let mut specs: Vec<String> = Vec::new();

    for (i, stats) in columns_stats.iter().enumerate() {
        let align = determine_alignment(stats);
        let ratio = ratios[i];

        // 窄数字列（序号列一类）：固定 2em，不参与 X 列的按比例伸缩，
        // 余下的列仍按原比例算法分配剩余宽度
        if stats.is_narrow_numeric {
            specs.push(format!("Q[{},wd={}]", align, NARROW_NUMERIC_WIDTH));
            continue;
        }

        // 生成 X[ratio, align] 格式
        let spec = if ratio == 1.0 {
            format!("X[{}]", align)
        } else {
            format!("X[{:.1},{}]", ratio, align)
        };
        specs.push(spec);
    }

    specs.join(" ")
}

/// 转义 LaTeX 特殊字符（不处理行内格式标记）。
fn escape_latex(s: &str) -> String {
    let mut result = s.to_string();
    let latex_escapes = [
        ('\\', "\\textbackslash{}"),
        ('&', "\\&"),
        ('%', "\\%"),
        ('$', "\\$"),
        ('#', "\\#"),
        ('_', "\\_"),
        ('{', "\\{"),
        ('}', "\\}"),
        ('~', "\\textasciitilde{}"),
        ('^', "\\textasciicircum{}"),
    ];
    for (from, to) in latex_escapes {
        result = result.replace(from, to);
    }
    result
}

/// 将单元格内容转换为 LaTeX：先解析 **加粗**/*斜体*/`代码`/链接 行内格式，
/// 再对纯文本段落做 LaTeX 转义。
fn cell_to_latex(cell: &str) -> String {
    let mut result = String::new();
    push_cell_inlines(&mut result, &crate::common::inline::parse(cell));
    result
}

/// 把 inline::parse 之后的节点序列推入 result（处理粗体 / 斜体嵌套）。
/// 末尾返回借用以方便链式调用时复用，但本文件内主要靠 result 累加。
fn push_cell_inlines(result: &mut String, inlines: &[Inline]) {
    for ip in inlines {
        match ip {
            Inline::Text(t) => result.push_str(&escape_latex(t)),
            Inline::Bold(children) => {
                result.push_str("\\textbf{");
                push_cell_inlines(result, children);
                result.push('}');
            }
            Inline::Italic(children) => {
                result.push_str("\\textit{");
                push_cell_inlines(result, children);
                result.push('}');
            }
            Inline::Code(t) => {
                result.push_str("\\texttt{");
                result.push_str(&escape_latex(t));
                result.push('}');
            }
            Inline::Link { text, .. } => result.push_str(&escape_latex(text)),
            // longtblr 单元格内不插图，降级为替代文本
            Inline::Image { alt, .. } => result.push_str(&escape_latex(alt)),
            // 单元格内交叉引用：\ref 在 longtblr 中可用，直接输出
            Inline::CrossRef(id) => {
                result.push_str("\\ref{");
                result.push_str(id);
                result.push('}');
            }
            Inline::Citation(keys) => {
                result.push_str("\\cite{");
                result.push_str(&keys.join(","));
                result.push('}');
            }
            // longtblr 单元格内 \footnote 不生效，降级为全角括号内联注释
            Inline::Footnote(t) => {
                result.push('（');
                result.push_str(&escape_latex(t));
                result.push('）');
            }
        }
    }
}

/// 处理表格行
fn process_row(cells: &[String], is_header: bool) -> String {
    let rendered: Vec<String> = cells
        .iter()
        .map(|cell| {
            let content = cell_to_latex(cell);
            if is_header {
                format!("\\heiti {}", content)
            } else {
                content
            }
        })
        .collect();

    rendered.join(" & ") + " \\\\"
}

/// 生成 longtblr 环境的完整代码
/// 输入：表格行数据 Vec<Vec<String>>，第一行是表头
/// `label` 为交叉引用锚点（tabularray 外层 `label=` 选项）；引用表格需同时提供 caption。
pub fn emit_longtblr(rows: &[Vec<String>], caption: Option<&str>, label: Option<&str>) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let num_cols = rows[0].len();
    if num_cols == 0 {
        return String::new();
    }

    // 收集各列内容（不包括表头）
    let mut columns: Vec<Vec<String>> = vec![Vec::new(); num_cols];
    for row in rows.iter().skip(1) {
        for (col_idx, cell) in row.iter().enumerate() {
            if col_idx < num_cols {
                columns[col_idx].push(cell.clone());
            }
        }
    }

    // 分析各列特征
    let columns_stats: Vec<ColumnStats> = columns.iter().map(|col| analyze_column(col)).collect();

    // 生成智能列规格
    let colspec = generate_smart_colspec(&columns_stats);

    // 生成调试信息
    let mut debug_info = String::from("% 智能表格分析结果:\n");
    for (i, stats) in columns_stats.iter().enumerate() {
        let align = determine_alignment(stats);
        let align_str = if align == 'c' { "居中" } else { "靠左" };
        debug_info.push_str(&format!(
            "%% 列{}: 平均宽度={:.1}, 最大宽度={:.1}, 数字列={}, 有标点={} → 对齐={}{}\n",
            i + 1,
            stats.avg_width,
            stats.max_width,
            if stats.is_numeric { "是" } else { "否" },
            if stats.has_punctuation { "是" } else { "否" },
            align_str,
            if stats.is_narrow_numeric {
                format!(", 窄数字列 → 固定列宽={}", NARROW_NUMERIC_WIDTH)
            } else {
                String::new()
            }
        ));
    }

    // 生成 longtblr 开始标记（外层选项按需含 caption 与 label）
    let mut outer_opts = String::new();
    if let Some(cap) = caption {
        outer_opts.push_str(&format!("caption={{{}}}", cap));
    }
    if let Some(lab) = label {
        if !outer_opts.is_empty() {
            outer_opts.push_str(", ");
        }
        outer_opts.push_str(&format!("label={{{}}}", lab));
    }
    let longtblr_begin = if outer_opts.is_empty() {
        format!(
            "{}\n\\begin{{longtblr}}{{\n  colspec = {{{}}},\n  rowhead = 1,\n  hlines,\n  vlines,\n  row{{1}} = {{c, font=\\heiti}},\n}}",
            debug_info, colspec
        )
    } else {
        format!(
            "{}\n\\begin{{longtblr}}[{}]{{\n  colspec = {{{}}},\n  rowhead = 1,\n  hlines,\n  vlines,\n  row{{1}} = {{c, font=\\heiti}},\n}}",
            debug_info, outer_opts, colspec
        )
    };

    let mut content_lines = vec![longtblr_begin];

    // 处理表头
    content_lines.push(process_row(&rows[0], true));

    // 处理表体
    for row in rows.iter().skip(1) {
        content_lines.push(process_row(row, false));
    }

    // 添加结束标记
    content_lines.push("\\end{longtblr}".to_string());

    content_lines.join("\n")
}

/// 将 Block::Table 转换为 longtblr LaTeX 代码
pub fn table_block_to_longtblr(block: &Block, caption: Option<&str>) -> String {
    match block {
        Block::Table {
            rows,
            caption: block_caption,
        } => emit_longtblr(rows, caption.or(block_caption.as_deref()), None),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_width() {
        assert_eq!(calc_display_width("hello"), 5.0);
        assert_eq!(calc_display_width("你好"), 3.6); // 2 * 1.8
        assert_eq!(calc_display_width("hello你好"), 8.6); // 5 + 3.6
    }

    #[test]
    fn test_numeric_detection() {
        assert!(is_numeric_content("123"));
        assert!(is_numeric_content("12.34"));
        assert!(is_numeric_content("-5"));
        assert!(is_numeric_content("12.5%"));
        assert!(is_numeric_content("100万"));
        assert!(is_numeric_content("2024/01/01"));
        assert!(!is_numeric_content("你好"));
        assert!(!is_numeric_content("Hello world"));
    }

    #[test]
    fn test_column_analysis() {
        let cells = vec!["123".to_string(), "456".to_string(), "789".to_string()];
        let stats = analyze_column(&cells);
        assert!(stats.is_numeric);
    }

    #[test]
    fn narrow_numeric_column_uses_fixed_width() {
        // 序号列：表头以外全是 2 位以内纯数字 → 固定 2em；其余列仍走 X 比例
        let rows = vec![
            vec!["序号".to_string(), "说明".to_string()],
            vec!["1".to_string(), "第一条说明文字，较长。".to_string()],
            vec!["12".to_string(), "第二条说明文字，同样较长。".to_string()],
        ];
        let result = emit_longtblr(&rows, None, None);
        assert!(result.contains("Q[c,wd=2em]"), "{result}");
        assert!(result.contains(" X["), "第二列应仍为 X 列: {result}");
    }

    #[test]
    fn three_digit_column_keeps_ratio_width() {
        // 出现 3 位数字即不算窄数字列，仍按原比例算法分配
        let rows = vec![
            vec!["数量".to_string(), "说明".to_string()],
            vec!["1".to_string(), "文字".to_string()],
            vec!["123".to_string(), "文字".to_string()],
        ];
        let result = emit_longtblr(&rows, None, None);
        assert!(!result.contains("wd=2em"), "{result}");
    }

    #[test]
    fn non_digit_content_is_not_narrow_numeric() {
        // 带单位/小数点的数字不算纯数字，不走固定列宽
        let rows = vec![
            vec!["比例".to_string(), "说明".to_string()],
            vec!["5%".to_string(), "文字".to_string()],
            vec!["1.5".to_string(), "文字".to_string()],
        ];
        let result = emit_longtblr(&rows, None, None);
        assert!(!result.contains("wd=2em"), "{result}");
    }

    #[test]
    fn narrow_numeric_ignores_header_and_empty_cells() {
        // 表头再宽也不影响判定；空单元格跳过
        let rows = vec![
            vec!["很长的表头文字说明".to_string(), "说明".to_string()],
            vec!["1".to_string(), "文字".to_string()],
            vec!["".to_string(), "文字".to_string()],
            vec!["99".to_string(), "文字".to_string()],
        ];
        let result = emit_longtblr(&rows, None, None);
        assert!(result.contains("Q[c,wd=2em]"), "{result}");
    }

    #[test]
    fn test_longtblr_output() {
        let rows = vec![
            vec!["列A".to_string(), "列B".to_string(), "列C".to_string()],
            vec!["1".to_string(), "2".to_string(), "文本内容".to_string()],
            vec!["3".to_string(), "4".to_string(), "更多文本".to_string()],
        ];
        let result = emit_longtblr(&rows, Some("测试表格"), None);
        assert!(result.contains("\\begin{longtblr}"));
        assert!(result.contains("\\end{longtblr}"));
        assert!(result.contains("caption={测试表格}"));
        assert!(!result.contains("label="));
    }

    #[test]
    fn test_longtblr_with_label() {
        let rows = vec![vec!["列A".to_string()], vec!["1".to_string()]];
        let result = emit_longtblr(&rows, Some("测试表格"), Some("tbl:products"));
        assert!(
            result.contains("caption={测试表格}, label={tbl:products}"),
            "{result}"
        );
    }

    #[test]
    fn table_cell_renders_citation() {
        let tex = emit_longtblr(&[vec!["文献".into()], vec!["[@a; @b]".into()]], None, None);
        assert!(tex.contains("\\cite{a,b}"), "{tex}");
    }
}
