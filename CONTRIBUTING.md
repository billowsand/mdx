# 贡献指南

感谢你愿意帮助改进 mdx。无论是问题报告、文档修订、测试用例还是代码贡献，都非常欢迎。

## 开始之前

- 使用新 Issue 前先搜索现有 Issue，避免重复。
- 安全漏洞请按 [SECURITY.md](SECURITY.md) 私下报告，不要提交公开 Issue。
- 较大的功能或行为变更，建议先通过 Feature request 说明使用场景与设计范围。

## 本地开发

需要 Rust 1.85 或更高版本。TeX 引擎不是运行单元测试的必需项。

```bash
git clone https://github.com/<your-name>/mdx.git
cd mdx
cargo build
cargo test
```

提交 Pull Request 前请运行：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

如需手动验证转换，可使用 `examples/` 中的示例：

```bash
cargo run -- docx examples/official.md --style official -o target/demo/official.docx
cargo run -- tex examples/research.md --style research -o target/demo/research.tex
```

## 代码与测试约定

- 保持改动聚焦，避免与目标无关的重构和格式化。
- 修改解析器或 emitter 行为时，请添加能够覆盖该行为的测试。
- 修改用户可见行为时，请同步更新 README、扩展语法文档及 CHANGELOG。
- 代码注释和用户输出可使用中文；Rust 标识符遵循项目现有英文命名风格。
- 每次代码修订都必须按 SemVer 更新 `Cargo.toml` 版本，并运行 Cargo 命令同步 `Cargo.lock`。

## Pull Request

请在 PR 中说明：

1. 解决了什么问题；
2. 为什么采用这一实现；
3. 如何验证，以及实际测试结果；
4. 是否有兼容性、格式输出或外部依赖变化。

提交贡献即表示你同意按本仓库的 [MIT License](LICENSE) 许可你的贡献。