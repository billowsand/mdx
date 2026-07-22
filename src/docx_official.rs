//! 公文 → docx pipeline。
//!
//! 第一版整体移植自 md2docx/md_to_docx_rust/src/main.rs，工具函数后续阶段会
//! 抽到 common/，此处保留行内副本以最大程度避免行为漂移。

use anyhow::{Context, Result};
use docx_rs::*;
use regex::Regex;
use std::fs::File;
use std::path::Path;

// ===== 常量 =====
const CIRCLE_NUMBERS_1: &[&str] = &[
    "⑴", "⑵", "⑶", "⑷", "⑸", "⑹", "⑺", "⑻", "⑼", "⑽",
    "⑾", "⑿", "⒀", "⒁", "⒂", "⒃", "⒄", "⒅", "⒆", "⒇",
];
const CIRCLE_NUMBERS_2: &[&str] = &[
    "①", "②", "③", "④", "⑤", "⑥", "⑦", "⑧", "⑨", "⑩",
    "⑪", "⑫", "⑬", "⑭", "⑮", "⑯", "⑰", "⑱", "⑲", "⑳",
];

// ===== 工具函数 =====

fn font_set(name: &str) -> RunFonts {
    RunFonts::new().ascii(name).hi_ansi(name).east_asia(name)
}

fn number_to_chinese(num: usize) -> String {
    let chinese_nums = ["", "一", "二", "三", "四", "五", "六", "七", "八", "九", "十"];
    if num <= 10 {
        chinese_nums[num].to_string()
    } else if num < 20 {
        format!("十{}", chinese_nums[num - 10])
    } else if num < 100 {
        let tens = num / 10;
        let ones = num % 10;
        if ones == 0 {
            format!("{}十", chinese_nums[tens])
        } else {
            format!("{}十{}", chinese_nums[tens], chinese_nums[ones])
        }
    } else {
        num.to_string()
    }
}

fn int_to_roman(num: usize) -> String {
    let val = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1];
    let syms = ["M", "CM", "D", "CD", "C", "XC", "L", "XL", "X", "IX", "V", "IV", "I"];
    let mut num = num;
    let mut result = String::new();
    for i in 0..val.len() {
        while num >= val[i] {
            result.push_str(syms[i]);
            num -= val[i];
        }
    }
    result
}

fn number_to_uppercase_letter(num: usize) -> String {
    let mut num = num;
    let mut result = String::new();
    while num > 0 {
        let remainder = (num - 1) % 26;
        result.insert(0, (65 + remainder) as u8 as char);
        num = (num - 1) / 26;
    }
    result
}

fn convert_quotes(text: &str) -> String {
    let mut text = text.to_string();
    text = text.replace('\u{201c}', "\"").replace('\u{201d}', "\"");
    text = text.replace('\u{2018}', "'").replace('\u{2019}', "'");

    let mut chars: Vec<char> = text.chars().collect();
    let mut in_double = false;
    for ch in &mut chars {
        if *ch == '"' {
            *ch = if !in_double { '\u{201c}' } else { '\u{201d}' };
            in_double = !in_double;
        }
    }
    let text: String = chars.into_iter().collect();

    let mut chars: Vec<char> = text.chars().collect();
    let mut in_single = false;
    for ch in &mut chars {
        if *ch == '\'' {
            *ch = if !in_single { '\u{2018}' } else { '\u{2019}' };
            in_single = !in_single;
        }
    }
    chars.into_iter().collect()
}

