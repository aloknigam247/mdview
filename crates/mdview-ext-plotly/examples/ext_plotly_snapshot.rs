use std::fs;

fn main() {
    let fixture = mdview_ext_plotly::locate_fixture("plotly.md");
    let md = fs::read_to_string(&fixture).expect("read fixture");
    let html = mdview_ext_plotly::render_markdown_html(&md);
    println!("{html}");
}
