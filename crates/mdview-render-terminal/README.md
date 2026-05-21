# mdview-render-terminal

ANSI (terminal) renderer for mdview. Walks a `comrak` AST and emits styled
text with rounded box-drawing for tables and fenced code blocks.

## Diagram placeholders

Mermaid, draw.io, and Plotly fenced blocks (`` ```mermaid ``, `` ```drawio ``,
`` ```plotly ``) are rendered as plain fenced code-frames carrying the raw
source, prefixed with the language label (e.g. `[ mermaid ]`). Terminal mode
does not invoke the Bun sidecar to rasterise diagrams into sixel — that full
SVG→sixel pipeline is tracked as a separate task. Today the frames act as
readable placeholders so the source remains visible without breaking layout.

## Image rendering

Local images are encoded to sixel via `mdview-sixel` when
`TerminalCaps::sixel == true`. Otherwise they fall back to a `[image: alt]`
placeholder line (also used for remote URLs and unresolvable paths).

## Inline HTML

`<sub>...</sub>` and `<sup>...</sup>` are mapped to Unicode subscript /
superscript glyphs where every inner character has a mapped codepoint
(`H<sub>2</sub>O` → `H₂O`, `mc<sup>2</sup>` → `mc²`). When any character is
unmappable, the literal `<sub>...</sub>` / `<sup>...</sup>` is preserved.

All other raw HTML inline / block content is emitted verbatim with the
`muted` theme style.