fn clean_heading_number(text: &str) -> String {
    let patterns = [
        // 1. 第X章/节/条/部分（支持中文数字或阿拉伯数字）
        r"^第[一二三四五六七八九十百零\d]+[章节条部分]\s*[、.．]?\s*",
        // 2. 全角括号中文数字 （一）（二）
        r"^[（(][一二三四五六七八九十百零]+[）)]\s*[、.．]?\s*",
        // 3. 中文数字+顿号/点号
        r"^[一二三四五六七八九十百零]+[、,.．]\s*",
        // 4. 全角括号阿拉伯数字 （1）（2）
        r"^[（(]\d+[）)]\s*[、.．]?\s*",
        // 5. 半角括号阿拉伯数字 (1)(2)
        r"^\(\d+\)\s*[、.．]?\s*",
        // 6. 多级数字编号 1.1 / 1.1.1 / 1.1.1.1（必须在单级数字之前）
        r"^\d+(?:\.\d+)+[.．]?\s*",
        // 7. 阿拉伯数字+点号/顿号+空格
        r"^\d+[.．、]\s+",
        // 8. 阿拉伯数字+空格（要求后面有非数字字符，用[^\d\s]近似）
        r"^\d+\s+[^\d\s]",
        // 9. 圆圈数字 ①②③
        r"^[①②③④⑤⑥⑦⑧⑨⑩⑪⑫⑬⑭⑮⑯⑰⑱⑲⑳㉑㉒㉓㉔㉕㉖㉗㉘㉙㉚㉛㉜㉝㉞㉟㊱㊲㊳㊴㊵㊶㊷㊸㊹㊺㊻㊼㊽㊾㊿]\s*[.．、]?\s*",
        // 10. 带圈数字 ⑴⑵⑶
        r"^[⑴⑵⑶⑷⑸⑹⑺⑻⑼⑽⑾⑿⒀⒁⒂⒃⒄⒅⒆⒇]\s*[.．、]?\s*",
        // 11. 罗马数字 ⅠⅡⅢ
        r"^[ⅠⅡⅢⅣⅤⅥⅦⅧⅨⅩ]\s*[.．、]?\s*",
        // 12. 字母编号 A. a)
        r"^[A-Za-z][)）.．]\s*",
    ];
    let mut cleaned = text.to_string();
    for pat in &patterns {
        if let Ok(re) = Regex::new(pat) {
            cleaned = re.replace(&cleaned, "").to_string();
        }
    }
    cleaned.trim().to_string()
}

// ===== 表格解析 =====

fn is_table_line(line: &str) -> bool {
    line.contains('|') && line.trim().starts_with('|') && line.trim().ends_with('|')
}

fn is_table_separator(line: &str) -> bool {
    let line = line.trim();
    if !line.starts_with('|') || !line.ends_with('|') {
        return false;
    }
    let content = &line[1..line.len() - 1];
    for part in content.split('|') {
        let part = part.trim();
        if part.is_empty() || !part.chars().all(|c| c == '-' || c == ':' || c == ' ') {
            return false;
        }
    }
    true
}

fn parse_table(lines: &[String], start_index: usize) -> (Option<Vec<Vec<String>>>, usize) {
    let mut table_lines = Vec::new();
    let mut i = start_index;

    while i < lines.len() {
        let line = lines[i].trim();
        if is_table_line(line) {
            table_lines.push(line.to_string());
        } else if line.is_empty() && !table_lines.is_empty() {
            i += 1;
            if i < lines.len() && is_table_line(lines[i].trim()) {
                continue;
            } else {
                break;
            }
        } else {
            break;
        }
        i += 1;
    }

    if table_lines.len() < 2 {
        return (None, start_index);
    }

    let mut table_data = Vec::new();
    let mut separator_found = false;

    for line in &table_lines {
        if is_table_separator(line) {
            separator_found = true;
            continue;
        }
        let mut line = line.trim();
        if line.starts_with('|') {
            line = &line[1..];
        }
        if line.ends_with('|') {
            line = &line[..line.len() - 1];
        }
        let cells: Vec<String> = line.split('|').map(|s| s.trim().to_string()).collect();
        table_data.push(cells);
    }

    if !separator_found || table_data.is_empty() {
        return (None, start_index);
    }

    (Some(table_data), i)
}

// ===== 文本格式化 =====

