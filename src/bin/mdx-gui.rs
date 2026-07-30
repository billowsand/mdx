#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use mdx::{
    ConvertOutcome, ConvertRequest, DocumentStyle, OutputFormat, ProgressEvent, ProgressLevel,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 620.0])
            .with_min_inner_size([620.0, 500.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native("mdx", options, Box::new(|cc| Ok(Box::new(MdxApp::new(cc)))))
}

struct MdxApp {
    input: String,
    output: String,
    template: String,
    format: OutputFormat,
    style: DocumentStyle,
    compile_pdf: bool,
    running: bool,
    receiver: Option<Receiver<WorkerMessage>>,
    logs: Vec<LogEntry>,
    outcome: Option<ConvertOutcome>,
}

impl MdxApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(egui::Visuals::light());

        Self {
            input: String::new(),
            output: String::new(),
            template: String::new(),
            format: OutputFormat::Docx,
            style: DocumentStyle::Official,
            compile_pdf: true,
            running: false,
            receiver: None,
            logs: vec![LogEntry::info("请选择 Markdown 文件或目录。")],
            outcome: None,
        }
    }

    fn poll_worker(&mut self) {
        let mut finished = false;
        if let Some(receiver) = &self.receiver {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    WorkerMessage::Progress(event) => self.logs.push(LogEntry::from(event)),
                    WorkerMessage::Finished(result) => {
                        self.running = false;
                        finished = true;
                        match result {
                            Ok(outcome) => {
                                self.logs.push(LogEntry::success(format!(
                                    "已生成：{}",
                                    outcome.output.display()
                                )));
                                if let Some(pdf) = &outcome.pdf {
                                    self.logs.push(LogEntry::success(format!(
                                        "已生成 PDF：{}",
                                        pdf.display()
                                    )));
                                }
                                self.outcome = Some(outcome);
                            }
                            Err(error) => self
                                .logs
                                .push(LogEntry::error(format!("转换失败：{error}"))),
                        }
                    }
                }
            }
        }
        if finished {
            self.receiver = None;
        }
    }

    fn choose_input_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("选择 Markdown 文件")
            .add_filter("Markdown", &["md", "markdown"])
            .pick_file()
        {
            self.set_input(path);
        }
    }

    fn choose_input_directory(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("选择包含 Markdown 的目录")
            .pick_folder()
        {
            self.set_input(path);
        }
    }

    fn set_input(&mut self, path: PathBuf) {
        self.output = suggested_output(&path, self.format)
            .to_string_lossy()
            .into_owned();
        self.input = path.to_string_lossy().into_owned();
        self.outcome = None;
    }

    fn choose_output(&mut self) {
        let extension = self.format.extension();
        let mut dialog = rfd::FileDialog::new()
            .set_title("选择输出文件")
            .add_filter(extension.to_ascii_uppercase(), &[extension]);

        if let Some(path) = non_empty_path(&self.output) {
            if let Some(parent) = path.parent() {
                dialog = dialog.set_directory(parent);
            }
            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                dialog = dialog.set_file_name(name);
            }
        }

        if let Some(mut path) = dialog.save_file() {
            ensure_extension(&mut path, extension);
            self.output = path.to_string_lossy().into_owned();
            self.outcome = None;
        }
    }

    fn choose_template(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("选择 LaTeX 模板")
            .add_filter("LaTeX", &["tex"])
            .pick_file()
        {
            self.template = path.to_string_lossy().into_owned();
        }
    }

    /// 切换格式后重算输出路径。tex 与 docx 的目录布局不同（tex 多一层收纳目录），
    /// 仅改扩展名会把 docx 留在 tex 的收纳目录里，因此有输入时整体重算。
    fn update_output_extension(&mut self) {
        if let Some(input) = non_empty_path(&self.input) {
            self.output = suggested_output(&input, self.format)
                .to_string_lossy()
                .into_owned();
        } else if let Some(mut path) = non_empty_path(&self.output) {
            path.set_extension(self.format.extension());
            self.output = path.to_string_lossy().into_owned();
        }
        self.outcome = None;
    }

    fn start_conversion(&mut self, ctx: &egui::Context) {
        let request = match self.build_request() {
            Ok(request) => request,
            Err(error) => {
                self.logs.push(LogEntry::error(error));
                return;
            }
        };

        self.logs.clear();
        self.logs.push(LogEntry::info("转换任务已启动。"));
        self.outcome = None;
        self.running = true;

        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        let repaint = ctx.clone();

        std::thread::spawn(move || {
            let progress_sender = sender.clone();
            let progress_repaint = repaint.clone();
            let result = mdx::convert_with_progress(request, move |event| {
                let _ = progress_sender.send(WorkerMessage::Progress(event));
                progress_repaint.request_repaint();
            })
            .map_err(|error| format!("{error:#}"));

            let _ = sender.send(WorkerMessage::Finished(result));
            repaint.request_repaint();
        });
    }

    fn build_request(&self) -> Result<ConvertRequest, String> {
        let input =
            non_empty_path(&self.input).ok_or_else(|| "请选择输入文件或目录。".to_owned())?;
        if !input.exists() {
            return Err(format!("输入路径不存在：{}", input.display()));
        }

        let output = non_empty_path(&self.output).ok_or_else(|| "请选择输出文件。".to_owned())?;
        let expected_extension = self.format.extension();
        let actual_extension = output.extension().and_then(|ext| ext.to_str());
        if !actual_extension
            .map(|ext| ext.eq_ignore_ascii_case(expected_extension))
            .unwrap_or(false)
        {
            return Err(format!("输出文件扩展名应为 .{expected_extension}"));
        }

        let template = if self.format == OutputFormat::Tex
            && self.style == DocumentStyle::Research
            && !self.template.trim().is_empty()
        {
            let path = PathBuf::from(self.template.trim());
            if !path.is_file() {
                return Err(format!("模板文件不存在：{}", path.display()));
            }
            Some(path)
        } else {
            None
        };

        Ok(ConvertRequest {
            input,
            output: Some(output),
            format: self.format,
            style: self.style,
            template,
            compile_pdf: self.format == OutputFormat::Tex && self.compile_pdf,
        })
    }

    fn accept_dropped_file(&mut self, ctx: &egui::Context) {
        let path = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .find_map(|file| file.path.clone())
        });
        if let Some(path) = path {
            let is_markdown = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
                .unwrap_or(false);
            if path.is_dir() || is_markdown {
                self.set_input(path);
                self.logs.push(LogEntry::info("已接受拖入的输入路径。"));
            } else {
                self.logs
                    .push(LogEntry::error("拖入项必须是 Markdown 文件或目录。"));
            }
        }
    }
}

