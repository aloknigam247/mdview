---
title: GFM fixture
tags: [gfm, markdown]
---

# GFM showcase

Paragraph with **bold**, *italic*, ***both***, ~~strikethrough~~, `inline code`,
a [link](https://example.com), and a literal autolink https://example.org.

## Lists

Unordered:

- apples
- bananas
  - nested item
  - another nested
- cherries

Ordered:

1. first
2. second
3. third

Task list:

- [x] write spec
- [ ] ship it
- [ ] celebrate

## Tables

| Feature       | Status | Notes                |
|---------------|:------:|----------------------|
| Headings      |   OK   | h1 – h6              |
| Tables        |   OK   | aligned columns      |
| Strikethrough |   OK   | ~~obsolete~~         |
| Autolink      |   OK   | https://example.com  |

## Blockquote

> ╭ rounded quote ╮
>
> A quote that spans
> multiple lines and contains **bold**
> and a [link](https://example.com).

## Thematic break

---

## Footnotes

Here is a reference[^note] and another[^again].

[^note]: A footnote body.
[^again]: A second footnote body with `code`.

## Hard/soft breaks

Line one.  
Line two after a hard break.

Line three
and a soft break.

## HTML inline (tagfilter on)

<p>Allowed paragraph.</p>
<script>alert("filtered")</script>