fn process_text_formatting(
    mut p: Paragraph,
    text: &str,
    font_name: &str,
    font_size: usize,
    force_bold: bool,
) -> Paragraph {
    let text = convert_quotes(text);
    let re = Regex::new(r"(\*\*.*?\*\*|\*.*?\*)").unwrap();
    let mut last_end = 0;

    for mat in re.find_iter(&text) {
        if mat.start() > last_end {
            let normal = &text[last_end..mat.start()];
            let mut run = Run::new()
                .add_text(normal)
                .fonts(font_set(font_name))
                .size(font_size);
            if force_bold {
                run = run.bold();
            }
            p = p.add_run(run);
        }

        let part = mat.as_str();
        if part.starts_with("**") && part.ends_with("**") && part.len() >= 4 {
            let inner = &part[2..part.len() - 2];
            p = p.add_run(
                Run::new()
                    .add_text(inner)
                    .bold()
                    .fonts(font_set(font_name))
                    .size(font_size),
            );
        } else if part.starts_with("*")
            && part.ends_with("*")
            && part.len() >= 2
            && !part.starts_with("**")
        {
            let inner = &part[1..part.len() - 1];
            let mut run = Run::new()
                .add_text(inner)
                .italic()
                .fonts(font_set(font_name))
                .size(font_size);
            if force_bold {
                run = run.bold();
            }
            p = p.add_run(run);
        }

        last_end = mat.end();
    }

    if last_end < text.len() {
        let normal = &text[last_end..];
        let mut run = Run::new()
            .add_text(normal)
            .fonts(font_set(font_name))
            .size(font_size);
        if force_bold {
            run = run.bold();
        }
        p = p.add_run(run);
    }

    p
}

// ===== 转换器 =====

struct Converter {
    docx: Docx,
    h2: usize,
    h3: usize,
    h4: usize,
    h5: usize,
    l1: usize,
    l2: usize,
    l3: usize,
    l4: usize,
    l5: usize,
    l6: usize,
    in_list: bool,
    list_level: usize,
    re_number_list: Regex,
    re_number_prefix: Regex,
}

impl Converter {
    fn new() -> Self {
        let mut conv = Converter {
            docx: Docx::new(),
            h2: 0,
            h3: 0,
            h4: 0,
            h5: 0,
            l1: 0,
            l2: 0,
            l3: 0,
            l4: 0,
            l5: 0,
            l6: 0,
            in_list: false,
            list_level: 0,
            re_number_list: Regex::new(r"^\d+\.").unwrap(),
            re_number_prefix: Regex::new(r"^\d+\.\s*").unwrap(),
        };
        conv.setup_page();
        conv
    }

    fn setup_page(&mut self) {
        let song_fonts = font_set("宋体");
        let footer = Footer::new().add_paragraph(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(LineSpacing::new().before(0).after(0))
                .add_run(
                    Run::new()
                        .add_text("\u{2014} ")
                        .fonts(song_fonts.clone())
                        .size(28),
                )
                .add_run(
                    Run::new()
                        .add_field_char(FieldCharType::Begin, false)
                        .fonts(song_fonts.clone())
                        .size(28),
                )
                .add_run(
                    Run::new()
                        .add_instr_text(InstrText::PAGE(InstrPAGE {}))
                        .fonts(song_fonts.clone())
                        .size(28),
                )
                .add_run(
                    Run::new()
                        .add_field_char(FieldCharType::Separate, false)
                        .fonts(song_fonts.clone())
                        .size(28),
                )
                .add_run(
                    Run::new()
                        .add_text("1")
                        .fonts(song_fonts.clone())
                        .size(28),
                )
                .add_run(
                    Run::new()
                        .add_field_char(FieldCharType::End, false)
                        .fonts(song_fonts.clone())
                        .size(28),
                )
                .add_run(
                    Run::new()
                        .add_text(" \u{2014}")
                        .fonts(song_fonts)
                        .size(28),
                ),
        );

        let docx = std::mem::replace(&mut self.docx, Docx::new());
        self.docx = docx
            .page_margin(
                PageMargin::new()
                    .top(2100) // 3.7cm
                    .bottom(1985) // 3.5cm
                    .left(1588) // 2.8cm
                    .right(1474) // 2.6cm
                    .footer(1588), // footer_distance = 2.8cm
            )
            .footer(footer);
    }

