use anyhow::Result;
use comrak::{format_html, Arena};
use mdview_core::{parse, Registry, RenderCtx, Theme};

use crate::builtins::builtin_extensions;

pub fn render_page(src: &str, title: &str) -> Result<String> {
    let mut registry = Registry::new();
    for ext in builtin_extensions() {
        registry.register(ext);
    }

    let mut src_owned = src.to_string();
    registry.apply_pre_parse(&mut src_owned);

    let arena = Arena::new();
    let ast = parse(&arena, &src_owned, &registry);
    registry.apply_transforms(ast);

    let theme = Theme::default();
    let ctx = RenderCtx::new(&theme);

    let mut opts = comrak::ComrakOptions::default();
    opts.extension.strikethrough = true;
    opts.extension.tasklist = true;
    opts.extension.table = true;
    opts.extension.autolink = true;
    opts.extension.footnotes = true;
    opts.extension.math_dollars = true;
    opts.extension.math_code = true;
    registry.apply_parser_opts(&mut opts);

    let mut body = String::new();
    for child in ast.children() {
        let mut matched = false;
        for ext in registry.html_renderers() {
            if let Some(html) = ext.render_html(child, &ctx) {
                body.push_str(&html.0);
                matched = true;
                break;
            }
        }
        if !matched {
            let mut buf: Vec<u8> = Vec::new();
            format_html(child, &opts, &mut buf)?;
            body.push_str(&String::from_utf8_lossy(&buf));
        }
    }

    Ok(wrap_page(&body, title))
}

