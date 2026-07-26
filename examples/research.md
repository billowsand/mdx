---
密级: 公开
文件类型: 技术报告
文件编号: MDX-DEMO-001
文件版本号: V1.0
撰写单位: 示例单位
撰写时间: 2026-07
文件名称: mdx 研究报告转换示例
---

# mdx 研究报告转换示例

<!-- [摘要] -->

本示例用于快速体验 `research` 样式生成的封面、目录、章节编号、表格与附录。

<!-- [正文] -->

## 项目概述 {#chap:overview}

mdx 使用 Rust 原生解析器把 Markdown 转换为中文研究报告。转换流程简单、可复现，并兼顾 DOCX 与 TeX 两类交付格式。

### 核心能力

- 生成带封面和目录的研究报告
- 自动管理章节、图表和附录编号
- 支持单文件或按文件名排序的目录输入

## 验证结果

详情参见第{@chap:overview}章；支持的输出组合见表{@tbl:outputs}。

| 输出 | 样式 | 状态 |
|---|---|---|
| DOCX | research | 支持 |
| TeX / PDF | research | 支持 |

: 支持的研究报告输出 {#tbl:outputs}

<!-- [附录] -->

## 示例命令

```bash
mdx docx examples/research.md --style research -o report.docx
mdx tex examples/research.md --style research -o report.tex
```
