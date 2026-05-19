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
    // Pass through inline HTML (e.g. <sub>, <sup>, <details>, <kbd>); we're a
    // local viewer for the user's own files, not a server rendering untrusted
    // markdown.
    opts.render.unsafe_ = true;
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
  html {{ scroll-behavior: smooth; }}
  html, body {{ background: var(--bg); color: var(--fg); }}
  body {{ font-family: "Inter", "Inter Variable", ui-sans-serif, system-ui, -apple-system,
                       "Segoe UI Variable Text", "Segoe UI", Roboto, sans-serif;
          margin: 32px; padding: 0; line-height: 1.7;
          min-width: min-content;
          -webkit-font-smoothing: antialiased; text-rendering: optimizeLegibility; }}
  * {{ scrollbar-width: thin; scrollbar-color: color-mix(in srgb, var(--accent) 55%, transparent) transparent; }}
  *::-webkit-scrollbar {{ width: 10px; height: 10px; }}
  *::-webkit-scrollbar-track {{ background: transparent; }}
  *::-webkit-scrollbar-thumb {{ background: color-mix(in srgb, var(--accent) 55%, transparent);
                                border-radius: 8px; border: 2px solid transparent;
                                background-clip: padding-box; }}
  *::-webkit-scrollbar-thumb:hover {{ background: var(--accent); background-clip: padding-box;
                                       border: 2px solid transparent; }}
  *::-webkit-scrollbar-corner {{ background: transparent; }}
  article.mdv {{ max-width: none; }}
  article.mdv h1, article.mdv h2, article.mdv h3, article.mdv h4 {{ line-height: 1.25; }}
  article.mdv h1 {{ font-size: 2.1em; border-bottom: 1px solid var(--border); padding-bottom: .2em; }}
  article.mdv h2 {{ font-size: 1.6em; margin-top: 1.8em; }}
  article.mdv a {{ color: var(--link); text-decoration: none; }}
  article.mdv a:hover {{ text-decoration: underline; }}
  article.mdv pre {{ background: var(--code-bg); padding: 16px; border-radius: 10px;
                     overflow-x: auto; border: 1px solid var(--border);
                     box-shadow: 0 1px 2px rgba(0,0,0,.04);
                     white-space: pre; }}
  article.mdv code {{ font-family: "JetBrains Mono", "JetBrainsMono Nerd Font", "Cascadia Code",
                                    "Cascadia Mono", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
                      font-size: .92em; font-variant-ligatures: contextual; }}
  article.mdv :not(pre) > code {{ background: var(--code-bg); padding: 2px 6px;
                                  border-radius: 4px; border: 1px solid var(--border); }}
  article.mdv table {{ border-collapse: separate; border-spacing: 0;
                       border: 1px solid var(--border); border-radius: 10px; overflow: hidden;
                       box-shadow: 0 1px 2px rgba(0,0,0,.04); }}
  article.mdv th, article.mdv td {{ border-bottom: 1px solid var(--border);
                                    padding: 10px 14px; text-align: left; }}
  /* GFM-style alignment: comrak emits align="left|center|right" on <th>/<td>.
     These selectors defer to the explicit attribute (a CSS text-align: left
     alone would otherwise suppress center/right from the markdown :--: syntax). */
  article.mdv th[align="center"], article.mdv td[align="center"] {{ text-align: center; }}
  article.mdv th[align="right"],  article.mdv td[align="right"]  {{ text-align: right; }}
  article.mdv th[align="left"],   article.mdv td[align="left"]   {{ text-align: left; }}
  article.mdv tr:last-child td {{ border-bottom: 0; }}
  article.mdv th {{ background: var(--code-bg); font-weight: 600; }}
  article.mdv blockquote {{ border-left: 3px solid var(--accent); margin: 0;
                            padding: 6px 16px; color: var(--muted); background: var(--code-bg);
                            border-radius: 0 10px 10px 0; }}
  article.mdv hr {{ border: 0; height: 1px; background: var(--border); margin: 2em 0; }}
  article.mdv img {{ max-width: 100%; border-radius: 8px; }}
  article.mdv ul, article.mdv ol {{ padding-inline-start: 24px; }}
  article.mdv .mermaid, article.mdv .plotly-chart, article.mdv .drawio-viewer {{ margin: 24px 0; }}
  /* Cap diagram containers to viewport width so they don't balloon with the
     expanded page width from long content elsewhere on the page. */
  article.mdv .plotly-chart {{ max-width: min(900px, calc(100vw - 64px)); }}
  article.mdv .mermaid, article.mdv .drawio-viewer, article.mdv .mxgraph {{
    max-width: calc(100vw - 64px); overflow-x: auto;
  }}
  /* Keep display math aligned to the content area rather than centering across
     the full page width (which would push it offscreen when long content
     has stretched the body). */
  article.mdv .mdv-math-block, article.mdv .katex-display, article.mdv [data-math-style="display"] {{
    text-align: left; width: fit-content; max-width: 100%; margin: 12px 0;
  }}
  article.mdv .mdv-math, article.mdv [data-math-style="inline"] {{ display: inline-block; }}
  article.mdv pre.mdv-code {{ padding: 0; }}
  article.mdv pre.mdv-code code {{ display: block; padding: 16px; }}
  #mdv-minimap {{ position: fixed; top: 16px; right: 16px; width: 140px; bottom: 16px;
                  background: color-mix(in srgb, var(--bg) 92%, transparent);
                  border: 1px solid var(--border); border-radius: 12px;
                  overflow: hidden; z-index: 1000;
                  box-shadow: 0 4px 18px rgba(0,0,0,.10);
                  cursor: ns-resize; transition: opacity .15s ease;
                  backdrop-filter: blur(6px); }}
  #mdv-minimap.mdv-hidden {{ opacity: 0; pointer-events: none; }}
  #mdv-minimap-content {{ transform-origin: top left; pointer-events: none;
                          user-select: none; position: absolute; top: 6px; left: 6px; }}
  #mdv-minimap-content article.mdv {{ margin: 0; }}
  #mdv-minimap-viewport {{ position: absolute; left: 0; right: 0;
                           border: 1px solid var(--accent);
                           background: color-mix(in srgb, var(--accent) 18%, transparent);
                           border-radius: 4px; pointer-events: none; }}
  @media (max-width: 760px) {{ #mdv-minimap {{ display: none; }} }}
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
    // Math rendering: covers comrak's inline/block spans AND the math extension's
    // .mdv-math / .mdv-math-block (which carry the raw TeX in data-tex).
    if (window.katex) {{
      document.querySelectorAll('[data-math-style]').forEach(el => {{
        const display = el.getAttribute('data-math-style') === 'display';
        try {{ katex.render(el.textContent, el, {{ displayMode: display, throwOnError: false }}); }}
        catch (e) {{ console.warn('katex:', e); }}
      }});
      document.querySelectorAll('.mdv-math, .mdv-math-block').forEach(el => {{
        const display = el.classList.contains('mdv-math-block');
        const tex = el.getAttribute('data-tex') || el.textContent;
        try {{ katex.render(tex, el, {{ displayMode: display, throwOnError: false }}); }}
        catch (e) {{ console.warn('katex:', e); }}
      }});
    }}
    if (window.mermaid) {{
      const __mdvDark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
      const __mdvMermaidVars = __mdvDark ? {{
        primaryColor: '#313244', primaryTextColor: '#cdd6f4', primaryBorderColor: '#cba6f7',
        lineColor: '#a6adc8', secondaryColor: '#fab387', tertiaryColor: '#89b4fa',
        background: '#1e1e2e', mainBkg: '#313244', secondBkg: '#45475a', tertiaryBkg: '#585b70',
        nodeBorder: '#cba6f7', clusterBkg: '#313244', clusterBorder: '#cba6f7',
        defaultLinkColor: '#a6adc8', titleColor: '#cdd6f4', edgeLabelBackground: '#313244',
        actorBkg: '#313244', actorBorder: '#cba6f7', actorTextColor: '#cdd6f4',
      }} : {{
        primaryColor: '#ccd0da', primaryTextColor: '#4c4f69', primaryBorderColor: '#8839ef',
        lineColor: '#6c6f85', secondaryColor: '#fe640b', tertiaryColor: '#1e66f5',
        background: '#eff1f5', mainBkg: '#ccd0da', secondBkg: '#bcc0cc', tertiaryBkg: '#acb0be',
        nodeBorder: '#8839ef', clusterBkg: '#ccd0da', clusterBorder: '#8839ef',
        defaultLinkColor: '#6c6f85', titleColor: '#4c4f69', edgeLabelBackground: '#ccd0da',
        actorBkg: '#ccd0da', actorBorder: '#8839ef', actorTextColor: '#4c4f69',
      }};
      mermaid.initialize({{ startOnLoad: false, theme: 'base', themeVariables: __mdvMermaidVars }});
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

    const __mdvSetupMinimap = () => {{
      const article = document.querySelector('article.mdv');
      if (!article) return;
      if (document.getElementById('mdv-minimap')) return;

      const minimap = document.createElement('div');
      minimap.id = 'mdv-minimap';

      const content = document.createElement('div');
      content.id = 'mdv-minimap-content';
      minimap.appendChild(content);

      const clone = article.cloneNode(true);
      clone.querySelectorAll('[id]').forEach(el => el.removeAttribute('id'));
      clone.querySelectorAll('script, style').forEach(el => el.remove());
      content.appendChild(clone);

      const viewport = document.createElement('div');
      viewport.id = 'mdv-minimap-viewport';
      minimap.appendChild(viewport);

      document.body.appendChild(minimap);

      let scale = 0.1;
      const PAD = 6;
      const update = () => {{
        const mapW = minimap.clientWidth - PAD * 2;
        const mapH = minimap.clientHeight - PAD * 2;
        const docW = Math.max(article.scrollWidth, article.clientWidth, 1);
        const docH = Math.max(document.documentElement.scrollHeight, 1);
        scale = Math.min(mapW / docW, mapH / docH);
        if (!isFinite(scale) || scale <= 0) scale = 0.1;
        content.style.transform = `scale(${{scale}})`;
        content.style.width = docW + 'px';
        content.style.height = docH + 'px';

        const viewH = window.innerHeight;
        viewport.style.top = (PAD + window.scrollY * scale) + 'px';
        viewport.style.height = Math.max(8, viewH * scale) + 'px';
      }};

      update();
      window.addEventListener('scroll', update, {{ passive: true }});
      window.addEventListener('resize', update);

      let dragging = false;
      const scrollFromY = (clientY) => {{
        const rect = minimap.getBoundingClientRect();
        const y = clientY - rect.top - PAD;
        const targetY = y / scale - window.innerHeight / 2;
        const maxY = document.documentElement.scrollHeight - window.innerHeight;
        window.scrollTo({{ top: Math.max(0, Math.min(maxY, targetY)), behavior: 'auto' }});
      }};
      minimap.addEventListener('mousedown', (e) => {{
        dragging = true;
        scrollFromY(e.clientY);
        e.preventDefault();
      }});
      window.addEventListener('mousemove', (e) => {{ if (dragging) scrollFromY(e.clientY); }});
      window.addEventListener('mouseup', () => {{ dragging = false; }});

      document.addEventListener('keydown', (e) => {{
        const tag = (document.activeElement && document.activeElement.tagName) || '';
        if (e.key === 'm' && !e.ctrlKey && !e.metaKey && !e.altKey
            && tag !== 'INPUT' && tag !== 'TEXTAREA') {{
          minimap.classList.toggle('mdv-hidden');
          if (!minimap.classList.contains('mdv-hidden')) update();
        }}
      }});

      const docTaller = document.documentElement.scrollHeight > window.innerHeight + 100;
      if (!docTaller) minimap.classList.add('mdv-hidden');
    }};
    setTimeout(__mdvSetupMinimap, 500);
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

    #[test]
    fn includes_minimap_scaffold() {
        let html = render_page("# Hi\n\nsome text\n", "t").expect("render");
        assert!(html.contains("#mdv-minimap"), "expected minimap CSS");
        assert!(html.contains("__mdvSetupMinimap"), "expected minimap JS");
    }
}
