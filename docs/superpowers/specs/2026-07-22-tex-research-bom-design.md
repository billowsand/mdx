# TeX Research UTF-8 BOM 兼容设计

## 目标

修复 `mdx tex <directory> --style research` 合并多份 Markdown 时，因目录中某个文件以 UTF-8 BOM 开头而无法识别该文件首行 Markdown 语法的问题。

## 根因

`src/tex_research/merger.rs` 使用 `fs::read_to_string` 逐个读取 Markdown 文件后直接拼接。Rust 会把 UTF-8 BOM 解码为 `U+FEFF`，因此非首个文件的 BOM 会保留在合并字符串中。若文件首行是标题或 research 区段标记，解析器实际看到的是 `\u{FEFF}## ...` 或 `\u{FEFF}<!-- [...] -->`，从而将其当作普通段落。

在 `rws-chapters` 中，`03` 至 `06` 章含 BOM，导致这些章未生成 `\\chapter{}`；`A`、`B` 附录也含 BOM，导致附录标记或标题未被识别。

## 方案

在 `tex research` 的 Markdown 文件读取边界移除文件开头的 UTF-8 BOM，再执行标题提取、首个 H1 移除和目录合并。处理应同时覆盖目录输入与单文件输入，只移除内容开头的 `U+FEFF`，不改写源文件，也不删除正文内部字符。

不在通用解析器中忽略每一行开头的 BOM，因为这会扩大语义范围并可能掩盖正文中的异常字符；也不直接修改 `rws-chapters`，因为程序仍会在其他带 BOM 的输入上复现。

## 测试

先添加回归测试，创建一个临时 Markdown 目录，其中第二个文件以 BOM 加 `## 第三章` 开头。修复前应观察到章节未生成；修复后应生成预期 `\\chapter{}`，且输出中不残留转义后的字面量标题。

同时运行：

- 新增的定向回归测试；
- 全部 `cargo test`；
- 使用真实 `rws-chapters` 执行 `tex research`；
- 检查第三至第六章均生成 `\\chapter{}`，附录标记生效，并确认 PDF 编译成功。

## 非目标

- 不修改 `rws-chapters` 的文件编码。
- 不重构 Markdown 解析器或其他输出分支。
- 不处理正文内部任意位置的 BOM。