    fn add_table(&mut self, table_data: Vec<Vec<String>>) {
        if table_data.is_empty() {
            return;
        }
        let max_cols = table_data.iter().map(|r| r.len()).max().unwrap_or(0);
        let rows = table_data.len();
        if rows == 0 || max_cols == 0 {
            return;
        }

        let mut table_rows = Vec::new();
        for (row_idx, row_data) in table_data.iter().enumerate() {
            let mut cells = Vec::new();
            for col_idx in 0..max_cols {
                let cell_data = row_data.get(col_idx).map(|s| s.as_str()).unwrap_or("");
                let align = if row_idx == 0 {
                    AlignmentType::Center
                } else {
                    AlignmentType::Left
                };
                // 首行黑体、其余行仿宋_GB2312；字号统一四号(28hp)。
                // 不再整行强制加粗——交由 process_text_formatting 解析 cell 内的
                // **加粗**/*斜体*，确保 markdown 行内格式在表格里同样生效。
                let font = if row_idx == 0 { "黑体" } else { "仿宋_GB2312" };
                let p = Paragraph::new().align(align);
                let p = process_text_formatting(p, cell_data, font, 28, false);
                cells.push(TableCell::new().add_paragraph(p));
            }
            table_rows.push(TableRow::new(cells));
        }

        let table = Table::new(table_rows).set_borders(
            TableBorders::new()
                .set(TableBorder::new(TableBorderPosition::Top).size(4))
                .set(TableBorder::new(TableBorderPosition::Left).size(4))
                .set(TableBorder::new(TableBorderPosition::Bottom).size(4))
                .set(TableBorder::new(TableBorderPosition::Right).size(4))
                .set(TableBorder::new(TableBorderPosition::InsideH).size(4))
                .set(TableBorder::new(TableBorderPosition::InsideV).size(4)),
        );

        let docx = std::mem::replace(&mut self.docx, Docx::new());
        self.docx = docx.add_table(table);
    }

    fn normal_base(&self) -> Paragraph {
        Paragraph::new()
            .align(AlignmentType::Both)
            .line_spacing(
                LineSpacing::new()
                    .line_rule(LineSpacingType::Exact)
                    .line(580)
                    .before(0)
                    .after(0),
            )
            .indent(None, Some(SpecialIndentType::FirstLine(640)), None, None)
    }

    fn add_paragraph(&mut self, p: Paragraph) {
        let docx = std::mem::replace(&mut self.docx, Docx::new());
        self.docx = docx.add_paragraph(p);
    }

    fn make_normal_paragraph(&self, text: &str) -> Paragraph {
        process_text_formatting(self.normal_base(), text, "仿宋_GB2312", 32, false)
    }

    fn make_h1_paragraph(&self, text: &str) -> Paragraph {
        let p = Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(
                LineSpacing::new()
                    .line_rule(LineSpacingType::Exact)
                    .line(580)
                    .before(0)
                    .after(0),
            )
            .indent(None, Some(SpecialIndentType::FirstLine(640)), None, None);
        process_text_formatting(p, text, "方正小标宋简体", 44, false)
    }

    fn make_h2_paragraph(&self, text: &str) -> Paragraph {
        let p = Paragraph::new()
            .line_spacing(
                LineSpacing::new()
                    .line_rule(LineSpacingType::Exact)
                    .line(580)
                    .before(0)
                    .after(0),
            )
            .indent(None, Some(SpecialIndentType::FirstLine(640)), None, None);
        process_text_formatting(p, text, "黑体", 32, false)
    }

    fn make_h3_paragraph(&self, text: &str) -> Paragraph {
        let p = Paragraph::new()
            .line_spacing(
                LineSpacing::new()
                    .line_rule(LineSpacingType::Exact)
                    .line(580)
                    .before(0)
                    .after(0),
            )
            .indent(None, Some(SpecialIndentType::FirstLine(640)), None, None);
        process_text_formatting(p, text, "楷体_GB2312", 32, false)
    }

