use anyhow::Result;
use comrak::nodes::{AstNode, NodeValue};
use comrak::{format_html, Arena};
#[allow(unused_imports)]
use mdview_config::TocPosition;
use mdview_config::{
    Action, CodeConfig, CodemapConfig, ConfigError, KeyBinding, Keymap, ThemeConfig, ThemeMode,
    TocConfig,
};
use mdview_core::{parse, Registry, RenderCtx, Theme};
use mdview_ext_highlight::Highlight;
use std::path::{Path, PathBuf};

use crate::builtins::builtin_extensions;

#[derive(Default, Debug, Clone, Copy)]
struct Features {
    drawio: bool,
    math: bool,
    mermaid: bool,
    plotly: bool,
}

impl Features {
    fn detect<'a>(root: &'a AstNode<'a>) -> Self {
        let mut f = Features::default();
        for node in root.descendants() {
            match &node.data.borrow().value {
                NodeValue::Math(_) => f.math = true,
                NodeValue::CodeBlock(cb) => {
                    let head = cb.info.split_whitespace().next().unwrap_or("");
                    let lang = head.split(':').next().unwrap_or("");
                    if lang.eq_ignore_ascii_case("mermaid") {
                        f.mermaid = true;
                    } else if lang.eq_ignore_ascii_case("drawio") {
                        f.drawio = true;
                    } else if lang.eq_ignore_ascii_case("plotly") {
                        f.plotly = true;
                    } else if lang.eq_ignore_ascii_case("math") {
                        f.math = true;
                    }
                }
                _ => {}
            }
        }
        f
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolvedMode {
    Dark,
    Light,
}

impl ResolvedMode {
    fn class(self) -> &'static str {
        match self {
            ResolvedMode::Dark => "theme-dark",
            ResolvedMode::Light => "theme-light",
        }
    }
}

fn resolve_mode(mode: ThemeMode) -> ResolvedMode {
    if mode.resolve_is_light() {
        ResolvedMode::Light
    } else {
        ResolvedMode::Dark
    }
}

fn lookup_theme_or_fallback(name: &str, fallback: &str) -> mdview_theme::Theme {
    if let Some(t) = mdview_theme::find(name) {
        return t.clone();
    }
    tracing::warn!("mdview-render: theme {name:?} not found; falling back to {fallback:?}");
    mdview_theme::find(fallback)
        .cloned()
        .or_else(|| mdview_theme::builtin_themes().into_iter().next().cloned())
        .unwrap_or_default()
}

/// Hex `#rrggbb` background color of the theme that will paint first, given the
/// resolved (config-only, pre-sessionStorage) mode. Falls back to the catppuccin
/// mocha base when the configured theme can't be resolved.
pub fn initial_bg_hex(theme_cfg: &ThemeConfig) -> String {
    let mode = resolve_mode(theme_cfg.mode);
    let (name, fallback) = match mode {
        ResolvedMode::Dark => (theme_cfg.dark.as_str(), "catppuccin-mocha"),
        ResolvedMode::Light => (theme_cfg.light.as_str(), "catppuccin-latte"),
    };
    let theme = lookup_theme_or_fallback(name, fallback);
    theme
        .colors
        .get("bg")
        .cloned()
        .unwrap_or_else(|| "#1e1e2e".to_string())
}

fn emit_root_inner(theme: &mdview_theme::Theme) -> String {
    let css = mdview_theme::emit_css(theme);
    let start = match css.find(":root {") {
        Some(i) => i + ":root {".len(),
        None => return String::new(),
    };
    let rest = &css[start..];
    let end = match rest.find('}') {
        Some(i) => i,
        None => return String::new(),
    };
    rest[..end].trim().to_string()
}

fn emit_theme_blocks(config: &ThemeConfig) -> String {
    let light = lookup_theme_or_fallback(&config.light, "catppuccin-latte");
    let dark = lookup_theme_or_fallback(&config.dark, "catppuccin-mocha");
    let light_inner = emit_root_inner(&light);
    let dark_inner = emit_root_inner(&dark);
    format!(
        ":root.theme-light {{\n{light_inner}\n}}\n:root.theme-dark {{\n{dark_inner}\n}}\n:root{{--bg:var(--mdv-bg);--fg:var(--mdv-fg);--accent:var(--mdv-accent);--muted:var(--mdv-muted);--code-bg:var(--mdv-code-bg);--border:var(--mdv-border-subtle);--link:var(--mdv-link);}}\n"
    )
}

#[allow(dead_code)]
pub fn render_page(src: &str, title: &str) -> Result<String> {
    render_page_with_theme(src, title, &ThemeConfig::default())
}

#[allow(dead_code)]
pub fn render_page_with_theme(src: &str, title: &str, theme_cfg: &ThemeConfig) -> Result<String> {
    render_page_with_config(src, title, theme_cfg, &Keymap::defaults())
}

pub fn render_page_with_config(
    src: &str,
    title: &str,
    theme_cfg: &ThemeConfig,
    keymap: &Keymap,
) -> Result<String> {
    render_page_with_config_and_source(src, title, theme_cfg, keymap, None)
}

pub fn render_page_with_config_and_source(
    src: &str,
    title: &str,
    theme_cfg: &ThemeConfig,
    keymap: &Keymap,
    source_dir: Option<&Path>,
) -> Result<String> {
    render_page_full(
        src,
        title,
        theme_cfg,
        keymap,
        source_dir,
        &[],
        &TocConfig::default(),
        &CodemapConfig::default(),
        &CodeConfig::default(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn render_page_full(
    src: &str,
    title: &str,
    theme_cfg: &ThemeConfig,
    keymap: &Keymap,
    source_dir: Option<&Path>,
    config_errors: &[ConfigError],
    toc: &TocConfig,
    codemap: &CodemapConfig,
    code: &CodeConfig,
) -> Result<String> {
    let mut registry = Registry::new();
    for ext in builtin_extensions() {
        registry.register(ext);
    }

    let mut src_owned = src.to_string();
    registry.apply_pre_parse(&mut src_owned);

    let arena = Arena::new();
    let ast = parse(&arena, &src_owned, &registry);
    registry.apply_transforms(ast);
    rewrite_image_urls(ast, source_dir);

    let theme = Theme::default();
    let mut ctx = RenderCtx::new(&theme);
    ctx.source_dir = source_dir.map(|p| p.to_path_buf());
    ctx.tab_width = code.tab_width;

    highlight_inline_hashbang(ast, &ctx, false);

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

    // Detect diagram features from live CodeBlock info strings before the pre-render pass rewrites
    // nested CodeBlock nodes to HtmlInline; otherwise nested diagrams would not inject their library.
    let features = Features::detect(ast);
    pre_render_nested_code_blocks(ast, &registry, &ctx);

    let mut body = String::new();
    for ext in registry.html_renderers() {
        if let Some(html) = ext.pre_render_html(&ctx) {
            body.push_str(&html.0);
        }
    }
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
            let raw = String::from_utf8_lossy(&buf);
            // Replace comrak's default GFM task-list <input type="checkbox" ...>
            // markers with inline Fluent System Icons SVGs (CheckmarkCircle /
            // Circle). Keeps the swap consistent with mdview-render-html.
            body.push_str(&mdview_render_html::swap_task_checkboxes(&raw));
        }
    }

    let body = add_img_lazy_attrs(body);
    Ok(wrap_page(
        &body,
        title,
        theme_cfg,
        keymap,
        config_errors,
        features,
        toc,
        codemap,
        code,
    ))
}

fn rewrite_image_urls<'a>(root: &'a AstNode<'a>, source_dir: Option<&Path>) {
    for node in root.descendants() {
        let mut data = node.data.borrow_mut();
        if let NodeValue::Image(link) = &mut data.value {
            if let Some(u) = resolve_to_mdview_url(&link.url, source_dir) {
                link.url = u;
            }
        }
    }
}

fn highlight_inline_hashbang<'a>(node: &'a AstNode<'a>, ctx: &RenderCtx<'_>, in_paragraph: bool) {
    let (is_link, is_paragraph, code_literal) = {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::Link(_) => (true, false, None),
            NodeValue::Paragraph => (false, true, None),
            NodeValue::Code(c) => (false, false, Some(c.literal.clone())),
            _ => (false, false, None),
        }
    };
    if is_link {
        return;
    }
    let in_paragraph = in_paragraph || is_paragraph;
    if in_paragraph {
        if let Some(literal) = code_literal {
            if let Some(html) = Highlight::render_inline_html_literal(&literal, ctx) {
                node.data.borrow_mut().value = NodeValue::HtmlInline(html.0);
                return;
            }
        }
    }
    for child in node.children() {
        highlight_inline_hashbang(child, ctx, in_paragraph);
    }
}

// The top-level dispatch loop in `render_page_full` only runs extensions over direct children of the
// document root, so a fenced code block nested in a list/blockquote never reaches an extension and
// falls back to comrak's plain output (no syntect, no hl_lines). Pre-render each nested fenced
// CodeBlock into HtmlInline here so `format_html` emits the extension output verbatim in place.
// Direct children of the root are left untouched for the existing dispatch loop.
fn pre_render_nested_code_blocks<'a>(
    root: &'a AstNode<'a>,
    registry: &Registry,
    ctx: &RenderCtx<'_>,
) {
    for node in root.descendants() {
        let is_nested_fenced = {
            let data = node.data.borrow();
            matches!(&data.value, NodeValue::CodeBlock(cb) if cb.fenced)
                && node.parent().is_some_and(|p| !p.same_node(root))
        };
        if !is_nested_fenced {
            continue;
        }
        let mut rendered = None;
        for ext in registry.html_renderers() {
            if let Some(html) = ext.render_html(node, ctx) {
                rendered = Some(html.0);
                break;
            }
        }
        if let Some(html) = rendered {
            node.data.borrow_mut().value = NodeValue::HtmlInline(html);
        }
    }
}

#[cfg(windows)]
const MDVIEW_PROTOCOL_BASE: &str = "http://mdview.localhost/";
#[cfg(not(windows))]
const MDVIEW_PROTOCOL_BASE: &str = "mdview://localhost/";

fn resolve_to_mdview_url(url: &str, source_dir: Option<&Path>) -> Option<String> {
    if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("data:")
        || url.starts_with("mdview://")
        || url.starts_with(MDVIEW_PROTOCOL_BASE)
    {
        return None;
    }
    let raw = PathBuf::from(url);
    let abs: PathBuf = if raw.is_absolute() {
        raw
    } else if let Some(dir) = source_dir {
        dir.join(raw)
    } else {
        return None;
    };
    let abs = std::fs::canonicalize(&abs).unwrap_or(abs);
    let s = abs.to_string_lossy();
    let stripped = s.strip_prefix(r"\\?\").unwrap_or(&s);
    Some(format!(
        "{MDVIEW_PROTOCOL_BASE}{}",
        urlencoding::encode(stripped)
    ))
}

fn binding_to_json(b: &KeyBinding) -> String {
    use mdview_config::keymap::Key;
    let (key_kind, key_value) = match b.key {
        Key::Char(c) => ("char", c.to_ascii_lowercase().to_string()),
        Key::Backspace => ("named", "Backspace".into()),
        Key::Delete => ("named", "Delete".into()),
        Key::Down => ("named", "ArrowDown".into()),
        Key::End => ("named", "End".into()),
        Key::Enter => ("named", "Enter".into()),
        Key::Esc => ("named", "Escape".into()),
        Key::F(n) => ("named", format!("F{n}")),
        Key::Home => ("named", "Home".into()),
        Key::Left => ("named", "ArrowLeft".into()),
        Key::PageDown => ("named", "PageDown".into()),
        Key::PageUp => ("named", "PageUp".into()),
        Key::Right => ("named", "ArrowRight".into()),
        Key::Space => ("named", " ".into()),
        Key::Tab => ("named", "Tab".into()),
        Key::Up => ("named", "ArrowUp".into()),
    };
    let esc = key_value.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "{{\"ctrl\":{},\"shift\":{},\"alt\":{},\"super\":{},\"kind\":\"{}\",\"key\":\"{}\"}}",
        b.ctrl, b.shift, b.alt, b.super_, key_kind, esc
    )
}

