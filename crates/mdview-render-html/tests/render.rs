use mdview_render_html::{
    htmlesc, render_markdown, AstNode, Html, HtmlRenderer, Registry, RenderCtx,
};

fn ctx() -> RenderCtx {
    RenderCtx::default()
}

#[test]
fn renders_heading() {
    let html = render_markdown("# Hello\n", &ctx(), &Registry::new());
    assert!(html.contains("<h1>Hello</h1>"), "html: {html}");
    assert!(html.starts_with("<!doctype html>"));
}

#[test]
fn renders_multiple_heading_levels() {
    let md = "# H1\n## H2\n### H3\n";
    let html = render_markdown(md, &ctx(), &Registry::new());
    assert!(html.contains("<h1>H1</h1>"));
    assert!(html.contains("<h2>H2</h2>"));
    assert!(html.contains("<h3>H3</h3>"));
}

#[test]
fn renders_paragraph_and_emphasis() {
    let html = render_markdown("A **bold** and *em* mix.\n", &ctx(), &Registry::new());
    assert!(html.contains("<p>A <strong>bold</strong> and <em>em</em> mix.</p>"));
}

#[test]
fn renders_unordered_list() {
    let md = "- one\n- two\n- three\n";
    let html = render_markdown(md, &ctx(), &Registry::new());
    assert!(html.contains("<ul>"));
    assert!(html.contains("<li>one</li>"));
    assert!(html.contains("<li>three</li>"));
}

#[test]
fn renders_ordered_list() {
    let md = "1. a\n2. b\n";
    let html = render_markdown(md, &ctx(), &Registry::new());
    assert!(html.contains("<ol"));
    assert!(html.contains("<li>a</li>"));
}

#[test]
fn renders_table_with_rounded_radius_css() {
    let md = "| A | B |\n| - | - |\n| 1 | 2 |\n";
    let html = render_markdown(md, &ctx(), &Registry::new());
    assert!(html.contains("<table>"));
    assert!(html.contains("<th>A</th>"));
    assert!(html.contains("<td>1</td>"));
    assert!(
        html.contains("border-radius:var(--mdv-radius-md)"),
        "rounded radius should be applied to tables"
    );
    assert!(html.contains("--mdv-radius-md: 10px"));
}

#[test]
fn renders_code_block() {
    let md = "```rust\nfn main() {}\n```\n";
    let html = render_markdown(md, &ctx(), &Registry::new());
    assert!(html.contains("<pre"));
    assert!(html.contains("<code"));
    assert!(html.contains("fn main()"));
}

#[test]
fn renders_blockquote() {
    let md = "> hello world\n";
    let html = render_markdown(md, &ctx(), &Registry::new());
    assert!(html.contains("<blockquote>"));
    assert!(html.contains("hello world"));
}

#[test]
fn renders_inline_code() {
    let html = render_markdown("use `Vec::new()` here.\n", &ctx(), &Registry::new());
    assert!(html.contains("<code>Vec::new()</code>"));
}

#[test]
fn renders_link() {
    let html = render_markdown("[ex](https://example.com)\n", &ctx(), &Registry::new());
    assert!(html.contains("href=\"https://example.com\""));
    assert!(html.contains(">ex</a>"));
}

#[test]
fn renders_image() {
    let html = render_markdown("![alt](https://x.test/y.png)\n", &ctx(), &Registry::new());
    assert!(html.contains("<img"));
    assert!(html.contains("src=\"https://x.test/y.png\""));
    assert!(html.contains("alt=\"alt\""));
}

#[test]
fn injects_live_reload_script_when_enabled() {
    let c = RenderCtx {
        live_reload: true,
        ..RenderCtx::default()
    };
    let html = render_markdown("# Hi\n", &c, &Registry::new());
    assert!(html.contains("__mdview_live"));
}

#[test]
fn skips_live_reload_script_when_disabled() {
    let html = render_markdown("# Hi\n", &RenderCtx::default(), &Registry::new());
    assert!(!html.contains("__mdview_live"));
}

struct UppercaseHeading;

impl HtmlRenderer for UppercaseHeading {
    fn name(&self) -> &'static str {
        "uppercase_heading"
    }
    fn node_types(&self) -> &'static [&'static str] {
        &["heading"]
    }
    fn test<'a>(&self, node: &'a AstNode<'a>) -> bool {
        matches!(
            node.data.borrow().value,
            comrak::nodes::NodeValue::Heading(_)
        )
    }
    fn render<'a>(&self, node: &'a AstNode<'a>, _ctx: &RenderCtx) -> Html {
        let mut text = String::new();
        collect_text(node, &mut text);
        format!(
            "<h1 class=\"mdv-custom\">{}</h1>",
            htmlesc::escape_html(&text.to_uppercase())
        )
    }
}

fn collect_text<'a>(node: &'a AstNode<'a>, out: &mut String) {
    use comrak::nodes::NodeValue;
    if let NodeValue::Text(t) = &node.data.borrow().value {
        out.push_str(t);
    }
    for c in node.children() {
        collect_text(c, out);
    }
}

#[test]
fn registry_override_is_applied() {
    let registry = Registry::new().with_html_renderer(UppercaseHeading);
    let html = render_markdown("# hello\n\nparagraph\n", &ctx(), &registry);
    assert!(
        html.contains("<h1 class=\"mdv-custom\">HELLO</h1>"),
        "override not applied: {html}"
    );
    assert!(html.contains("<p>paragraph</p>"));
}