impl eframe::App for MdxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_worker();
        if !self.running {
            self.accept_dropped_file(ctx);
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading("mdx 文档转换器");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(format!("v{}", env!("CARGO_PKG_VERSION")));
                });
            });
            ui.label("将 Markdown 转换为中文公文或研究报告（DOCX / TeX / PDF）");
            ui.add_space(8.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(!self.running, |ui| {
                egui::Grid::new("conversion_form")
                    .num_columns(2)
                    .spacing([14.0, 12.0])
                    .show(ui, |ui| {
                        ui.label("输入");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.input)
                                    .desired_width(ui.available_width() - 174.0)
                                    .hint_text("Markdown 文件或目录，也可拖入窗口"),
                            );
                            if ui.button("选择文件").clicked() {
                                self.choose_input_file();
                            }
                            if ui.button("选择目录").clicked() {
                                self.choose_input_directory();
                            }
                        });
                        ui.end_row();

                        ui.label("输出格式");
                        ui.horizontal(|ui| {
                            let old_format = self.format;
                            ui.selectable_value(&mut self.format, OutputFormat::Docx, "DOCX");
                            ui.selectable_value(&mut self.format, OutputFormat::Tex, "TeX / PDF");
                            if self.format != old_format {
                                self.update_output_extension();
                            }
                        });
                        ui.end_row();

                        ui.label("文档样式");
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut self.style,
                                DocumentStyle::Official,
                                "中文公文",
                            );
                            ui.selectable_value(
                                &mut self.style,
                                DocumentStyle::Research,
                                "研究报告",
                            );
                        });
                        ui.end_row();

                        ui.label("输出文件");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.output)
                                    .desired_width(ui.available_width() - 88.0)
                                    .hint_text("输出文件路径"),
                            );
                            if ui.button("浏览…").clicked() {
                                self.choose_output();
                            }
                        });
                        ui.end_row();

                        if self.format == OutputFormat::Tex && self.style == DocumentStyle::Research
                        {
                            ui.label("自定义模板");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.template)
                                        .desired_width(ui.available_width() - 88.0)
                                        .hint_text("可选；留空使用内置模板"),
                                );
                                if ui.button("浏览…").clicked() {
                                    self.choose_template();
                                }
                            });
                            ui.end_row();
                        }

                        if self.format == OutputFormat::Tex {
                            ui.label("PDF");
                            ui.checkbox(
                                &mut self.compile_pdf,
                                "检测到 XeLaTeX 或 Tectonic 时同时编译 PDF",
                            );
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(16.0);
            ui.horizontal(|ui| {
                let convert_button = egui::Button::new(egui::RichText::new("开始转换").size(17.0))
                    .min_size(egui::vec2(132.0, 38.0));
                if ui.add_enabled(!self.running, convert_button).clicked() {
                    self.start_conversion(ctx);
                }

                if self.running {
                    ui.spinner();
                    ui.label("正在转换，请稍候…");
                } else if let Some(outcome) = &self.outcome {
                    ui.colored_label(egui::Color32::from_rgb(28, 120, 74), "转换成功");
                    if ui.button("打开输出位置").clicked() {
                        if let Err(error) = open_output_location(&outcome.output) {
                            self.logs
                                .push(LogEntry::error(format!("无法打开输出位置：{error}")));
                        }
                    }
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            ui.label(egui::RichText::new("任务日志").strong());

            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for log in &self.logs {
                        ui.colored_label(log.color, &log.message);
                    }
                });
        });
    }
}

