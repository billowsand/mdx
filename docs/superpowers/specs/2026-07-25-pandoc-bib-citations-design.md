# Pandoc Bib Citations Design

## Goal

Add bibliography-backed Markdown citations to both TeX output styles. Authors
declare one BibTeX file in Markdown front matter and use the supported Pandoc
bracket forms `[@key]` and `[@a; @b]`. Conversion validates the bibliography
and cited keys before producing output, then emits LaTeX `\cite` commands.

## Scope

### Supported

- A single bibliography declaration in the Markdown document:

  ```yaml
  ---
  bibliography: references.bib
  ---
  ```

- A single citation: `[@key]`.
- Multiple citations in one marker: `[@a; @b]`.
- Citations in paragraphs, list items, and table cells.
- Citation emission for both `tex --style research` and
  `tex --style official`.
- Strict BibTeX parsing, duplicate-key detection, and cited-key validation.
- A readable literal fallback for Docx output.

### Not supported

- Multiple bibliography files.
- Bare narrative citations such as `@key`.
- Suppressed-author citations such as `[-@key]`.
- Citation locators, prefixes, or suffixes.
- Citations inside headings.
- Nested Markdown formatting inside citation markers.
- Native Word bibliography or citation fields.

## User-Facing Syntax

The bibliography is declared in the opening front matter. The path is resolved
relative to the input Markdown file's parent directory for file input, or
relative to the input directory for directory input.

```markdown
---
bibliography: sources/library.bib
---

已有研究给出了相同结论 [@zhang2024]。
两种方法可结合使用 [@li2023; @wang2022]。
```

The corresponding TeX is:

```tex
已有研究给出了相同结论 \cite{zhang2024}。
两种方法可结合使用 \cite{li2023,wang2022}。
```

In directory input mode, only the opening front matter of the first Markdown
file in filename sort order supplies document metadata and the bibliography
declaration. Later chapter files cannot replace it.

## Architecture

### Shared front matter

Create `src/common/front_matter.rs` and move the existing research cover-field
parsing into it. The parsed metadata includes the current cover fields plus:

```rust
pub bibliography: Option<String>
```

The parser returns both metadata and the Markdown body with the opening front
matter removed. An unclosed opening `---` block remains ordinary Markdown, as
it does today. Both TeX pipelines consume this shared result so official TeX
does not leak YAML keys into document content.

The input collection path must expose the unmodified Markdown content until
front matter is removed. Horizontal-rule stripping, where still required,
happens after metadata extraction.

### Citation AST

Extend `Inline` with:

```rust
Citation(Vec<String>)
```

The inline parser recognizes a bracket only when every semicolon-separated
component has the exact shape `@<non-empty-key>`, allowing surrounding
whitespace around components. A malformed or unsupported bracket expression
stays ordinary text.

The citation recognizer runs before links and basic emphasis parsing, while
inline-code extraction still protects code spans. Fenced code blocks continue
to bypass inline parsing. Table cells use the same inline parser and therefore
receive identical citation behavior.

### Bibliography validation

Create `src/common/citation.rs` with three responsibilities:

1. Traverse blocks and collect citation keys from paragraphs, lists, and table
   cells.
2. Validate the declared bibliography using `biblatex = "0.12"`.
3. After all validation succeeds, copy the source bibliography to the TeX
   output directory as `references.bib`.

`biblatex::Bibliography::parse` is the source of truth for BibTeX syntax,
duplicate citation keys, and the final key set. Missing cited keys are
deduplicated, sorted, and reported together.

The key comparison is exact and case-sensitive, matching BibTeX citation-key
semantics and LaTeX output.

### TeX emission and templates

Both TeX emitters render:

```rust
Inline::Citation(vec!["a", "b"]) -> r"\cite{a,b}"
```

Bibliography setup is enabled only when at least one citation exists:

```tex
\usepackage[style=gb7714-2015]{biblatex}
\addbibresource{references.bib}
```

Both output styles place `\printbibliography` before `\end{document}`. The
research template removes `\nocite{*}`, ensuring that only cited entries appear.
If a valid bibliography is declared but there are no citations, the file is
validated but not copied and no bibliography package or empty reference list is
generated.

