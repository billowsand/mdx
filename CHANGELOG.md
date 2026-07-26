# 更新日志

本项目的所有重要变更都会记录在此文件中，格式参考
[Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循
[语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

## [2.4.0] - 2026-07-26

### 新增

- 面向 GitHub 的项目首页、贡献指南、行为准则、安全策略与 Issue/PR 模板。
- 跨平台持续集成，以及由 `v*` 标签触发的多平台二进制发布工作流。
- Cargo 包元数据、MIT 许可证和自动发布说明分类；明确关闭 crates.io 发布，避免与现有同名 crate 冲突。
- GitHub 社交媒体预览图（`docs/assets/social-preview.png`）及可编辑 SVG 源文件。

### 变更

- 将 `Cargo.lock` 纳入版本控制，以保证 CLI 应用构建可复现。
- 开放跟踪用户语法文档与精选示例，让用户能直接从仓库开始体验。

## [2.3.0] - 2026-07-25

### 新增

- 加粗和斜体内容可嵌套链接、图片、交叉引用、代码及文献引用。

### 修复

- TeX PDF 编译优先使用 XeLaTeX，并在不可用时回退到 Tectonic。
- DOCX 输出会自动创建目标目录。