enum WorkerMessage {
    Progress(ProgressEvent),
    Finished(Result<ConvertOutcome, String>),
}

struct LogEntry {
    message: String,
    color: egui::Color32,
}

impl LogEntry {
    fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            color: egui::Color32::from_rgb(55, 65, 81),
        }
    }

    fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            color: egui::Color32::from_rgb(28, 120, 74),
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            color: egui::Color32::from_rgb(185, 28, 28),
        }
    }
}

impl From<ProgressEvent> for LogEntry {
    fn from(event: ProgressEvent) -> Self {
        let color = match event.level {
            ProgressLevel::Info => egui::Color32::from_rgb(55, 65, 81),
            ProgressLevel::Warning => egui::Color32::from_rgb(180, 100, 10),
        };
        Self {
            message: event.message,
            color,
        }
    }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "sfss".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../font/sfss.ttf"
        ))),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "sfss".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("sfss".to_owned());
    ctx.set_fonts(fonts);
}

fn non_empty_path(value: &str) -> Option<PathBuf> {
    let value = value.trim();
    (!value.is_empty()).then(|| PathBuf::from(value))
}

/// 默认输出落在输入旁边，命名与目录布局沿用 CLI 规则
/// （tex 收进单独目录，docx 为同级单文件）。
fn suggested_output(input: &Path, format: OutputFormat) -> PathBuf {
    input
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(mdx::default_output(input, format))
}

fn ensure_extension(path: &mut PathBuf, extension: &str) {
    let has_expected_extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case(extension))
        .unwrap_or(false);
    if !has_expected_extension {
        path.set_extension(extension);
    }
}

#[cfg(target_os = "windows")]
fn open_output_location(path: &Path) -> std::io::Result<()> {
    Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .map(|_| ())
}

#[cfg(target_os = "macos")]
fn open_output_location(path: &Path) -> std::io::Result<()> {
    Command::new("open").arg("-R").arg(path).spawn().map(|_| ())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_output_location(path: &Path) -> std::io::Result<()> {
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    Command::new("xdg-open").arg(directory).spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggested_output_stays_next_to_input_file() {
        let input = Path::new("reports/source.md");
        assert_eq!(
            suggested_output(input, OutputFormat::Docx),
            Path::new("reports/source.docx")
        );
        // tex 连带 data/、figures/、.cls 等，收进 reports/source/ 目录
        assert_eq!(
            suggested_output(input, OutputFormat::Tex),
            Path::new("reports/source/source.tex")
        );
    }

    #[test]
    fn ensure_extension_replaces_unexpected_extension() {
        let mut path = PathBuf::from("report.txt");
        ensure_extension(&mut path, "docx");
        assert_eq!(path, PathBuf::from("report.docx"));
    }
}