fn keymap_json(keymap: &Keymap) -> String {
    let mut entries: Vec<String> = Vec::new();
    if let Some(b) = keymap.get(Action::Quit) {
        entries.push(format!("\"quit\":{}", binding_to_json(b)));
    }
    if let Some(b) = keymap.get(Action::ToggleBionic) {
        entries.push(format!("\"toggle-bionic\":{}", binding_to_json(b)));
    }
    if let Some(b) = keymap.get(Action::ToggleCodemap) {
        entries.push(format!("\"toggle-codemap\":{}", binding_to_json(b)));
    }
    if let Some(b) = keymap.get(Action::ToggleTheme) {
        entries.push(format!("\"toggle-theme\":{}", binding_to_json(b)));
    }
    if let Some(b) = keymap.get(Action::ToggleToc) {
        entries.push(format!("\"toggle-toc\":{}", binding_to_json(b)));
    }
    format!("{{{}}}", entries.join(","))
}

fn build_head_extras(features: Features) -> String {
    let mut s = String::new();
    if features.math {
        s.push_str("<link rel=\"stylesheet\" href=\"https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css\">\n");
    }
    s
}

fn build_lib_scripts(features: Features) -> String {
    let mut s = String::new();
    if features.math {
        s.push_str("<script defer src=\"https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js\"></script>\n");
        s.push_str("<script defer src=\"https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/contrib/auto-render.min.js\"></script>\n");
    }
    if features.mermaid {
        s.push_str("<script defer src=\"https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js\"></script>\n");
    }
    if features.plotly {
        s.push_str("<script defer src=\"https://cdn.plot.ly/plotly-2.35.2.min.js\"></script>\n");
    }
    if features.drawio {
        s.push_str(
            "<script defer src=\"https://viewer.diagrams.net/js/viewer-static.min.js\"></script>\n",
        );
    }
    s
}

fn add_img_lazy_attrs(html: String) -> String {
    html.replace("<img ", "<img loading=\"lazy\" decoding=\"async\" ")
}

fn minify_head(html: String) -> String {
    let Some(end) = html.find("</head>") else {
        return html;
    };
    let head_end = end + "</head>".len();
    let head = &html[..head_end];
    let rest = &html[head_end..];
    let mut out = String::with_capacity(html.len());
    let mut last_blank = false;
    for line in head.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            if !last_blank {
                out.push('\n');
                last_blank = true;
            }
            continue;
        }
        out.push_str(trimmed);
        out.push('\n');
        last_blank = false;
    }
    out.push_str(rest);
    out
}