Docx emitters reconstruct citations as `[@key]` or `[@a; @b]`. This keeps the
source meaning visible without claiming to create native Word citation fields.

## Processing Order and Side Effects

Both TeX pipelines follow this order:

1. Read and merge Markdown input.
2. Extract shared front matter.
3. Parse Markdown into blocks.
4. Collect citation keys.
5. Validate the bibliography declaration, file, syntax, duplicate keys, and
   cited keys.
6. Validate existing cross-references.
7. Create output directories and extract embedded LaTeX resources.
8. Copy `references.bib` when citations exist.
9. Relocate images.
10. Emit the main TeX file and chapter/appendix parts.
11. Attempt PDF compilation with the existing compiler workflow.

Citation or cross-reference validation failures occur before steps that create
or overwrite output files. Existing output from an earlier successful run is
left untouched; the failed run does not partially refresh it.

## Error Handling

The following are hard errors and preserve the program's non-zero exit behavior:

- A citation exists but `bibliography` is not declared.
- A declared bibliography path is missing, is not a file, or cannot be read.
- The bibliography is syntactically invalid.
- The bibliography contains a duplicate citation key.
- Any Markdown citation key is absent from the bibliography.

A declared bibliography is validated even when the document has no citations.
Unused BibTeX entries are accepted without warnings. Multiple missing citation
keys are reported in one deterministic, sorted message. Errors identify the
relevant Markdown input, bibliography path, or citation keys.

## Code Boundaries

- Create `src/common/front_matter.rs` for metadata extraction.
- Create `src/common/citation.rs` for citation collection, BibTeX validation,
  and successful-copy handling.
- Modify `src/common/ast.rs` and `src/common/inline.rs` for the citation node and
  syntax.
- Modify `src/input.rs` so metadata is extracted before horizontal-rule
  handling and directory metadata comes from the first file.
- Modify `src/tex_official.rs`, `src/tex_research/merger.rs`, and
  `src/tex_research_emitter.rs` for validation and TeX emission.
- Modify `resources/research/template.tex` to remove unconditional bibliography
  output and `\nocite{*}`.
- Update exhaustive `Inline` handling in Docx, table, image, cross-reference,
  and flattening code without changing those features' existing behavior.
- Update `README.md` and `docs/markdown-extensions.md`.
- Add `biblatex = "0.12"` and bump the feature version from `2.2.0` to `2.3.0`.

## Testing Strategy

Implementation follows red-green-refactor cycles.

### Inline parser tests

- `[@key]` becomes one citation key.
- `[@a; @b]` becomes two ordered keys.
- Citations coexist with surrounding text.
- Unsupported and malformed variants remain text.
- Inline and fenced code retain literal citation text.

### Front matter and input tests

- A single bibliography path is extracted and the YAML block is removed.
- An unclosed front matter block remains content.
- Directory input uses only the first sorted file's declaration.
- Relative paths resolve from the specified input base directory.

### Bibliography validation tests

- A valid BibTeX file and complete key set pass.
- Citation without a declaration fails.
- Missing, non-file, and unreadable paths fail.
- Malformed BibTeX fails.
- Duplicate citation keys fail.
- One or multiple missing citation keys fail with deterministic output.
- Unused entries pass silently.
- A declared valid BibTeX file is checked even with no citations.

### Emitter and pipeline tests

- Research and official TeX emit `\cite{key}` and `\cite{a,b}`.
- Docx-facing rendering preserves bracket citation text.
- Table citations emit LaTeX citations.
- Cited documents copy the Bib file as `references.bib`.
- Generated TeX contains conditional BibLaTeX setup and no `\nocite{*}`.
- A failed validation run creates no new TeX, Bib, image, part, or PDF output.
- Existing cross-reference, image, table, footnote, and chapter-splitting tests
  remain green.

## Version and Verification

This is a backward-compatible feature, so `Cargo.toml` advances from `2.2.0` to
`2.3.0`. Cargo updates `Cargo.lock` as part of dependency resolution.

Fresh completion verification consists of:

```text
cargo fmt --check
cargo test
cargo check
```

If `mdx` is already installed from this checkout, reinstall with
`cargo install --path .` and verify `mdx --version` reports `2.3.0`.