    fn make_h4_paragraph(&self, text: &str) -> Paragraph {
        let p = Paragraph::new()
            .line_spacing(
                LineSpacing::new()
                    .line_rule(LineSpacingType::Exact)
                    .line(580)
                    .before(0)
                    .after(0),
            )
            .indent(None, Some(SpecialIndentType::FirstLine(640)), None, None);
        process_text_formatting(p, text, "仿宋_GB2312", 32, false)
    }

    fn make_h5_paragraph(&self, text: &str) -> Paragraph {
        let p = Paragraph::new()
            .line_spacing(
                LineSpacing::new()
                    .line_rule(LineSpacingType::Exact)
                    .line(580)
                    .before(0)
                    .after(0),
            )
            .indent(None, Some(SpecialIndentType::FirstLine(640)), None, None);
        process_text_formatting(p, text, "仿宋_GB2312", 32, true)
    }

    fn make_list_paragraph(&self, prefix: &str, content: &str) -> Paragraph {
        let p = self.normal_base();
        let p = p.add_run(
            Run::new()
                .add_text(prefix)
                .fonts(font_set("仿宋_GB2312"))
                .size(32),
        );
        process_text_formatting(p, content, "仿宋_GB2312", 32, false)
    }

    fn reset_list_counters(&mut self) {
        self.in_list = false;
        self.list_level = 0;
        self.l1 = 0;
        self.l2 = 0;
        self.l3 = 0;
        self.l4 = 0;
        self.l5 = 0;
        self.l6 = 0;
    }

    fn get_list_prefix(&mut self, list_level: usize) -> String {
        match list_level {
            1 => {
                if !self.in_list || self.list_level > 1 {
                    self.l2 = 0;
                    self.l3 = 0;
                    self.l4 = 0;
                    self.l5 = 0;
                    self.l6 = 0;
                }
                if !self.in_list {
                    self.l1 = 0;
                }
                self.l1 += 1;
                if self.l1 <= CIRCLE_NUMBERS_2.len() {
                    CIRCLE_NUMBERS_2[self.l1 - 1].to_string()
                } else {
                    format!("({})", self.l1)
                }
            }
            2 => {
                if !self.in_list || self.list_level != 2 {
                    if self.list_level > 2 {
                        self.l3 = 0;
                        self.l4 = 0;
                        self.l5 = 0;
                        self.l6 = 0;
                    }
                    if !self.in_list || self.list_level < 2 {
                        self.l2 = 0;
                        self.l3 = 0;
                        self.l4 = 0;
                        self.l5 = 0;
                        self.l6 = 0;
                    }
                }
                self.l2 += 1;
                if self.l2 <= CIRCLE_NUMBERS_1.len() {
                    CIRCLE_NUMBERS_1[self.l2 - 1].to_string()
                } else {
                    format!("({})", self.l2)
                }
            }
            3 => {
                if !self.in_list || self.list_level < 3 {
                    self.l3 = 0;
                    self.l4 = 0;
                    self.l5 = 0;
                    self.l6 = 0;
                }
                self.l3 += 1;
                let ch = (b'a' + ((self.l3 - 1) % 26) as u8) as char;
                format!("{}.", ch)
            }
            4 => {
                if !self.in_list || self.list_level < 4 {
                    self.l4 = 0;
                    self.l5 = 0;
                    self.l6 = 0;
                }
                self.l4 += 1;
                format!("{}.", int_to_roman(self.l4))
            }
            5 => {
                if !self.in_list || self.list_level < 5 {
                    self.l5 = 0;
                    self.l6 = 0;
                }
                self.l5 += 1;
                format!("({})", number_to_uppercase_letter(self.l5))
            }
            6 => {
                if !self.in_list || self.list_level < 6 {
                    self.l6 = 0;
                }
                self.l6 += 1;
                format!("{})", self.l6)
            }
            _ => String::new(),
        }
    }

