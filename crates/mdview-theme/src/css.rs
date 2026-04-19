use crate::_stubs::Theme;
use std::fmt::Write;

pub fn emit_css(theme: &Theme) -> String {
    let mut out = String::with_capacity(2048);

    out.push_str(":root {\n");
    for (key, value) in &theme.colors {
        let _ = writeln!(out, "  --mdv-{}: {};", key.replace('.', "-"), value);
    }
    let _ = writeln!(out, "  --mdv-radius-sm: {}px;", theme.radii.sm);
    let _ = writeln!(out, "  --mdv-radius-md: {}px;", theme.radii.md);
    let _ = writeln!(out, "  --mdv-radius-lg: {}px;", theme.radii.lg);
    let _ = writeln!(out, "  --mdv-font-body: {};", theme.typography.body);
    let _ = writeln!(out, "  --mdv-font-mono: {};", theme.typography.mono);
    let _ = writeln!(out, "  --mdv-font-headings: {};", theme.typography.headings);
    out.push_str("}\n\n");

    out.push_str(
        "body {\n  background: var(--mdv-bg);\n  color: var(--mdv-fg);\n  font-family: var(--mdv-font-body);\n  line-height: 1.7;\n  font-size: 16px;\n  margin: 0;\n  padding: 2.5rem 3rem;\n}\n\n",
    );

    out.push_str(
        "h1, h2, h3, h4, h5, h6 {\n  font-family: var(--mdv-font-headings);\n  line-height: 1.25;\n  margin: 1.8em 0 0.6em;\n  letter-spacing: -0.01em;\n}\nh1 { font-size: 2.25rem; }\nh2 { font-size: 1.75rem; }\nh3 { font-size: 1.375rem; }\nh4 { font-size: 1.15rem; }\nh5 { font-size: 1rem; }\nh6 { font-size: 0.9rem; color: var(--mdv-muted); }\n\n",
    );

    out.push_str(
        "a { color: var(--mdv-link); text-decoration: none; border-bottom: 1px solid color-mix(in srgb, var(--mdv-link) 40%, transparent); }\na:hover { border-bottom-color: var(--mdv-link); }\n\n",
    );

    out.push_str(
        "code { font-family: var(--mdv-font-mono); background: var(--mdv-code-bg); border-radius: var(--mdv-radius-sm); padding: 0.15em 0.4em; font-size: 0.92em; }\npre { background: var(--mdv-code-bg); border-radius: var(--mdv-radius-md); padding: 1.1rem 1.25rem; overflow-x: auto; box-shadow: 0 1px 2px rgba(0,0,0,0.06), 0 1px 3px rgba(0,0,0,0.04); border: 1px solid var(--mdv-border-subtle); }\npre code { background: transparent; padding: 0; }\n\n",
    );

    out.push_str(
        "blockquote { margin: 1.2em 0; padding: 0.25em 1.2em; border-left: 3px solid var(--mdv-accent); color: var(--mdv-muted); border-radius: var(--mdv-radius-sm); background: color-mix(in srgb, var(--mdv-accent) 5%, transparent); }\n\n",
    );

    out.push_str(
        "table { border-collapse: separate; border-spacing: 0; width: 100%; margin: 1.2em 0; border: 1px solid var(--mdv-table-border); border-radius: var(--mdv-radius-md); overflow: hidden; box-shadow: 0 1px 2px rgba(0,0,0,0.05); }\nth, td { padding: 0.6em 0.9em; text-align: left; border-bottom: 1px solid var(--mdv-table-border); }\ntr:last-child td { border-bottom: none; }\nth { background: color-mix(in srgb, var(--mdv-accent) 8%, transparent); font-weight: 600; }\n\n",
    );

    out.push_str(
        "hr { border: 0; border-top: 1px solid var(--mdv-border-subtle); margin: 2em 0; }\nimg { max-width: 100%; border-radius: var(--mdv-radius-md); }\nul, ol { padding-left: 1.6em; }\nli { margin: 0.3em 0; }\n",
    );

    out
}