fn wrap_page(body: &str, title: &str) -> String {
    let title_esc = html_escape(title);
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{title_esc}</title>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css">
<style>
  :root {{
    --fg: #111827; --bg: #ffffff; --muted: #6b7280; --accent: #6c7cff;
    --code-bg: #f6f8fa; --border: #e5e7eb; --link: #2563eb;
  }}
  @media (prefers-color-scheme: dark) {{
    :root {{ --fg: #e5e7eb; --bg: #0d1117; --muted: #9ca3af; --accent: #8b9eff;
             --code-bg: #161b22; --border: #30363d; --link: #8ab4f8; }}
  }}
  html, body {{ background: var(--bg); color: var(--fg); }}
  body {{ font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, sans-serif;
          max-width: 860px; margin: 40px auto; padding: 0 24px; line-height: 1.7; }}
  article.mdv h1, article.mdv h2, article.mdv h3, article.mdv h4 {{ line-height: 1.25; }}
  article.mdv h1 {{ font-size: 2.1em; border-bottom: 1px solid var(--border); padding-bottom: .2em; }}
  article.mdv h2 {{ font-size: 1.6em; margin-top: 1.8em; }}
  article.mdv a {{ color: var(--link); text-decoration: none; }}
  article.mdv a:hover {{ text-decoration: underline; }}
  article.mdv pre {{ background: var(--code-bg); padding: 16px; border-radius: 10px;
                     overflow-x: auto; border: 1px solid var(--border);
                     box-shadow: 0 1px 2px rgba(0,0,0,.04); }}
  article.mdv code {{ font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
                      font-size: .92em; }}
  article.mdv :not(pre) > code {{ background: var(--code-bg); padding: 2px 6px;
                                  border-radius: 4px; border: 1px solid var(--border); }}
  article.mdv table {{ border-collapse: separate; border-spacing: 0;
                       border: 1px solid var(--border); border-radius: 10px; overflow: hidden;
                       box-shadow: 0 1px 2px rgba(0,0,0,.04); }}
  article.mdv th, article.mdv td {{ border-bottom: 1px solid var(--border);
                                    padding: 10px 14px; text-align: left; }}
  article.mdv tr:last-child td {{ border-bottom: 0; }}
  article.mdv th {{ background: var(--code-bg); font-weight: 600; }}
  article.mdv blockquote {{ border-left: 3px solid var(--accent); margin: 0;
                            padding: 6px 16px; color: var(--muted); background: var(--code-bg);
                            border-radius: 0 10px 10px 0; }}
  article.mdv hr {{ border: 0; height: 1px; background: var(--border); margin: 2em 0; }}
  article.mdv img {{ max-width: 100%; border-radius: 8px; }}
  article.mdv .mermaid, article.mdv .plotly-chart, article.mdv .drawio-viewer {{ margin: 24px 0; }}
  article.mdv pre.mdv-code {{ padding: 0; }}
  article.mdv pre.mdv-code code {{ display: block; padding: 16px; }}
</style>
</head>
<body>
<article class="mdv">{body}</article>
<script src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/contrib/auto-render.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js"></script>
<script src="https://cdn.plot.ly/plotly-2.35.2.min.js"></script>
<script src="https://viewer.diagrams.net/js/viewer-static.min.js"></script>
<script>
  window.addEventListener('DOMContentLoaded', () => {{
    // comrak emits <span data-math-style="inline|display">TEX</span>; render each with katex.
    if (window.katex) {{
      document.querySelectorAll('[data-math-style]').forEach(el => {{
        const display = el.getAttribute('data-math-style') === 'display';
        try {{
          katex.render(el.textContent, el, {{ displayMode: display, throwOnError: false }});
        }} catch (e) {{ console.warn('katex:', e); }}
      }});
    }}
    // Fenced ```math blocks — lifted out of their <pre><code> wrapper and rendered.
    document.querySelectorAll('pre.mdv-code[data-lang="math"] code, pre code.language-math').forEach(el => {{
      const pre = el.closest('pre');
      const span = document.createElement('div');
      span.setAttribute('data-math-style', 'display');
      span.textContent = el.textContent.trim();
      pre.replaceWith(span);
      if (window.katex) {{
        try {{ katex.render(span.textContent, span, {{ displayMode: true, throwOnError: false }}); }}
        catch (e) {{ console.warn('katex:', e); }}
      }}
    }});
    if (window.mermaid) {{
      mermaid.initialize({{ startOnLoad: false, theme: 'default' }});
      mermaid.run({{ querySelector: '.mermaid' }}).catch(e => console.warn('mermaid:', e));
    }}
    document.querySelectorAll('.plotly-chart[data-spec]').forEach(el => {{
      try {{
        const spec = JSON.parse(el.getAttribute('data-spec'));
        Plotly.newPlot(el, spec.data || [], spec.layout || {{}}, {{responsive: true}});
      }} catch (e) {{ console.warn('plotly:', e); }}
    }});
    // Drawio: the extension emits <div class="drawio-viewer" data-xml-b64="...">.
    // Rewrite each into the `class="mxgraph" data-mxgraph='{{xml:...}}'` form the
    // real drawio viewer-static expects, then invoke GraphViewer.processElements.
    document.querySelectorAll('.drawio-viewer[data-xml-b64]').forEach(el => {{
      try {{
        const b64 = el.getAttribute('data-xml-b64');
        const xml = decodeURIComponent(atob(b64).split('').map(c =>
          '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2)).join(''));
        el.className = 'mxgraph';
        el.removeAttribute('data-xml-b64');
        el.setAttribute('data-mxgraph', JSON.stringify({{
          xml, highlight: '#0000ff', lightbox: false, nav: true,
          resize: true, toolbar: 'zoom layers tags lightbox',
        }}));
      }} catch (e) {{ console.warn('drawio rewrite:', e); }}
    }});
    if (window.GraphViewer && typeof window.GraphViewer.processElements === 'function') {{
      try {{
        // drawio takes a class name (no leading dot) or its default ("mxgraph")
        window.GraphViewer.processElements('mxgraph');
      }} catch (e) {{ console.warn('drawio:', e); }}
    }}
  }});
</script>
</body>
</html>
"##
    )
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_heading() {
        let html = render_page("# Hello", "test").expect("render");
        assert!(html.contains("<h1>"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn renders_table_with_rounded_css() {
        let html = render_page("| a | b |\n|---|---|\n| 1 | 2 |\n", "t").expect("render");
        assert!(html.contains("<table>"));
        assert!(html.contains("border-radius"));
    }

    #[test]
    fn mermaid_fenced_becomes_mermaid_div() {
        let html =
            render_page("```mermaid\ngraph TD; A-->B;\n```\n", "t").expect("render");
        assert!(html.contains("class=\"mermaid\""), "html: {html}");
    }

    #[test]
    fn code_block_goes_through_highlight_extension() {
        let html = render_page("```rust\nfn main(){}\n```\n", "t").expect("render");
        assert!(html.contains("mdv-code") || html.contains("language-rust"));
    }
}