#[allow(clippy::too_many_arguments)]
fn wrap_page(
    body: &str,
    title: &str,
    theme_cfg: &ThemeConfig,
    keymap: &Keymap,
    config_errors: &[ConfigError],
    features: Features,
    toc: &TocConfig,
    codemap: &CodemapConfig,
    code: &CodeConfig,
) -> String {
    let title_esc = html_escape(title);
    let theme_css = emit_theme_blocks(theme_cfg);
    let mode_class = resolve_mode(theme_cfg.mode).class();
    let initial_bg = initial_bg_hex(theme_cfg);
    let keymap_js = keymap_json(keymap);
    let banner_html = render_config_banner(config_errors);
    let body_style = if config_errors.is_empty() {
        String::new()
    } else {
        " style=\"padding-top: 48px\"".to_string()
    };
    let head_extras = build_head_extras(features);
    let lib_scripts = build_lib_scripts(features);
    let toc_pos = toc.position.as_kebab();
    let toc_levels = toc.levels;
    let codemap_enabled = if codemap.enabled { "true" } else { "false" };
    let tab_width = code.tab_width;
    let toc_pos_class = format!("mdv-toc--{toc_pos}");
    let toc_aside = format!(
        "<aside id=\"mdv-toc\" class=\"mdv-toc {toc_pos_class} mdv-toc--hidden\" aria-hidden=\"true\"><header><span class=\"mdv-toc__title\">Table of Content</span></header><nav id=\"mdv-toc-nav\"></nav></aside>"
    );
    minify_head(format!(
        r##"<!DOCTYPE html>
<html lang="en" class="{mode_class}" style="background:{initial_bg}">
<head>
<meta charset="utf-8">
<title>{title_esc}</title>
<script>(function(){{try{{var s=window.sessionStorage;if(!s)return;var t=s.getItem('mdv:theme');if(t==='light'||t==='dark'){{var h=document.documentElement;h.classList.remove('theme-light','theme-dark');h.classList.add('theme-'+t);h.style.background='';}}}}catch(_){{}}}})()</script>
<script>
(function() {{
    try {{
        const post = (evt) => {{
            if (window.ipc && typeof window.ipc.postMessage === 'function') {{
                try {{ window.ipc.postMessage('profile:' + evt); }} catch (_) {{}}
            }}
        }};
        post('html_first_byte');
        document.addEventListener('DOMContentLoaded', () => post('dom_loaded'));
        window.addEventListener('load', () => post('window_load'));
        window.addEventListener('load', () => requestAnimationFrame(() => post('after_paint')));
    }} catch (_) {{}}
}})();
</script>
{head_extras}<style>
{theme_css}
  html {{ scroll-behavior: smooth; }}
  html, body {{ background: var(--bg); color: var(--fg); }}
  body {{ font-family: "Inter", "Inter Variable", ui-sans-serif, system-ui, -apple-system,
                       "Segoe UI Variable Text", "Segoe UI", Roboto, sans-serif;
          margin: 32px; padding: 0; line-height: 1.7;
          min-width: min-content;
          -webkit-font-smoothing: antialiased; text-rendering: optimizeLegibility; }}
  body {{ font-size: calc(16px * var(--mdv-zoom, 1)); }}
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
                     white-space: pre; tab-size: {tab_width}; -moz-tab-size: {tab_width}; }}
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
  article.mdv li:has(> .mdv-task-icon) {{
    list-style: none;
    margin-left: -1.5em;
  }}
  article.mdv .mdv-task-icon {{
    display: inline-block;
    vertical-align: -0.15em;
    margin-right: 0.35em;
    width: 1em;
    height: 1em;
  }}
  article.mdv .mdv-task-checked {{ color: var(--accent); }}
  article.mdv .mdv-task-unchecked {{ color: var(--muted); }}
  article.mdv .mermaid, article.mdv .plotly-chart, article.mdv .drawio-viewer {{ margin: 24px 0; }}
  /* Cap diagram containers to viewport width so they don't balloon with the
     expanded page width from long content elsewhere on the page. */
  article.mdv .plotly-chart {{ max-width: min(900px, calc(100vw - 64px)); }}
  article.mdv .mermaid, article.mdv .drawio-viewer, article.mdv .mxgraph {{
    max-width: calc(100vw - 64px); overflow-x: auto;
  }}
  article.mdv .mermaid svg {{
    display: block;
    height: calc(var(--mdv-mermaid-height) * var(--mdv-zoom, 1));
    max-width: none !important;
    width: calc(var(--mdv-mermaid-width) * var(--mdv-zoom, 1));
  }}
  html.mdv-zoomed article.mdv img[style*="--mdv-img-width"] {{
    height: calc(var(--mdv-img-height) * var(--mdv-zoom, 1));
    max-width: none;
    width: calc(var(--mdv-img-width) * var(--mdv-zoom, 1));
  }}
  html.mdv-zoomed article.mdv .plotly-chart[style*="--mdv-plotly-width"] {{
    height: calc(var(--mdv-plotly-height) * var(--mdv-zoom, 1));
    max-width: none;
    width: calc(var(--mdv-plotly-width) * var(--mdv-zoom, 1));
  }}
  article.mdv .mxgraph > svg[style*="--mdv-drawio-width"] {{
    display: block;
    height: calc(var(--mdv-drawio-height) * var(--mdv-zoom, 1)) !important;
    max-width: none !important;
    min-height: 0 !important;
    min-width: 0 !important;
    width: calc(var(--mdv-drawio-width) * var(--mdv-zoom, 1)) !important;
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
  .mdv-code .hl-line {{ display: block; }}
  .mdv-code .hl-line--mark {{
    background: var(--mdv-code-hl-bg, rgba(255, 213, 79, 0.12));
    box-shadow: inset 3px 0 0 var(--mdv-accent-mauve, var(--mdv-accent, #cba6f7));
    margin: 0 calc(var(--mdv-code-padding, 1em) * -1);
    padding-left: var(--mdv-code-padding, 1em);
    padding-right: var(--mdv-code-padding, 1em);
  }}
  .mdv-code .mdv-tok, .mdv-code-inline .mdv-tok            {{ color: inherit; }}
  .mdv-code .mdv-tok-comment, .mdv-code-inline .mdv-tok-comment    {{ color: var(--mdv-muted, var(--muted)); font-style: italic; }}
  .mdv-code .mdv-tok-constant, .mdv-code-inline .mdv-tok-constant   {{ color: var(--mdv-accent-peach, var(--accent)); }}
  .mdv-code .mdv-tok-function, .mdv-code-inline .mdv-tok-function   {{ color: var(--mdv-accent-blue, var(--accent)); }}
  .mdv-code .mdv-tok-keyword, .mdv-code-inline .mdv-tok-keyword    {{ color: var(--mdv-accent-mauve, var(--accent)); font-weight: bold; }}
  .mdv-code .mdv-tok-string, .mdv-code-inline .mdv-tok-string     {{ color: var(--mdv-accent-green, var(--accent)); }}
  .mdv-code .mdv-tok-type, .mdv-code-inline .mdv-tok-type       {{ color: var(--mdv-accent-yellow, var(--accent)); }}
  .mdv-toc {{
    background: color-mix(in srgb, var(--bg) 92%, transparent);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 12px 16px;
    box-shadow: 0 4px 18px rgba(0,0,0,.10);
    backdrop-filter: blur(6px);
    z-index: 1001;
    font-size: 0.94em;
    color: var(--fg);
  }}
  .mdv-toc--hidden {{ display: none; }}
  .mdv-toc header {{ display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }}
  .mdv-toc header span {{ font-weight: 600; }}
  .mdv-toc__title {{ color: #cba6f7; }}
  .mdv-toc nav ul {{ list-style: none; padding-left: 0; margin: 0; }}
  .mdv-toc nav ul ul {{ padding-left: 16px; }}
  .mdv-toc nav li {{ margin: 4px 0; }}
  .mdv-toc nav a {{ display: block; color: var(--fg); text-decoration: none; padding: 2px 6px; border-radius: 4px; border-left: 3px solid transparent; }}
  .mdv-toc nav a:hover {{ background: color-mix(in srgb, var(--accent) 12%, transparent); }}
  .mdv-toc__active {{ border-left-color: var(--accent) !important; font-weight: 600; background: color-mix(in srgb, var(--accent) 8%, transparent); }}
  .mdv-toc--floating-right {{ position: fixed; top: 80px; right: 60px; width: 280px; max-height: calc(100vh - 120px); overflow-y: auto; }}
  .mdv-toc--floating-left  {{ position: fixed; top: 80px; left: 60px;  width: 280px; max-height: calc(100vh - 120px); overflow-y: auto; }}
  .mdv-toc--fixed-right    {{ float: right; width: 240px; margin: 0 0 1em 1em; }}
  .mdv-toc--fixed-left     {{ float: left;  width: 240px; margin: 0 1em 1em 0; }}
  .mdv-toc--inline         {{ margin: 1em 0; }}
  .mdv-toc--floating-center {{
    position: fixed;
    top: 50%; left: 50%;
    transform: translate(-50%, -50%);
    width: 360px;
    max-width: 90vw;
    max-height: 70vh;
    overflow-y: auto;
    z-index: 1002;
  }}
  #mdv-minimap {{ position: fixed; top: 16px; right: 16px; width: 140px; bottom: 16px;
                  background: color-mix(in srgb, var(--bg) 92%, transparent);
                  border: 1px solid var(--border); border-radius: 12px;
                  overflow: hidden; z-index: 1000;
                  box-shadow: 0 4px 18px rgba(0,0,0,.10);
                  cursor: ns-resize; transition: opacity .15s ease;
                  backdrop-filter: blur(6px);
                  user-select: none; touch-action: none; }}
  #mdv-minimap.mdv-hidden {{ display: none; }}
  #mdv-minimap-content {{ transform-origin: top left; pointer-events: none;
                          user-select: none; position: absolute; top: 6px; left: 6px; }}
  #mdv-minimap-content article.mdv {{ margin: 0; }}
  #mdv-minimap-viewport {{ position: absolute; left: 0; right: 0;
                           border: 1px solid var(--accent);
                           background: color-mix(in srgb, var(--accent) 18%, transparent);
                           border-radius: 4px; pointer-events: none; }}
  html.mdv-codemap-dragging, html.mdv-codemap-dragging * {{ scroll-behavior: auto !important; }}
  html.mdv-codemap-dragging #mdv-minimap-viewport {{ transition: none !important; }}
  @media (max-width: 760px) {{ #mdv-minimap {{ display: none; }} }}
  #mdv-context-menu {{ position: fixed; z-index: 10000;
                       background: var(--mdv-card-bg, var(--mdv-bg));
                       border: 1px solid var(--mdv-border-subtle, var(--border));
                       border-radius: 8px; padding: 6px 0;
                       box-shadow: 0 4px 18px rgba(0,0,0,.18);
                       min-width: 220px; font-size: 0.9em; display: none;
                       user-select: none; }}
  #mdv-context-menu.mdv-cm--open {{ display: block; }}
  #mdv-context-menu .mdv-cm__item {{ display: flex; align-items: center; gap: 0.6em;
                                     padding: 6px 14px; cursor: pointer;
                                     color: var(--mdv-fg, var(--fg)); }}
  #mdv-context-menu .mdv-cm__item:hover {{ background: color-mix(in srgb,
                                           var(--mdv-accent, var(--accent)) 18%, transparent); }}
  #mdv-context-menu .mdv-cm__item.mdv-cm__item--disabled {{ color: var(--mdv-muted, var(--muted));
                                                            cursor: not-allowed; opacity: 0.6; }}
  #mdv-context-menu .mdv-cm__item.mdv-cm__item--disabled:hover {{ background: transparent; }}
  #mdv-context-menu .mdv-cm__indicator {{ width: 1em; display: inline-block; text-align: center; }}
  #mdv-context-menu .mdv-cm__indicator--active {{ color: var(--mdv-accent-mauve, var(--mdv-accent, var(--accent))); }}
  #mdv-context-menu .mdv-cm__shortcut {{ margin-left: auto; color: var(--mdv-muted, var(--muted));
                                          font-size: 0.85em; padding-left: 1.5em; }}
  #mdv-context-menu .mdv-cm__divider {{ height: 1px; background: var(--mdv-border-subtle, var(--border));
                                         margin: 4px 0; }}
  .mdv-bionic {{ font-weight: 700; }}
  .mdv-code-wrap {{ position: relative; }}
  .mdv-copy {{
    position: absolute;
    top: 0.4rem;
    right: 0.4rem;
    z-index: 1;
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 1px solid var(--mdv-border-subtle, rgba(127, 127, 127, 0.25));
    border-radius: 5px;
    background: var(--mdv-code-bg, rgba(127, 127, 127, 0.08));
    color: var(--mdv-fg, currentColor);
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.12s ease, background 0.12s ease;
  }}
  .mdv-code-wrap:hover .mdv-copy,
  .mdv-copy:focus-visible {{ opacity: 1; }}
  .mdv-copy:hover {{ background: var(--mdv-accent-soft, rgba(127, 127, 127, 0.18)); }}
  .mdv-copy svg {{ width: 14px; height: 14px; display: block; }}
  .mdv-copy.mdv-copy--ok {{ color: var(--mdv-accent, #4caf50); }}
  #mdv-config-banner {{
    position: fixed; top: 0; left: 0; right: 0; z-index: 9999;
    background: color-mix(in srgb, var(--mdv-accent-yellow, #f9e2af) 92%, transparent);
    color: #1e1e2e;
    border-bottom: 1px solid var(--mdv-accent-yellow, #f9e2af);
    padding: 8px 16px;
    font-size: 13px;
    display: flex; align-items: center; gap: 12px;
    box-shadow: 0 2px 8px rgba(0,0,0,.12);
  }}
  #mdv-config-banner .mdv-config-banner__icon {{ font-size: 16px; flex-shrink: 0; }}
  #mdv-config-banner .mdv-config-banner__text {{ flex: 1; min-width: 0; }}
  #mdv-config-banner .mdv-config-banner__more,
  #mdv-config-banner .mdv-config-banner__close {{
    background: transparent; border: 1px solid rgba(0,0,0,0.18);
    color: #1e1e2e; padding: 3px 8px; border-radius: 4px;
    font-size: 12px; cursor: pointer;
  }}
  #mdv-config-banner .mdv-config-banner__more:hover,
  #mdv-config-banner .mdv-config-banner__close:hover {{ background: rgba(0,0,0,0.06); }}
  #mdv-config-banner .mdv-config-banner__list {{
    list-style: none; margin: 0; padding: 8px 0 0;
    font-family: var(--mdv-font-mono, ui-monospace, monospace);
    font-size: 12px;
    width: 100%;
    border-top: 1px solid rgba(0,0,0,0.1); margin-top: 6px;
  }}
  #mdv-config-banner.mdv-config-banner--expanded {{ flex-wrap: wrap; }}
  #mdv-config-banner.mdv-config-banner--hidden {{ display: none; }}
  body:has(#mdv-config-banner:not(.mdv-config-banner--hidden)) {{
    padding-top: 48px;
  }}
  .mdv-help-overlay {{
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: none;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    backdrop-filter: blur(2px);
  }}
  .mdv-help-overlay.mdv-help-open {{ display: flex; }}
  .mdv-help-panel {{
    background: var(--mdv-bg, #fff);
    color: var(--mdv-fg, #000);
    border: 1px solid var(--mdv-border-subtle, rgba(127,127,127,0.25));
    border-radius: 12px;
    box-shadow: 0 18px 48px rgba(0,0,0,0.35);
    min-width: 360px;
    max-width: 540px;
    width: 90vw;
    max-height: 80vh;
    padding: 20px 22px;
    font: 14px/1.5 var(--mdv-font-body, system-ui);
    outline: none;
    overflow-y: auto;
  }}
  .mdv-help-title {{
    margin: 0 0 12px;
    font-size: 16px;
    font-weight: 600;
  }}
  .mdv-help-row {{
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    padding: 6px 0;
    border-top: 1px solid var(--mdv-border-subtle, rgba(127,127,127,0.12));
  }}
  .mdv-help-row:first-of-type {{ border-top: 0; }}
  .mdv-help-action {{ font-weight: 500; }}
  .mdv-help-desc {{ color: var(--mdv-muted, #888); font-size: 13px; }}
  .mdv-help-binding {{ display: inline-flex; align-items: center; gap: 4px; flex-shrink: 0; }}
  .mdv-help-key {{
    background: var(--mdv-code-bg, rgba(127,127,127,0.12));
    border: 1px solid var(--mdv-border-subtle, rgba(127,127,127,0.25));
    border-radius: 5px;
    padding: 1px 7px;
    font-family: var(--mdv-font-mono, ui-monospace, monospace);
    font-size: 12px;
  }}
  .mdv-help-plus {{ color: var(--mdv-muted, #888); font-size: 12px; }}
  .mdv-help-unbound {{ color: var(--mdv-muted, #888); }}
  .mdv-help-footer {{ margin-top: 14px; font-size: 12px; color: var(--mdv-muted, #888); }}
</style>
</head>
<body{body_style}>
{banner_html}{toc_aside}<div id="mdv-context-menu" role="menu" aria-hidden="true"></div>
<article class="mdv">{body}</article>
{lib_scripts}
<script>
  (() => {{
    const listeners = window.__mdv_listeners || Object.create(null);
    window.__mdv_listeners = listeners;
    window.__mdv_on = (name, fn) => {{
      if (typeof fn !== 'function') return () => {{}};
      const bucket = listeners[name] || (listeners[name] = new Set());
      bucket.add(fn);
      return () => bucket.delete(fn);
    }};
    window.__mdv_emit = (name, detail) => {{
      (listeners[name] || []).forEach(fn => {{
        try {{ fn(detail); }} catch (err) {{ console.warn('mdv event:', name, err); }}
      }});
    }};
    const tokenNames = [
      '--mdv-accent', '--mdv-accent-blue', '--mdv-accent-green', '--mdv-accent-mauve',
      '--mdv-accent-peach', '--mdv-accent-yellow', '--mdv-bg', '--mdv-border-subtle',
      '--mdv-code-bg', '--mdv-fg', '--mdv-link', '--mdv-muted',
    ];
    window.__mdv_colorscheme_detail = (mode) => {{
      const html = document.documentElement;
      const resolvedMode = mode || (html.classList.contains('theme-dark') ? 'dark' : 'light');
      const styles = getComputedStyle(html);
      const colors = {{}};
      tokenNames.forEach(name => {{ colors[name] = styles.getPropertyValue(name).trim(); }});
      return {{ mode: resolvedMode, colors }};
    }};
  }})();
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
    const __mdvTheme = () => window.__mdv_colorscheme_detail();
    const __mdvColor = (detail, name, fallback) => detail.colors[name] || fallback;
    const __mdvZoomLevel = () => parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--mdv-zoom')) || 1;
    const __mdvCaptureBase = (el, prefix) => {{
      const scaled = el.style.getPropertyValue('--' + prefix + '-width') ? __mdvZoomLevel() : 1;
      const rect = el.getBoundingClientRect();
      const w = rect.width / scaled, h = rect.height / scaled;
      if (w > 0 && h > 0) {{
        el.style.setProperty('--' + prefix + '-width', w + 'px');
        el.style.setProperty('--' + prefix + '-height', h + 'px');
      }}
    }};
    const __mdvMermaidVars = (detail) => ({{
      primaryColor: __mdvColor(detail, '--mdv-code-bg', '#ccd0da'),
      primaryTextColor: __mdvColor(detail, '--mdv-fg', '#4c4f69'),
      primaryBorderColor: __mdvColor(detail, '--mdv-accent', '#8839ef'),
      lineColor: __mdvColor(detail, '--mdv-muted', '#6c6f85'),
      secondaryColor: __mdvColor(detail, '--mdv-accent-peach', '#fe640b'),
      tertiaryColor: __mdvColor(detail, '--mdv-accent-blue', '#1e66f5'),
      background: __mdvColor(detail, '--mdv-bg', '#eff1f5'),
      mainBkg: __mdvColor(detail, '--mdv-code-bg', '#ccd0da'),
      secondBkg: __mdvColor(detail, '--mdv-border-subtle', '#bcc0cc'),
      tertiaryBkg: __mdvColor(detail, '--mdv-muted', '#acb0be'),
      nodeBorder: __mdvColor(detail, '--mdv-accent', '#8839ef'),
      clusterBkg: __mdvColor(detail, '--mdv-code-bg', '#ccd0da'),
      clusterBorder: __mdvColor(detail, '--mdv-accent', '#8839ef'),
      defaultLinkColor: __mdvColor(detail, '--mdv-muted', '#6c6f85'),
      titleColor: __mdvColor(detail, '--mdv-fg', '#4c4f69'),
      edgeLabelBackground: __mdvColor(detail, '--mdv-code-bg', '#ccd0da'),
      actorBkg: __mdvColor(detail, '--mdv-code-bg', '#ccd0da'),
      actorBorder: __mdvColor(detail, '--mdv-accent', '#8839ef'),
      actorTextColor: __mdvColor(detail, '--mdv-fg', '#4c4f69'),
    }});
    const __mdvRenderMermaid = (detail) => {{
      if (!window.mermaid) return;
      const nodes = Array.from(document.querySelectorAll('.mermaid'));
      nodes.forEach(el => {{
        if (!el.dataset.mdvSource) el.dataset.mdvSource = el.textContent || '';
        el.removeAttribute('data-processed');
        el.innerHTML = '';
        el.textContent = el.dataset.mdvSource;
      }});
      mermaid.initialize({{ startOnLoad: false, theme: 'base', themeVariables: __mdvMermaidVars(detail) }});
      mermaid.run({{ nodes }}).then(() => {{
        document.querySelectorAll('.mermaid svg').forEach(svg => {{
          const viewBox = svg.viewBox && svg.viewBox.baseVal;
          const rect = svg.getBoundingClientRect();
          const width = viewBox && viewBox.width ? viewBox.width : rect.width;
          const height = viewBox && viewBox.height ? viewBox.height : rect.height;
          if (width > 0) svg.style.setProperty('--mdv-mermaid-width', width + 'px');
          if (height > 0) svg.style.setProperty('--mdv-mermaid-height', height + 'px');
        }});
      }}).catch(e => console.warn('mermaid:', e));
    }};
    const __mdvClone = (value) => JSON.parse(JSON.stringify(value || {{}}));
    const __mdvPlotlyTheme = (detail, spec) => {{
      const data = __mdvClone(spec.data || []);
      const layout = __mdvClone(spec.layout || {{}});
      const fg = __mdvColor(detail, '--mdv-fg', '#4c4f69');
      const muted = __mdvColor(detail, '--mdv-muted', '#6c6f85');
      const grid = __mdvColor(detail, '--mdv-border-subtle', '#bcc0cc');
      layout.paper_bgcolor = __mdvColor(detail, '--mdv-bg', '#eff1f5');
      layout.plot_bgcolor = __mdvColor(detail, '--mdv-code-bg', '#ccd0da');
      layout.font = Object.assign({{}}, layout.font || {{}}, {{ color: fg }});
      ['xaxis', 'yaxis'].forEach(axis => {{
        layout[axis] = Object.assign({{}}, layout[axis] || {{}}, {{
          color: fg,
          gridcolor: grid,
          linecolor: muted,
          tickcolor: muted,
          zerolinecolor: muted,
        }});
      }});
      data.forEach((trace, i) => {{
        if (!trace || trace.marker) return;
        const accents = ['--mdv-accent-blue', '--mdv-accent-green', '--mdv-accent-mauve', '--mdv-accent-peach'];
        trace.marker = {{ color: __mdvColor(detail, accents[i % accents.length], __mdvColor(detail, '--mdv-accent', '#8839ef')) }};
      }});
      return {{ data, layout }};
    }};
    const __mdvRenderPlotly = (detail) => {{
      if (!window.Plotly) return;
      document.querySelectorAll('.plotly-chart[data-spec]').forEach(el => {{
        try {{
          if (!el.__mdvSpec) el.__mdvSpec = JSON.parse(el.getAttribute('data-spec'));
          const themed = __mdvPlotlyTheme(detail, el.__mdvSpec);
          if (Array.isArray(el.data)) {{
            const relayout = {{
              paper_bgcolor: themed.layout.paper_bgcolor,
              plot_bgcolor: themed.layout.plot_bgcolor,
              'font.color': themed.layout.font && themed.layout.font.color,
            }};
            ['xaxis', 'yaxis'].forEach(axis => {{
              const a = themed.layout[axis];
              if (!a) return;
              relayout[axis + '.color'] = a.color;
              relayout[axis + '.gridcolor'] = a.gridcolor;
              relayout[axis + '.linecolor'] = a.linecolor;
              relayout[axis + '.tickcolor'] = a.tickcolor;
              relayout[axis + '.zerolinecolor'] = a.zerolinecolor;
            }});
            Plotly.relayout(el, relayout);
            themed.data.forEach((trace, i) => {{
              if (trace && trace.marker && trace.marker.color != null) {{
                Plotly.restyle(el, {{ 'marker.color': trace.marker.color }}, [i]);
              }}
            }});
          }} else {{
            Plotly.newPlot(el, themed.data, themed.layout, {{ responsive: true, displaylogo: false }})
              .then(() => __mdvCaptureBase(el, 'mdv-plotly'))
              .catch(e => console.warn('plotly:', e));
          }}
        }} catch (e) {{ console.warn('plotly:', e); }}
      }});
    }};
    const __mdvDecodeB64 = (b64) => decodeURIComponent(atob(b64).split('').map(c =>
      '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2)).join(''));
    const __mdvDrawioXml = (xml, detail) => {{
      try {{
        const doc = new DOMParser().parseFromString(xml, 'text/xml');
        doc.querySelectorAll('mxCell[vertex="1"], mxCell[edge="1"]').forEach(cell => {{
          const style = cell.getAttribute('style') || '';
          const parts = style.split(';').filter(part =>
            part && !/^(fillColor|strokeColor|fontColor)=/.test(part));
          if (cell.getAttribute('vertex') === '1') parts.push('fillColor=' + __mdvColor(detail, '--mdv-code-bg', '#ccd0da'));
          parts.push('strokeColor=' + __mdvColor(detail, '--mdv-accent', '#8839ef'));
          parts.push('fontColor=' + __mdvColor(detail, '--mdv-fg', '#4c4f69'));
          cell.setAttribute('style', parts.join(';') + ';');
        }});
        return new XMLSerializer().serializeToString(doc);
      }} catch (_) {{
        return xml;
      }}
    }};
    const __mdvRenderDrawio = (detail) => {{
      document.querySelectorAll('.drawio-viewer[data-xml-b64], .mxgraph[data-mdv-drawio-xml]').forEach(el => {{
        try {{
          const xml = el.getAttribute('data-mdv-drawio-xml') || __mdvDecodeB64(el.getAttribute('data-xml-b64'));
          el.setAttribute('data-mdv-drawio-xml', xml);
          el.className = 'mxgraph';
          el.removeAttribute('data-xml-b64');
          el.innerHTML = '';
          el.setAttribute('data-mxgraph', JSON.stringify({{
            xml: __mdvDrawioXml(xml, detail),
            background: __mdvColor(detail, '--mdv-bg', '#eff1f5'),
            highlight: __mdvColor(detail, '--mdv-accent-blue', '#1e66f5'),
            lightbox: false, nav: true,
            resize: true, toolbar: 'zoom layers tags lightbox',
          }}));
        }} catch (e) {{ console.warn('drawio rewrite:', e); }}
      }});
      if (window.GraphViewer && typeof window.GraphViewer.processElements === 'function') {{
        try {{ window.GraphViewer.processElements('mxgraph'); }}
        catch (e) {{ console.warn('drawio:', e); }}
      }}
      requestAnimationFrame(() => {{
        document.querySelectorAll('.mxgraph > svg').forEach(svg => {{
          __mdvCaptureBase(svg, 'mdv-drawio');
          if (!svg.getAttribute('viewBox')) {{
            const w = parseFloat(svg.style.getPropertyValue('--mdv-drawio-width'));
            const h = parseFloat(svg.style.getPropertyValue('--mdv-drawio-height'));
            if (w > 0 && h > 0) svg.setAttribute('viewBox', '0 0 ' + w + ' ' + h);
          }}
        }});
      }});
    }};
    const __mdvRenderDiagrams = (detail) => {{
      __mdvRenderMermaid(detail);
      __mdvRenderPlotly(detail);
      __mdvRenderDrawio(detail);
    }};
    __mdvRenderDiagrams(__mdvTheme());
    window.__mdv_on('colorscheme', __mdvRenderDiagrams);
    const __mdvInitImages = () => {{
      document.querySelectorAll('article.mdv img').forEach(img => {{
        const capture = () => {{ if (img.naturalWidth > 0) __mdvCaptureBase(img, 'mdv-img'); }};
        if (img.complete && img.naturalWidth > 0) capture();
        else img.addEventListener('load', capture, {{ once: true }});
      }});
    }};
    __mdvInitImages();
    let __mdvPlotlyResizeRaf = 0;
    const __mdvResizePlotly = () => {{
      if (__mdvPlotlyResizeRaf) return;
      __mdvPlotlyResizeRaf = requestAnimationFrame(() => {{
        __mdvPlotlyResizeRaf = 0;
        if (!window.Plotly || !Plotly.Plots || typeof Plotly.Plots.resize !== 'function') return;
        document.querySelectorAll('.plotly-chart').forEach(el => {{
          if (Array.isArray(el.data)) {{
            try {{ Plotly.Plots.resize(el); }} catch (e) {{ console.warn('plotly resize:', e); }}
          }}
        }});
      }});
    }};
    window.__mdv_on('zoom', __mdvResizePlotly);

    // Codemap is hidden on load by hardcoded default (tasks.md #12/#13). The DOM
    // is NOT mounted until the user first toggles it on; once mounted, subsequent
    // toggles flip a `mdv-hidden` class (display:none) rather than unmounting,
    // so we never pay the construction cost twice.
    let __mdvMinimap = null;
    const __mdvMountMinimap = () => {{
      if (__mdvMinimap) return __mdvMinimap;
      const article = document.querySelector('article.mdv');
      if (!article) return null;

      const minimap = document.createElement('div');
      minimap.id = 'mdv-minimap';
      minimap.classList.add('mdv-hidden');

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

      let isDragging = false;
      let pendingY = 0;
      let rafId = 0;
      const scroller = document.scrollingElement || document.documentElement;
      const applyDragScroll = () => {{
        const rect = minimap.getBoundingClientRect();
        const y = pendingY - rect.top - PAD;
        const targetY = y / scale - window.innerHeight / 2;
        const maxY = scroller.scrollHeight - window.innerHeight;
        const top = Math.max(0, Math.min(maxY, targetY));
        scroller.scrollTop = top;
        viewport.style.top = (PAD + top * scale) + 'px';
      }};
      const dragFrame = () => {{
        if (!isDragging) return;
        applyDragScroll();
        rafId = requestAnimationFrame(dragFrame);
      }};

      window.addEventListener('scroll', () => {{ if (__mdvMinimap && !__mdvMinimap.hidden && !isDragging) update(); }}, {{ passive: true }});
      window.addEventListener('resize', () => {{ if (__mdvMinimap && !__mdvMinimap.hidden) update(); }});

      minimap.addEventListener('pointerdown', (e) => {{
        if (e.button !== 0) return;
        isDragging = true;
        pendingY = e.clientY;
        document.documentElement.classList.add('mdv-codemap-dragging');
        try {{ minimap.setPointerCapture(e.pointerId); }} catch (_) {{}}
        e.preventDefault();
        applyDragScroll();
        rafId = requestAnimationFrame(dragFrame);
      }});
      minimap.addEventListener('pointermove', (e) => {{
        if (!isDragging) return;
        pendingY = e.clientY;
      }});
      const endDrag = (e) => {{
        if (!isDragging) return;
        isDragging = false;
        if (rafId) {{ cancelAnimationFrame(rafId); rafId = 0; }}
        document.documentElement.classList.remove('mdv-codemap-dragging');
        try {{ minimap.releasePointerCapture(e.pointerId); }} catch (_) {{}}
      }};
      minimap.addEventListener('pointerup', endDrag);
      minimap.addEventListener('pointercancel', endDrag);

      __mdvMinimap = {{ el: minimap, update, hidden: true }};
      return __mdvMinimap;
    }};

    window.__mdvToggleCodemap = () => {{
      // First toggle: lazy-mount and show. Subsequent toggles: flip visibility
      // on the already-mounted element.
      const mm = __mdvMountMinimap();
      if (!mm) return;
      mm.hidden = !mm.hidden;
      mm.el.classList.toggle('mdv-hidden', mm.hidden);
      if (!mm.hidden) mm.update();
    }};
    window.__mdvCodemapVisible = () =>
      !!(__mdvMinimap && !__mdvMinimap.hidden);
  }});
</script>
<script id="mdv-keymap" type="application/json">{keymap_js}</script>
<script>
  (() => {{
    const raw = document.getElementById('mdv-keymap');
    let bindings = {{}};
    try {{ bindings = JSON.parse((raw && raw.textContent) || '{{}}'); }} catch (e) {{ console.warn('mdv-keymap parse:', e); }}
    // Modifier matching is exact, Shift included, with one necessary exception:
    // for non-letter character keys (like '?' or ':') bound without an explicit
    // Shift, the Shift flag is ignored, because producing that glyph on many
    // layouts inherently requires Shift. Letter chords keep exact Shift matching
    // so distinct chords never collide (Ctrl+B must not match Ctrl+Shift+B).
    const matchBinding = (b, e) => {{
      if (!b) return false;
      if (b.ctrl !== !!e.ctrlKey) return false;
      if (b.alt !== !!e.altKey) return false;
      if (b.super !== !!e.metaKey) return false;
      if (b.kind === 'char') {{
        const isLetter = b.key >= 'a' && b.key <= 'z';
        if ((isLetter || b.shift) && b.shift !== !!e.shiftKey) return false;
        return (e.key || '').toLowerCase() === b.key;
      }}
      if (b.shift !== !!e.shiftKey) return false;
      return e.key === b.key;
    }};
    window.__mdvToggleTheme = () => {{
      const html = document.documentElement;
      const isDark = html.classList.contains('theme-dark');
      html.classList.remove('theme-light', 'theme-dark');
      const newMode = isDark ? 'light' : 'dark';
      html.classList.add('theme-' + newMode);
      html.style.background = '';
      try {{ window.sessionStorage.setItem('mdv:theme', newMode); }} catch (_) {{}}
      if (window.ipc && typeof window.ipc.postMessage === 'function') {{
        try {{ window.ipc.postMessage('theme-' + newMode); }} catch (err) {{ console.warn('ipc:', err); }}
      }}
      if (typeof window.__mdv_emit === 'function' && typeof window.__mdv_colorscheme_detail === 'function') {{
        window.__mdv_emit('colorscheme', window.__mdv_colorscheme_detail(newMode));
      }}
    }};
    window.__mdv_config = Object.assign(window.__mdv_config || {{}}, {{
      keymap: bindings,
      toc: {{ position: "{toc_pos}", levels: {toc_levels} }},
      codemap: {{ enabled: {codemap_enabled} }}
    }});
    // Ordered dispatch table: one keydown selects at most one action. A ready()
    // that fails falls through to the next candidate rather than swallowing the
    // event, so an absent optional toggle cannot mask a later binding.
    const mdvActions = [
      ['quit',
        () => !!(window.ipc && typeof window.ipc.postMessage === 'function'),
        () => {{ try {{ window.ipc.postMessage('quit'); }} catch (err) {{ console.warn('ipc:', err); }} }}],
      ['toggle-theme',
        () => typeof window.__mdvToggleTheme === 'function',
        () => window.__mdvToggleTheme()],
      ['toggle-bionic',
        () => typeof window.__mdvToggleBionic === 'function',
        () => window.__mdvToggleBionic()],
      ['toggle-codemap',
        () => typeof window.__mdvToggleCodemap === 'function',
        () => window.__mdvToggleCodemap()],
      ['toggle-toc',
        () => typeof window.__mdvToggleToc === 'function',
        () => window.__mdvToggleToc()],
    ];
    document.addEventListener('keydown', (e) => {{
      const tag = (document.activeElement && document.activeElement.tagName) || '';
      if (tag === 'INPUT' || tag === 'TEXTAREA') return;
      for (const [name, ready, run] of mdvActions) {{
        if (!matchBinding(bindings[name], e)) continue;
        if (!ready()) continue;
        e.preventDefault();
        run();
        return;
      }}
    }});
  }})();
</script>
<script>
  (function setupZoom() {{
    const html = document.documentElement;
    let zoom = 1.0;
    const MIN = 0.5, MAX = 3.0, STEP = 0.1;
    const setZoom = (next) => {{
      const clamped = Math.max(MIN, Math.min(MAX, Number(next) || 1));
      if (Math.abs(clamped - zoom) < 0.001) return;
      zoom = clamped;
      html.style.setProperty('--mdv-zoom', zoom.toFixed(1));
      html.classList.toggle('mdv-zoomed', Math.abs(zoom - 1) > 0.001);
      if (typeof window.__mdv_emit === 'function') window.__mdv_emit('zoom', {{ scale: zoom }});
    }};
    window.addEventListener('wheel', (e) => {{
      if (!e.ctrlKey) return;
      e.preventDefault();
      setZoom(zoom + (e.deltaY < 0 ? STEP : -STEP));
    }}, {{ passive: false }});
  }})();
</script>
<script>
  (function setupBionic() {{
    let enabled = false;
    const SKIP_SELECTORS = 'a, code, em, kbd, mark, pre, strong, sub, sup, [data-math-style], .katex';

    function shouldSkip(el) {{
      while (el && el.nodeType === 1) {{
        if (el.matches && el.matches(SKIP_SELECTORS)) return true;
        el = el.parentElement;
      }}
      return false;
    }}

    function bionicWord(word) {{
      if (word.length <= 3) {{
        return '<b class="mdv-bionic">' + word[0] + '</b>' + word.slice(1);
      }}
      const half = Math.ceil(word.length / 2);
      return '<b class="mdv-bionic">' + word.slice(0, half) + '</b>' + word.slice(half);
    }}

    function transformTextNode(textNode) {{
      const parent = textNode.parentElement;
      if (!parent || shouldSkip(parent)) return;
      const text = textNode.nodeValue;
      if (!/[A-Za-z]/.test(text)) return;
      const html = text.replace(/[A-Za-z]+/g, function (w) {{ return bionicWord(w); }});
      const span = document.createElement('span');
      span.dataset.mdvBionic = '1';
      span.innerHTML = html;
      parent.replaceChild(span, textNode);
    }}

    function applyBionic() {{
      const roots = document.querySelectorAll('article.mdv p, article p');
      const nodes = [];
      roots.forEach(function (p) {{
        const walker = document.createTreeWalker(
          p, NodeFilter.SHOW_TEXT,
          {{ acceptNode: function (n) {{
            if (!n.nodeValue.trim()) return NodeFilter.FILTER_REJECT;
            if (shouldSkip(n.parentElement)) return NodeFilter.FILTER_REJECT;
            return NodeFilter.FILTER_ACCEPT;
          }} }}
        );
        let m; while ((m = walker.nextNode())) nodes.push(m);
      }});
      nodes.forEach(transformTextNode);
    }}

    function removeBionic() {{
      document.querySelectorAll('span[data-mdv-bionic="1"]').forEach(function (span) {{
        const txt = document.createTextNode(span.textContent);
        span.parentNode.replaceChild(txt, span);
      }});
    }}

    window.__mdvToggleBionic = function () {{
      enabled = !enabled;
      if (enabled) applyBionic();
      else removeBionic();
      try {{
        if (enabled) window.sessionStorage.setItem('mdv:bionic', '1');
        else window.sessionStorage.removeItem('mdv:bionic');
      }} catch (_) {{}}
    }};
    try {{
      if (window.sessionStorage && window.sessionStorage.getItem('mdv:bionic') === '1') {{
        window.__mdvToggleBionic();
      }}
    }} catch (_) {{}}
  }})();
</script>
<script>
  (function setupToc() {{
    const aside = document.getElementById('mdv-toc');
    if (!aside) return;
    const nav = document.getElementById('mdv-toc-nav');
    const article = document.querySelector('article.mdv');
    if (!article || !nav) {{ aside.remove(); return; }}

    const cfg = (window.__mdv_config && window.__mdv_config.toc) || {{ position: "floating-right", levels: 3 }};
    const maxLevel = Math.max(1, Math.min(6, cfg.levels || 3));

    function slugify(text) {{
      return text.toLowerCase()
        .replace(/[^a-z0-9\s-]/g, '')
        .trim()
        .replace(/\s+/g, '-');
    }}

    const headings = Array.from(article.querySelectorAll('h1, h2, h3, h4, h5, h6'))
      .filter(h => parseInt(h.tagName[1], 10) <= maxLevel);

    if (headings.length < 2) {{ aside.remove(); return; }}

    // Indent is computed relative to the shallowest heading actually present
    // in the document. A doc whose top-level heading is H2 should not waste
    // a leading indent step on the missing H1.
    let shallowest = 6;
    for (const h of headings) {{
      const l = parseInt(h.tagName[1], 10);
      if (l < shallowest) shallowest = l;
    }}

    const slugCounts = {{}};
    headings.forEach(h => {{
      if (!h.id) {{
        let base = slugify(h.textContent || '');
        if (!base) base = 'section';
        const count = slugCounts[base] || 0;
        h.id = count === 0 ? base : `${{base}}-${{count}}`;
        slugCounts[base] = count + 1;
      }}
    }});

    const INDENT_PX = 16;
    const root = document.createElement('ul');
    headings.forEach(h => {{
      const lvl = parseInt(h.tagName[1], 10);
      const indent = lvl - shallowest;
      const li = document.createElement('li');
      li.dataset.tocLevel = String(lvl);
      li.dataset.tocIndent = String(indent);
      if (indent > 0) {{
        li.style.paddingLeft = (indent * INDENT_PX) + 'px';
      }}
      const a = document.createElement('a');
      a.href = '#' + h.id;
      a.dataset.slug = h.id;
      a.textContent = h.textContent || '';
      a.addEventListener('click', (e) => {{
        e.preventDefault();
        const target = document.getElementById(h.id);
        if (target) {{
          // Reset the inline axis: scrolling back to x=0 guarantees the left margin is
          // visible, and avoids the browser's default 'nearest' inline scrolling shifting
          // scrollX when the document overflows horizontally.
          const y = target.getBoundingClientRect().top + window.scrollY;
          window.scrollTo({{ left: 0, top: y, behavior: 'smooth' }});
        }}
        history.replaceState(null, '', '#' + h.id);
      }});
      li.appendChild(a);
      root.appendChild(li);
    }});
    nav.appendChild(root);

    const linkBySlug = {{}};
    nav.querySelectorAll('a[data-slug]').forEach(a => {{ linkBySlug[a.dataset.slug] = a; }});
    let activeLink = null;
    const observer = new IntersectionObserver((entries) => {{
      entries.forEach(entry => {{
        const link = linkBySlug[entry.target.id];
        if (!link) return;
        if (entry.isIntersecting) {{
          if (activeLink) activeLink.classList.remove('mdv-toc__active');
          link.classList.add('mdv-toc__active');
          activeLink = link;
        }}
      }});
    }}, {{ rootMargin: '-10% 0px -80% 0px' }});
    headings.forEach(h => observer.observe(h));

    window.__mdvToggleToc = function () {{
      const wasHidden = aside.classList.contains('mdv-toc--hidden');
      aside.classList.toggle('mdv-toc--hidden');
      aside.setAttribute('aria-hidden', wasHidden ? 'false' : 'true');
    }};

    document.addEventListener('keydown', (e) => {{
      if (e.key === 'Escape' && !aside.classList.contains('mdv-toc--hidden')) {{
        window.__mdvToggleToc();
      }}
    }});
  }})();
</script>
<script>
  (function setupContextMenu() {{
    const menu = document.getElementById('mdv-context-menu');
    if (!menu) return;
    let ctxTarget = null;

    function tocVisible() {{
      const el = document.querySelector('#mdv-toc');
      if (!el) return false;
      return !el.classList.contains('mdv-toc--hidden')
          && !el.classList.contains('mdv-hidden')
          && getComputedStyle(el).display !== 'none';
    }}
    function codemapVisible() {{
      if (typeof window.__mdvCodemapVisible === 'function') return window.__mdvCodemapVisible();
      const el = document.querySelector('#mdv-minimap');
      return !!el && !el.classList.contains('mdv-hidden') && getComputedStyle(el).display !== 'none';
    }}
    function themeIsDark() {{ return document.documentElement.classList.contains('theme-dark'); }}
    function bionicActive() {{
      return !!document.querySelector('span[data-mdv-bionic="1"]');
    }}

    function formatBinding(b) {{
      if (!b) return '';
      const parts = [];
      if (b.ctrl)  parts.push('Ctrl');
      if (b.alt)   parts.push('Alt');
      if (b.shift) parts.push('Shift');
      if (b.super) parts.push('Super');
      let k = b.key || '';
      if (b.kind === 'char') k = k.length === 1 ? k.toUpperCase() : k;
      if (k === ' ') k = 'Space';
      parts.push(k);
      return parts.join('+');
    }}
    function shortcutFor(action) {{
      const cfg = (window.__mdv_config && window.__mdv_config.keymap) || {{}};
      const b = cfg[action];
      if (!b) return '';
      if (typeof b === 'string') return b;
      return formatBinding(b);
    }}

    function buildItems() {{
      const sel = window.getSelection();
      const hasSel = !!(sel && !sel.isCollapsed && sel.toString().length > 0);
      const onLink  = ctxTarget && ctxTarget.closest && ctxTarget.closest('a[href]');
      const onImage = ctxTarget && ctxTarget.closest && ctxTarget.closest('img');
      const items = [];
      items.push({{
        label: 'Copy', shortcut: 'Ctrl+C', disabled: !hasSel,
        on: () => {{ try {{ navigator.clipboard.writeText(sel.toString()); }} catch (_) {{}} }},
      }});
      if (onLink)  items.push({{ label: 'Copy link address',  on: () => navigator.clipboard.writeText(onLink.href) }});
      if (onImage) items.push({{ label: 'Copy image address', on: () => navigator.clipboard.writeText(onImage.src) }});
      if (onLink)  items.push({{ label: 'Open link in browser',
                                  on: () => window.ipc && window.ipc.postMessage('open-link ' + onLink.href) }});
      items.push({{ divider: true }});
      items.push({{
        label: tocVisible() ? 'Hide TOC' : 'Show TOC',
        shortcut: shortcutFor('toggle-toc'),
        indicator: tocVisible() ? '●' : '○',
        on: () => window.__mdvToggleToc && window.__mdvToggleToc(),
      }});
      items.push({{
        label: codemapVisible() ? 'Hide Codemap' : 'Show Codemap',
        shortcut: shortcutFor('toggle-codemap'),
        indicator: codemapVisible() ? '●' : '○',
        on: () => window.__mdvToggleCodemap && window.__mdvToggleCodemap(),
      }});
      items.push({{
        label: 'Switch to ' + (themeIsDark() ? 'Light' : 'Dark') + ' theme',
        shortcut: shortcutFor('toggle-theme'),
        on: () => window.__mdvToggleTheme && window.__mdvToggleTheme(),
      }});
      items.push({{
        label: bionicActive() ? 'Disable Bionic Reading' : 'Enable Bionic Reading',
        shortcut: shortcutFor('toggle-bionic'),
        indicator: bionicActive() ? '●' : '○',
        on: () => window.__mdvToggleBionic && window.__mdvToggleBionic(),
      }});
      items.push({{ divider: true }});
      items.push({{ label: 'Reload', on: () => window.ipc && window.ipc.postMessage('reload') }});
      items.push({{ label: 'Quit', shortcut: shortcutFor('quit'),
                    on: () => window.ipc && window.ipc.postMessage('quit') }});
      return items;
    }}

    function renderMenu(items) {{
      menu.innerHTML = '';
      for (const it of items) {{
        if (it.divider) {{
          const d = document.createElement('div');
          d.className = 'mdv-cm__divider';
          menu.appendChild(d);
          continue;
        }}
        const el = document.createElement('div');
        el.className = 'mdv-cm__item' + (it.disabled ? ' mdv-cm__item--disabled' : '');
        el.setAttribute('role', 'menuitem');
        const ind = document.createElement('span');
        ind.className = 'mdv-cm__indicator' + (it.indicator === '●' ? ' mdv-cm__indicator--active' : '');
        ind.textContent = it.indicator || '';
        const label = document.createElement('span');
        label.className = 'mdv-cm__label';
        label.textContent = it.label;
        el.appendChild(ind); el.appendChild(label);
        if (it.shortcut) {{
          const sc = document.createElement('span');
          sc.className = 'mdv-cm__shortcut';
          sc.textContent = it.shortcut;
          el.appendChild(sc);
        }}
        if (!it.disabled) {{
          el.addEventListener('click', () => {{ closeMenu(); try {{ it.on(); }} catch (_) {{}} }});
        }}
        menu.appendChild(el);
      }}
    }}

    function openMenu(x, y) {{
      renderMenu(buildItems());
      menu.classList.add('mdv-cm--open');
      menu.setAttribute('aria-hidden', 'false');
      menu.style.left = '0px'; menu.style.top = '0px';
      const r = menu.getBoundingClientRect();
      const vw = window.innerWidth, vh = window.innerHeight;
      const left = Math.max(4, Math.min(x, vw - r.width - 8));
      const top  = Math.max(4, Math.min(y, vh - r.height - 8));
      menu.style.left = left + 'px';
      menu.style.top  = top + 'px';
    }}
    function closeMenu() {{
      menu.classList.remove('mdv-cm--open');
      menu.setAttribute('aria-hidden', 'true');
    }}

    document.addEventListener('contextmenu', (e) => {{
      e.preventDefault();
      ctxTarget = e.target;
      openMenu(e.clientX, e.clientY);
    }});
    document.addEventListener('mousedown', (e) => {{ if (!menu.contains(e.target)) closeMenu(); }});
    document.addEventListener('keydown', (e) => {{ if (e.key === 'Escape') closeMenu(); }});
    window.addEventListener('blur', closeMenu);
    window.addEventListener('resize', closeMenu);
    window.addEventListener('scroll', closeMenu, {{ passive: true }});
  }})();
</script>
<script>
  (function setupConfigBanner() {{
    const banner = document.getElementById('mdv-config-banner');
    if (!banner) return;
    const more = banner.querySelector('.mdv-config-banner__more');
    const close = banner.querySelector('.mdv-config-banner__close');
    const list = banner.querySelector('.mdv-config-banner__list');
    if (more && list) {{
      more.addEventListener('click', () => {{
        const showing = !list.hasAttribute('hidden');
        if (showing) {{
          list.setAttribute('hidden', '');
          banner.classList.remove('mdv-config-banner--expanded');
          more.textContent = 'Show all';
        }} else {{
          list.removeAttribute('hidden');
          banner.classList.add('mdv-config-banner--expanded');
          more.textContent = 'Hide all';
        }}
      }});
    }}
    function dismiss() {{ banner.classList.add('mdv-config-banner--hidden'); }}
    if (close) close.addEventListener('click', dismiss);
    document.addEventListener('keydown', (e) => {{
      if (e.key === 'Escape' && !banner.classList.contains('mdv-config-banner--hidden')) {{
        dismiss();
      }}
    }});
  }})();
</script>
<script>
  (function setupCopyButtons() {{
    const clipboard = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>`;
    const check = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>`;
    document.querySelectorAll('pre.mdv-code').forEach(pre => {{
      if (pre.parentElement && pre.parentElement.classList.contains('mdv-code-wrap')) return;
      const code = pre.querySelector(':scope > code');
      if (!code) return;
      const wrap = document.createElement('div');
      wrap.className = 'mdv-code-wrap';
      pre.parentNode.insertBefore(wrap, pre);
      wrap.appendChild(pre);
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'mdv-copy';
      btn.setAttribute('aria-label', 'Copy code');
      btn.innerHTML = clipboard;
      btn.addEventListener('click', async () => {{
        const text = code.innerText;
        const show = (ok) => {{
          btn.innerHTML = ok ? check : clipboard;
          btn.classList.toggle('mdv-copy--ok', ok);
          btn.setAttribute('aria-label', ok ? 'Copied' : 'Copy code');
          setTimeout(() => {{
            btn.innerHTML = clipboard;
            btn.classList.remove('mdv-copy--ok');
            btn.setAttribute('aria-label', 'Copy code');
          }}, 1500);
        }};
        try {{
          if (navigator.clipboard && navigator.clipboard.writeText) {{
            await navigator.clipboard.writeText(text);
            show(true);
          }} else {{
            const ta = document.createElement('textarea');
            ta.value = text;
            ta.style.position = 'fixed';
            ta.style.opacity = '0';
            document.body.appendChild(ta);
            ta.select();
            const ok = document.execCommand('copy');
            document.body.removeChild(ta);
            show(ok);
          }}
        }} catch (_) {{
          show(false);
        }}
      }});
      wrap.appendChild(btn);
    }});
  }})();
</script>
<script>
  (function setupHelp() {{
    if (document.getElementById('mdv-help-overlay')) return;
    const ACTIONS = [
      {{ name: 'quit',           desc: 'Close the mdview window' }},
      {{ name: 'toggle-bionic',  desc: 'Toggle bionic-reading transform' }},
      {{ name: 'toggle-codemap', desc: 'Show / hide the right-edge minimap' }},
      {{ name: 'toggle-theme',   desc: 'Flip between light and dark themes' }},
      {{ name: 'toggle-toc',     desc: 'Show / hide the floating table of contents' }},
    ];
    function formatBinding(b) {{
      if (!b) return null;
      const parts = [];
      if (b.ctrl)  parts.push('Ctrl');
      if (b.shift) parts.push('Shift');
      if (b.alt)   parts.push('Alt');
      if (b.super) parts.push('Super');
      let k = b.key || '';
      if (b.kind === 'char') k = k.length === 1 ? k.toUpperCase() : k;
      if (k === ' ') k = 'Space';
      parts.push(k);
      return parts.join('+');
    }}
    function renderBinding(b) {{
      if (!b) return '<span class="mdv-help-unbound">—</span>';
      const formatted = formatBinding(b);
      if (!formatted) return '<span class="mdv-help-unbound">—</span>';
      const parts = formatted.split('+');
      return '<span class="mdv-help-binding">' + parts.map((p, i) =>
        (i > 0 ? '<span class="mdv-help-plus">+</span>' : '') +
        '<kbd class="mdv-help-key">' + p + '</kbd>'
      ).join('') + '</span>';
    }}
    function buildPanel() {{
      const overlay = document.createElement('div');
      overlay.id = 'mdv-help-overlay';
      overlay.className = 'mdv-help-overlay';
      const panel = document.createElement('div');
      panel.className = 'mdv-help-panel';
      panel.setAttribute('role', 'dialog');
      panel.setAttribute('aria-modal', 'true');
      panel.setAttribute('aria-label', 'Keyboard shortcuts');
      panel.setAttribute('tabindex', '-1');
      const title = document.createElement('h2');
      title.className = 'mdv-help-title';
      title.textContent = 'Keyboard shortcuts';
      panel.appendChild(title);
      const km = (window.__mdv_config && window.__mdv_config.keymap) || {{}};
      for (const action of ACTIONS) {{
        const row = document.createElement('div');
        row.className = 'mdv-help-row';
        const left = document.createElement('span');
        left.innerHTML = '<span class="mdv-help-action">' + action.name + '</span> <span class="mdv-help-desc">' + action.desc + '</span>';
        const right = document.createElement('span');
        right.innerHTML = renderBinding(km[action.name] || null);
        row.appendChild(left);
        row.appendChild(right);
        panel.appendChild(row);
      }}
      const footer = document.createElement('p');
      footer.className = 'mdv-help-footer';
      footer.innerHTML = '<kbd class="mdv-help-key">?</kbd> toggles this panel · <kbd class="mdv-help-key">Esc</kbd> closes it';
      panel.appendChild(footer);
      overlay.appendChild(panel);
      return overlay;
    }}
    let lastFocus = null;
    const overlay = buildPanel();
    document.body.appendChild(overlay);
    const panel = overlay.querySelector('.mdv-help-panel');
    function open() {{
      lastFocus = document.activeElement;
      overlay.classList.add('mdv-help-open');
      panel.focus();
    }}
    function close() {{
      overlay.classList.remove('mdv-help-open');
      if (lastFocus && typeof lastFocus.focus === 'function') lastFocus.focus();
    }}
    function isOpen() {{ return overlay.classList.contains('mdv-help-open'); }}
    overlay.addEventListener('click', (e) => {{ if (e.target === overlay) close(); }});
    document.addEventListener('keydown', (e) => {{
      const tag = (document.activeElement && document.activeElement.tagName) || '';
      if (tag === 'INPUT' || tag === 'TEXTAREA') return;
      if (e.key === '?' && !e.ctrlKey && !e.metaKey) {{
        e.preventDefault();
        isOpen() ? close() : open();
      }} else if (e.key === 'Escape' && isOpen()) {{
        e.preventDefault();
        close();
      }}
    }});
  }})();
</script>
</body>
</html>
"##
    ))
}

fn render_config_banner(errors: &[ConfigError]) -> String {
    if errors.is_empty() {
        return String::new();
    }
    let count = errors.len();
    let first = html_escape(&errors[0].to_string());
    let summary = if count == 1 {
        format!("config: 1 error — {first}")
    } else {
        format!("config: {count} errors — {first}")
    };
    let more_button = if count > 1 {
        "<button class=\"mdv-config-banner__more\" type=\"button\">Show all</button>"
    } else {
        ""
    };
    let list_html = if count > 1 {
        let items: String = errors
            .iter()
            .map(|e| format!("<li>{}</li>", html_escape(&e.to_string())))
            .collect();
        format!("<ul class=\"mdv-config-banner__list\" hidden>{items}</ul>")
    } else {
        String::new()
    };
    format!(
        "<div id=\"mdv-config-banner\" role=\"alert\">\
<span class=\"mdv-config-banner__icon\" aria-hidden=\"true\">\u{26A0}</span>\
<span class=\"mdv-config-banner__text\">{summary}</span>\
{more_button}\
<button class=\"mdv-config-banner__close\" type=\"button\" aria-label=\"Dismiss\">\u{00D7}</button>\
{list_html}\
</div>"
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
    fn inline_code_hashbang_highlights_in_gui() {
        let html = render_page("Use `#!rust let x = 1;` now.\n", "t").expect("render");
        assert!(
            html.contains("<code class=\"mdv-code-inline\" data-lang=\"rust\">"),
            "{html}"
        );
        assert!(
            html.contains("<span class=\"mdv-tok mdv-tok-type\">let</span>"),
            "{html}"
        );
        assert!(!html.contains("#!rust"), "{html}");
    }

    #[test]
    fn inline_code_without_hashbang_keeps_existing_html() {
        let html = render_page("Use `let x = 1;` now.\n", "t").expect("render");
        assert!(html.contains("<code>let x = 1;</code>"), "{html}");
        assert!(!html.contains("<code class=\"mdv-code-inline\""), "{html}");
    }

    #[test]
    fn inline_code_unknown_lang_left_untouched() {
        let html = render_page("Use `#!nope let x = 1;` now.\n", "t").expect("render");
        assert!(html.contains("<code>#!nope let x = 1;</code>"), "{html}");
        assert!(!html.contains("<code class=\"mdv-code-inline\""), "{html}");
    }

    #[test]
    fn inline_code_in_link_text_not_highlighted() {
        let html =
            render_page("See [`#!rust let x = 1;`](https://e.x) now.\n", "t").expect("render");
        assert!(!html.contains("<code class=\"mdv-code-inline\""), "{html}");
        assert!(html.contains("#!rust let x = 1;"), "{html}");
    }

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
        let html = render_page("```mermaid\ngraph TD; A-->B;\n```\n", "t").expect("render");
        assert!(html.contains("class=\"mermaid\""), "html: {html}");
    }

    #[test]
    fn zoom_scales_media_crisply_via_geometry() {
        let html = render_page("# Hi\n\n![alt](x.png)\n", "t").expect("render");
        assert!(
            html.contains("window.__mdv_emit('zoom', { scale: zoom })"),
            "zoom event emission missing"
        );
        assert!(
            html.contains("html.classList.toggle('mdv-zoomed'"),
            "mdv-zoomed gating missing"
        );
        for prop in [
            "--mdv-img-width",
            "--mdv-plotly-width",
            "--mdv-drawio-width",
        ] {
            assert!(
                html.contains(&format!("calc(var({prop}) * var(--mdv-zoom, 1))")),
                "geometry calc rule missing for {prop}"
            );
        }
        assert!(
            html.contains("Plotly.Plots.resize(el)"),
            "plotly re-layout on zoom missing"
        );
        assert!(
            !html.contains("mdv-zoom-frame") && !html.contains("setupMdvZoomMedia"),
            "transform-based zoom machinery must not be present"
        );
    }

    #[test]
    fn code_block_goes_through_highlight_extension() {
        let html = render_page("```rust\nfn main(){}\n```\n", "t").expect("render");
        assert!(html.contains("mdv-code") || html.contains("language-rust"));
    }

    #[test]
    fn nested_list_code_block_gets_hl_lines_mark() {
        let src = "- item\n\n    ```rust hl_lines=\"2\"\n    let a = 1;\n    let b = 2;\n    ```\n";
        let html = render_page(src, "t").expect("render");
        assert!(
            html.contains("<pre class=\"mdv-code\""),
            "nested block not highlighted: {html}"
        );
        assert!(
            !html.contains("<pre><code class=\"language-rust\">"),
            "nested block fell back to comrak default: {html}"
        );
        assert_eq!(
            html.matches("hl-line hl-line--mark").count(),
            1,
            "expected exactly one marked line in nested block: {html}"
        );
    }

    #[test]
    fn nested_blockquote_code_block_gets_hl_lines_mark() {
        let src = "> ```rust hl_lines=\"2\"\n> let a = 1;\n> let b = 2;\n> ```\n";
        let html = render_page(src, "t").expect("render");
        assert!(
            html.contains("<pre class=\"mdv-code\""),
            "nested block not highlighted: {html}"
        );
        assert!(
            !html.contains("<pre><code class=\"language-rust\">"),
            "nested block fell back to comrak default: {html}"
        );
        assert_eq!(
            html.matches("hl-line hl-line--mark").count(),
            1,
            "expected exactly one marked line in nested block: {html}"
        );
    }

    #[test]
    fn nested_list_mermaid_uses_selective_extension() {
        let src = "- item\n\n    ```mermaid\n    graph TD; A-->B;\n    ```\n";
        let html = render_page(src, "t").expect("render");
        assert!(
            html.contains("class=\"mermaid\""),
            "nested mermaid not handled by ext: {html}"
        );
        assert!(
            html.contains("mermaid.min.js"),
            "nested mermaid client library not injected: {html}"
        );
    }

    #[test]
    fn top_level_code_block_stays_a_code_block() {
        let src = "```rust hl_lines=\"1\"\nlet a = 1;\n```\n";
        let html = render_page(src, "t").expect("render");
        assert!(
            html.contains("<pre class=\"mdv-code\""),
            "top-level block not highlighted: {html}"
        );
        assert_eq!(
            html.matches("hl-line hl-line--mark").count(),
            1,
            "expected exactly one marked line in top-level block: {html}"
        );
    }

    #[test]
    fn includes_minimap_scaffold() {
        let html = render_page("# Hi\n\nsome text\n", "t").expect("render");
        assert!(html.contains("#mdv-minimap"), "expected minimap CSS");
        assert!(
            html.contains("__mdvToggleCodemap"),
            "expected minimap toggle JS"
        );
        assert!(
            html.contains("__mdvMountMinimap"),
            "expected lazy-mount JS function"
        );
    }

    #[test]
    fn codemap_dom_not_mounted_on_load() {
        // tasks.md #12: codemap should not be present in the initial document.
        // Only the minimap *toggle* and *mount* function definitions appear in
        // script bodies; the page itself must not include a pre-built
        // `<div id="mdv-minimap">` or `<div id="mdv-minimap-content">` element.
        let html = render_page("# Hi\n\nsome text\n", "t").expect("render");
        let body_start = html.find("<body").expect("body tag");
        let body_end = html.find("</body>").expect("end body tag");
        let body = &html[body_start..body_end];
        // Strip out <script>…</script> blocks before scanning so we only
        // examine markup that the browser materializes on initial paint.
        let mut stripped = String::with_capacity(body.len());
        let mut rest = body;
        while let Some(open) = rest.find("<script") {
            stripped.push_str(&rest[..open]);
            let after_open = &rest[open..];
            let close = after_open
                .find("</script>")
                .map(|i| i + "</script>".len())
                .unwrap_or(after_open.len());
            rest = &after_open[close..];
        }
        stripped.push_str(rest);
        assert!(
            !stripped.contains("id=\"mdv-minimap\""),
            "minimap DOM must not be present on initial page load; found in: {stripped}"
        );
        assert!(
            !stripped.contains("id=\"mdv-minimap-content\""),
            "minimap content DOM must not be present on initial page load"
        );
    }

    #[test]
    fn codemap_initial_state_is_hardcoded_hidden() {
        // tasks.md #13: no flicker. The early <head> bootstrap script must not
        // touch any `mdv-codemap-hidden` html class (we removed the flip), and
        // the page must not depend on sessionStorage for initial codemap state.
        let html = render_page("# Hi\n", "t").expect("render");
        let head_end = html.find("</head>").expect("head end");
        let head = &html[..head_end];
        assert!(
            !head.contains("mdv-codemap-hidden"),
            "early bootstrap must not toggle mdv-codemap-hidden; got head: {head}"
        );
        assert!(
            !head.contains("mdv:codemap-hidden"),
            "initial state must not read mdv:codemap-hidden from sessionStorage"
        );
    }

    #[test]
    fn emits_theme_root_variables() {
        let html = render_page("# Hi\n", "t").expect("render");
        assert!(
            html.contains("--mdv-bg:"),
            "expected --mdv-bg from theme css"
        );
        assert!(
            html.contains("--mdv-fg:"),
            "expected --mdv-fg from theme css"
        );
        assert!(
            html.contains("--mdv-accent-mauve:"),
            "expected --mdv-accent-mauve from accent palette"
        );
        assert!(
            html.contains("--bg:var(--mdv-bg)"),
            "expected legacy --bg bridging var"
        );
    }

    #[test]
    fn emits_both_light_and_dark_theme_blocks() {
        use mdview_config::ThemeMode;
        let cfg = ThemeConfig {
            mode: ThemeMode::Dark,
            ..ThemeConfig::default()
        };
        let html = render_page_with_theme("# Hi\n", "t", &cfg).expect("render");
        assert!(
            html.contains(":root.theme-light {"),
            "expected :root.theme-light block"
        );
        assert!(
            html.contains(":root.theme-dark {"),
            "expected :root.theme-dark block"
        );
        assert!(
            html.contains("<html lang=\"en\" class=\"theme-dark\""),
            "expected initial <html class=\"theme-dark\">"
        );
    }

    #[test]
    fn light_mode_sets_initial_html_class() {
        use mdview_config::ThemeMode;
        let cfg = ThemeConfig {
            mode: ThemeMode::Light,
            ..ThemeConfig::default()
        };
        let html = render_page_with_theme("# Hi\n", "t", &cfg).expect("render");
        assert!(
            html.contains("<html lang=\"en\" class=\"theme-light\""),
            "expected initial <html class=\"theme-light\">"
        );
    }

    #[test]
    fn initial_bg_hex_uses_mocha_for_default_dark() {
        use mdview_config::ThemeMode;
        let cfg = ThemeConfig {
            mode: ThemeMode::Dark,
            ..ThemeConfig::default()
        };
        assert_eq!(initial_bg_hex(&cfg), "#1e1e2e");
    }

    #[test]
    fn initial_bg_hex_uses_latte_when_light_mode() {
        use mdview_config::ThemeMode;
        let cfg = ThemeConfig {
            mode: ThemeMode::Light,
            ..ThemeConfig::default()
        };
        assert_eq!(initial_bg_hex(&cfg), "#eff1f5");
    }

    #[test]
    fn initial_bg_hex_falls_back_when_theme_missing() {
        use mdview_config::ThemeMode;
        let cfg = ThemeConfig {
            mode: ThemeMode::Dark,
            dark: "does-not-exist".to_string(),
            ..ThemeConfig::default()
        };
        // Falls back to catppuccin-mocha base.
        assert_eq!(initial_bg_hex(&cfg), "#1e1e2e");
    }

    #[test]
    fn html_has_inline_background_matching_theme_dark() {
        use mdview_config::ThemeMode;
        let cfg = ThemeConfig {
            mode: ThemeMode::Dark,
            ..ThemeConfig::default()
        };
        let html = render_page_with_theme("# Hi\n", "t", &cfg).expect("render");
        assert!(
            html.contains("<html lang=\"en\" class=\"theme-dark\" style=\"background:#1e1e2e\">"),
            "expected initial <html> to inline the dark theme background; got:\n{}",
            &html[..html.find("<head").unwrap_or(200.min(html.len()))]
        );
    }

    #[test]
    fn html_has_inline_background_matching_theme_light() {
        use mdview_config::ThemeMode;
        let cfg = ThemeConfig {
            mode: ThemeMode::Light,
            ..ThemeConfig::default()
        };
        let html = render_page_with_theme("# Hi\n", "t", &cfg).expect("render");
        assert!(
            html.contains("<html lang=\"en\" class=\"theme-light\" style=\"background:#eff1f5\">"),
            "expected initial <html> to inline the light theme background"
        );
    }

    #[test]
    fn omits_config_banner_when_no_errors() {
        let html = render_page("# Hi\n", "t").expect("render");
        assert!(
            !html.contains("id=\"mdv-config-banner\""),
            "banner should be absent when there are no errors"
        );
    }

    #[test]
    fn renders_config_banner_with_errors() {
        let errors = vec![
            ConfigError::keymap(
                "quit",
                Some("Ctrr+Q".to_string()),
                "unknown modifier \"Ctrr\"; expected one of Ctrl, Shift, Alt, Super",
            ),
            ConfigError::keymap(
                "togle-theme",
                Some("Ctrl+T".to_string()),
                "unknown action \"togle-theme\"",
            ),
            ConfigError::toc_fatal(
                "levels",
                Some("9".to_string()),
                "out of range; expected 1..=6",
            ),
        ];
        let html = render_page_full(
            "# Hi\n",
            "t",
            &ThemeConfig::default(),
            &Keymap::defaults(),
            None,
            &errors,
            &TocConfig::default(),
            &CodemapConfig::default(),
            &CodeConfig::default(),
        )
        .expect("render");
        assert!(html.contains("id=\"mdv-config-banner\""));
        assert!(html.contains("config: 3 errors"));
        assert!(html.contains("mdv-config-banner__list"));
        assert!(html.contains("Show all"));
        // HTML-escaped quote markers should be present.
        assert!(html.contains("&quot;"));
    }

    #[test]
    fn single_error_omits_show_all_button() {
        let errors = vec![ConfigError::toc_fatal(
            "levels",
            Some("9".to_string()),
            "out of range; expected 1..=6",
        )];
        let html = render_page_full(
            "# Hi\n",
            "t",
            &ThemeConfig::default(),
            &Keymap::defaults(),
            None,
            &errors,
            &TocConfig::default(),
            &CodemapConfig::default(),
            &CodeConfig::default(),
        )
        .expect("render");
        assert!(html.contains("id=\"mdv-config-banner\""));
        assert!(html.contains("config: 1 error"));
        assert!(!html.contains("class=\"mdv-config-banner__more\""));
        assert!(!html.contains("class=\"mdv-config-banner__list\""));
    }

    #[test]
    fn head_omits_math_when_no_math_in_doc() {
        let html = render_page("# hi", "t").expect("render");
        assert!(!html.contains("katex.min.css"), "html: {html}");
        assert!(!html.contains("katex.min.js"));
    }

    #[test]
    fn head_includes_math_when_inline_math_present() {
        let html = render_page("hello $x$ world\n", "t").expect("render");
        assert!(html.contains("katex.min.css"), "html: {html}");
        assert!(html.contains("katex.min.js"));
    }

    #[test]
    fn head_includes_math_when_display_math_present() {
        let html = render_page("$$x^2$$\n", "t").expect("render");
        assert!(html.contains("katex.min.css"), "html: {html}");
    }

    #[test]
    fn head_includes_mermaid_when_block_present() {
        let html = render_page("```mermaid\nflowchart LR\nA-->B\n```\n", "t").expect("render");
        assert!(html.contains("mermaid.min.js"), "html: {html}");
    }

    #[test]
    fn head_includes_plotly() {
        let html = render_page("```plotly\n{\"data\":[]}\n```\n", "t").expect("render");
        assert!(html.contains("plotly-2.35.2.min.js"), "html: {html}");
    }

    #[test]
    fn head_includes_drawio() {
        let html = render_page("```drawio\n<mxfile/>\n```\n", "t").expect("render");
        assert!(html.contains("viewer.diagrams.net"), "html: {html}");
    }

    #[test]
    fn head_includes_only_needed() {
        let html = render_page("$x$\n", "t").expect("render");
        assert!(html.contains("katex.min.js"));
        assert!(!html.contains("mermaid.min.js"), "mermaid should be absent");
        assert!(
            !html.contains("plotly-2.35.2.min.js"),
            "plotly should be absent"
        );
        assert!(
            !html.contains("viewer.diagrams.net"),
            "drawio should be absent"
        );
    }

    #[test]
    fn showcase_fixture_includes_all_libs() {
        let src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/showcase.md"),
        )
        .expect("read showcase.md");
        let html = render_page(&src, "showcase").expect("render");
        assert!(html.contains("katex.min.css"), "katex css missing");
        assert!(html.contains("katex.min.js"), "katex js missing");
        assert!(html.contains("mermaid.min.js"), "mermaid missing");
        assert!(html.contains("plotly-2.35.2.min.js"), "plotly missing");
        assert!(html.contains("viewer.diagrams.net"), "drawio missing");
    }

    #[test]
    fn rewrites_relative_image_urls_to_mdview_protocol() {
        let dir = std::env::temp_dir();
        let html = render_page_with_config_and_source(
            "![alt](./test.png)\n",
            "t",
            &ThemeConfig::default(),
            &Keymap::defaults(),
            Some(dir.as_path()),
        )
        .expect("render");
        let src = html
            .split_once("<img ")
            .and_then(|(_, rest)| rest.split_once("src=\""))
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(src, _)| src)
            .unwrap_or_else(|| panic!("no <img src> in rendered page, got: {html}"));
        assert!(
            src.starts_with(MDVIEW_PROTOCOL_BASE),
            "expected image src under {MDVIEW_PROTOCOL_BASE}, got: {src}"
        );
        assert!(
            src.contains("test.png"),
            "rewritten src lost the image filename, got: {src}"
        );
        assert_ne!(src, "./test.png", "relative image src was not rewritten");
    }

    #[test]
    fn style_block_includes_toc_and_task_icon_rules() {
        let html = render_page("# Hi\n", "t").expect("render");
        assert!(
            html.contains(".mdv-toc--hidden"),
            "expected .mdv-toc--hidden rule in <style>"
        );
        assert!(
            html.contains(".mdv-toc--floating-right"),
            "expected .mdv-toc--floating-right rule in <style>"
        );
        assert!(
            html.contains(".mdv-task-icon"),
            "expected Fluent task icon CSS rule in <style>"
        );
        assert!(
            html.contains(".mdv-task-checked"),
            "expected .mdv-task-checked rule in <style>"
        );
        assert!(
            html.contains(".mdv-task-unchecked"),
            "expected .mdv-task-unchecked rule in <style>"
        );
        assert!(
            !html.contains("task-list-item > input"),
            "obsolete native-checkbox rule should be gone"
        );
    }

    #[test]
    fn task_list_html_uses_fluent_svg_icons() {
        let html = render_page("- [x] done\n- [ ] todo\n", "t").expect("render");
        assert!(
            html.contains("mdv-task-checked"),
            "Fluent CheckmarkCircle SVG missing in body: {html}"
        );
        assert!(
            html.contains("mdv-task-unchecked"),
            "Fluent Circle SVG missing in body: {html}"
        );
        assert!(
            !html.contains("<input type=\"checkbox\""),
            "native checkbox input should be replaced: {html}"
        );
    }

    #[test]
    fn plain_bullet_list_is_not_decorated_with_task_icons() {
        let html = render_page("- one\n- two\n", "t").expect("render");
        assert!(
            !html.contains("<svg class=\"mdv-task-icon"),
            "plain bullets should not get task icons: {html}"
        );
    }

    #[test]
    fn emits_toc_aside_and_toggle_js() {
        let html = render_page("# Hi\n\n## Section\n\n## Other\n", "t").expect("render");
        assert!(
            html.contains("id=\"mdv-toc\""),
            "expected TOC aside element with id=mdv-toc"
        );
        assert!(
            html.contains("window.__mdvToggleToc"),
            "expected window.__mdvToggleToc to be defined"
        );
        assert!(
            html.contains("mdv-toc--floating-right"),
            "expected default floating-right position class"
        );
    }

    #[test]
    fn toc_initial_state_stays_hidden_until_toggle() {
        let html = render_page("# One\n\n## Two\n", "t").expect("render");
        assert!(
            html.contains(
                "id=\"mdv-toc\" class=\"mdv-toc mdv-toc--floating-right mdv-toc--hidden\" aria-hidden=\"true\""
            ),
            "TOC aside must be emitted hidden on load; got: {html}"
        );
        assert!(
            !html.contains("aside.classList.remove('mdv-toc--hidden')"),
            "setupToc must not reveal the TOC during initial load"
        );
        assert!(
            html.contains("window.__mdvToggleToc = function"),
            "toggle-toc must remain available so the hidden TOC can be shown"
        );
    }

    #[test]
    fn toc_click_scroll_resets_inline_axis() {
        let html = render_page("# Hi\n\n## Target\n\n## Other\n", "t").expect("render");
        assert!(
            html.contains(
                "const y = target.getBoundingClientRect().top + window.scrollY;\n          window.scrollTo({ left: 0, top: y, behavior: 'smooth' });"
            ),
            "TOC click handler must reset the inline axis via window.scrollTo; got: {html}"
        );
        assert!(
            !html.contains("scrollIntoView"),
            "TOC click handler must not use scrollIntoView, which scrolls the inline axis"
        );
    }

    #[test]
    fn toc_heading_text_is_table_of_content_singular() {
        let html = render_page("# Hi\n\n## A\n\n## B\n", "t").expect("render");
        assert!(
            html.contains(">Table of Content<"),
            "expected literal 'Table of Content' heading"
        );
        // Old text must be gone.
        assert!(
            !html.contains(">Contents<"),
            "TOC heading must no longer say 'Contents'"
        );
        // Singular: no trailing 's'.
        assert!(
            !html.contains("Table of Contents"),
            "TOC heading must be singular ('Content', not 'Contents')"
        );
    }

    #[test]
    fn toc_title_class_is_styled_mauve() {
        let html = render_page("# Hi\n\n## A\n", "t").expect("render");
        assert!(
            html.contains("class=\"mdv-toc__title\""),
            "expected title span carries mdv-toc__title class"
        );
        assert!(
            html.contains(".mdv-toc__title") && html.contains("#cba6f7"),
            "expected catppuccin mauve (#cba6f7) rule for .mdv-toc__title"
        );
    }

    #[test]
    fn toc_aside_omits_close_button() {
        let html = render_page("# Hi\n\n## Section\n", "t").expect("render");
        assert!(
            html.contains("id=\"mdv-toc\""),
            "expected TOC aside element"
        );
        assert!(
            html.contains("class=\"mdv-toc__title\""),
            "expected TOC title span to remain"
        );
        assert!(
            !html.contains("id=\"mdv-toc-close\""),
            "TOC close button must be removed"
        );
        assert!(!html.contains("closeBtn"), "no dead closeBtn JS reference");
        assert!(
            !html.contains(".mdv-toc header button"),
            "dead close-button CSS must be removed"
        );
    }

    #[test]
    fn toc_levels_propagates_to_js_data_island() {
        let toc = TocConfig {
            position: TocPosition::FloatingRight,
            levels: 4,
        };
        let html = render_page_full(
            "# H1\n## H2\n### H3\n#### H4\n##### H5\n",
            "t",
            &ThemeConfig::default(),
            &Keymap::defaults(),
            None,
            &[],
            &toc,
            &CodemapConfig::default(),
            &CodeConfig::default(),
        )
        .expect("render");
        assert!(
            html.contains("levels: 4"),
            "expected `levels: 4` in __mdv_config TOC island"
        );
        assert!(
            !html.contains("depth:"),
            "old `depth:` key should be gone from data island"
        );
    }

    #[test]
    fn toc_indent_computation_handles_shallowest_baseline() {
        // The indent rule is exercised in JS at runtime. We assert the JS
        // source carries the baseline algorithm (shallowest-level scan +
        // per-li padding) so the contract isn't silently regressed by future
        // edits.
        let html = render_page("# Hi\n## A\n### B\n", "t").expect("render");
        assert!(
            html.contains("shallowest"),
            "TOC JS must compute the shallowest heading level"
        );
        assert!(
            html.contains("lvl - shallowest"),
            "TOC JS indent must be relative to the shallowest heading"
        );
        assert!(
            html.contains("paddingLeft"),
            "TOC JS must apply per-li padding for indent"
        );
        assert!(
            html.contains("tocIndent") || html.contains("data-toc-indent"),
            "TOC JS should expose computed indent on each <li>"
        );
    }

    #[test]
    fn embeds_keymap_data_island() {
        let html = render_page("# Hi\n", "t").expect("render");
        assert!(
            html.contains("id=\"mdv-keymap\""),
            "expected mdv-keymap data island"
        );
    }

    #[test]
    fn binding_to_json_punctuation_has_no_shift() {
        // A "?" binding must serialize with shift:false and the raw glyph so the
        // GUI matcher can ignore the DOM Shift flag for it.
        let b: KeyBinding = "?".parse().expect("parse");
        let json = binding_to_json(&b);
        assert!(json.contains("\"shift\":false"), "got: {json}");
        assert!(json.contains("\"kind\":\"char\""), "got: {json}");
        assert!(json.contains("\"key\":\"?\""), "got: {json}");
    }

    #[test]
    fn images_get_lazy_attrs() {
        let html = render_page("![alt](https://example.com/a.png)\n", "t").expect("render");
        assert!(
            html.contains("loading=\"lazy\""),
            "expected lazy attr in {html}"
        );
        assert!(
            html.contains("decoding=\"async\""),
            "expected decoding attr"
        );
    }

    #[test]
    fn head_minified() {
        let html = render_page("# hi\n", "t").expect("render");
        let head_end = html.find("</head>").unwrap();
        let head = &html[..head_end];
        for line in head.lines() {
            assert!(
                !line.starts_with("  "),
                "head line should be stripped: {line:?}"
            );
        }
    }

    #[test]
    fn tab_size_css_applied() {
        let html = render_page("```\n\tindented\n```\n", "t").expect("render");
        assert!(
            html.contains("tab-size: 4"),
            "expected tab-size CSS property as fallback"
        );
        // Tabs are expanded to spaces by the highlight extension
        assert!(
            html.contains("    indented"),
            "expected tab expanded to 4 spaces in code block HTML"
        );
    }
}