    fn parse_markdown(&mut self, content: &str) {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // 处理标题
            if line.starts_with('#') {
                self.reset_list_counters();
                let mut level = 0;
                let mut rest = line;
                while rest.starts_with('#') {
                    level += 1;
                    rest = &rest[1..];
                }
                let rest = rest.trim();
                let rest = clean_heading_number(rest);

                match level {
                    1 => {
                        self.h2 = 0;
                        self.h3 = 0;
                        self.h4 = 0;
                        self.h5 = 0;
                        let p = self.make_h1_paragraph(&rest);
                        self.add_paragraph(p);
                        self.add_paragraph(Paragraph::new());
                    }
                    2 => {
                        self.h2 += 1;
                        self.h3 = 0;
                        self.h4 = 0;
                        self.h5 = 0;
                        let num = number_to_chinese(self.h2);
                        let title = format!("{}、{}", num, rest);
                        let p = self.make_h2_paragraph(&title);
                        self.add_paragraph(p);
                    }
                    3 => {
                        self.h3 += 1;
                        self.h4 = 0;
                        self.h5 = 0;
                        let num = number_to_chinese(self.h3);
                        let title = format!("（{}）{}", num, rest);
                        let p = self.make_h3_paragraph(&title);
                        self.add_paragraph(p);
                    }
                    4 => {
                        self.h4 += 1;
                        self.h5 = 0;
                        let title = format!("{}.{}", self.h4, rest);
                        let p = self.make_h4_paragraph(&title);
                        self.add_paragraph(p);
                    }
                    5 => {
                        self.h5 += 1;
                        let title = format!("({}){}", self.h5, rest);
                        let p = self.make_h5_paragraph(&title);
                        self.add_paragraph(p);
                    }
                    _ => {}
                }
            }
            // 处理表格
            else if is_table_line(line) {
                self.reset_list_counters();
                let (table_data, new_i) = parse_table(&lines, i);
                if let Some(data) = table_data {
                    self.add_table(data);
                    i = new_i - 1;
                } else {
                    let p = self.make_normal_paragraph(line);
                    self.add_paragraph(p);
                }
            }
            // 处理列表
            else if line.starts_with("- ")
                || line.starts_with("* ")
                || self.re_number_list.is_match(line)
            {
                let indent_level = lines[i].len() - lines[i].trim_start().len();
                let list_level = match indent_level {
                    0 => 1,
                    1..=4 => 2,
                    5..=8 => 3,
                    9..=12 => 4,
                    13..=16 => 5,
                    _ => 6,
                };

                let content = if line.starts_with("- ") || line.starts_with("* ") {
                    line[2..].trim().to_string()
                } else {
                    self.re_number_prefix.replace(line, "").to_string()
                };

                let prefix = self.get_list_prefix(list_level);
                let p = self.make_list_paragraph(&prefix, &content);
                self.add_paragraph(p);
                self.in_list = true;
                self.list_level = list_level;
            }
            // 处理空行
            else if line.is_empty() {
                // 忽略空行
            }
            // 处理普通文本
            else {
                self.reset_list_counters();
                let p = self.make_normal_paragraph(line);
                self.add_paragraph(p);
            }

            i += 1;
        }
    }

    fn write(&self, output: &Path) -> Result<()> {
        let file = File::create(output)
            .with_context(|| format!("创建输出文件 {} 失败", output.display()))?;
        self.docx
            .clone()
            .build()
            .pack(file)
            .with_context(|| format!("写入 docx {} 失败", output.display()))?;
        Ok(())
    }
}

// ===== 公共入口 =====

pub fn run(input: &Path, output: Option<&Path>) -> Result<()> {
    let content = crate::input::collect(input)?;
    let output_path = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::input::default_output(input, "docx"));

    println!("正在转换: {}", input.display());

    let mut conv = Converter::new();
    conv.parse_markdown(&content);
    conv.write(&output_path)?;

    println!("[完成] 转换完成: {}", output_path.display());
    Ok(())
}
