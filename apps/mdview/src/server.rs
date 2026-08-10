use anyhow::Result;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub type HtmlStore = Arc<RwLock<Arc<String>>>;

pub struct Server {
    pub port: u16,
    pub html: HtmlStore,
    _handle: tokio::task::JoinHandle<()>,
}

pub async fn serve_html(html: String) -> Result<Server> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let html: HtmlStore = Arc::new(RwLock::new(Arc::new(html)));

    let html_for_loop = html.clone();
    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let body = match html_for_loop.read() {
                Ok(guard) => guard.clone(),
                Err(_) => Arc::new(String::new()),
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf).await;
                let bytes = body.as_bytes();
                let headers = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
                    bytes.len()
                );
                let _ = stream.write_all(headers.as_bytes()).await;
                let _ = stream.write_all(bytes).await;
                let _ = stream.flush().await;
            });
        }
    });

    Ok(Server {
        port,
        html,
        _handle: handle,
    })
}

/// Outcome of mapping a request path onto the serve root.
#[derive(Debug, PartialEq, Eq)]
pub enum Route {
    /// `/` — serve the file named on the command line.
    Index,
    /// A `.md` document resolved to an absolute path inside the serve root.
    Markdown(PathBuf),
    /// Malformed percent-encoding or a non-UTF-8 target.
    BadRequest,
    /// Outside the serve root, not a `.md`, or missing.
    NotFound,
}

/// Percent-decode exactly once. Returns None on a malformed escape or a byte
/// sequence that is not valid UTF-8 — both are rejected as 400 rather than
/// silently coerced, so a mangled path can never be reinterpreted as a
/// different file.
fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3)?;
            let hex = std::str::from_utf8(hex).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Map a raw request target onto the serve root.
///
/// Containment is enforced on the *canonicalized* paths with component-aware
/// `Path::starts_with`, never a string comparison: a string prefix test would
/// accept `C:\root-evil` for root `C:\root`. Canonicalizing also normalizes
/// `/` vs `\` and resolves symlinks/junctions, so a decoded `..` cannot escape
/// even when it survives client-side normalization.
pub fn route_request(target: &str, root: &Path) -> Route {
    let path = target.split(['?', '#']).next().unwrap_or("");
    let Some(decoded) = percent_decode(path) else {
        return Route::BadRequest;
    };
    if decoded == "/" || decoded.is_empty() {
        return Route::Index;
    }
    let rel = decoded.trim_start_matches('/');
    if !rel.to_ascii_lowercase().ends_with(".md") {
        return Route::NotFound;
    }
    let rel = PathBuf::from(rel.replace('\\', "/"));
    if rel
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
        || rel.is_absolute()
    {
        return Route::NotFound;
    }
    let (Ok(root_abs), Ok(target_abs)) = (root.canonicalize(), root.join(&rel).canonicalize())
    else {
        return Route::NotFound;
    };
    if !target_abs.starts_with(&root_abs) {
        return Route::NotFound;
    }
    Route::Markdown(target_abs)
}

fn http_response(status: &str, content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
        body.len()
    )
    .into_bytes();
    out.extend_from_slice(body);
    out
}

fn render_response<F>(render: &F, file: &Path) -> Vec<u8>
where
    F: Fn(&Path) -> Result<String>,
{
    match render(file) {
        Ok(body) => http_response("200 OK", "text/html; charset=utf-8", body.as_bytes()),
        Err(e) => http_response(
            "500 Internal Server Error",
            "text/plain; charset=utf-8",
            format!("render error: {e}").as_bytes(),
        ),
    }
}

/// Serve markdown from `root`, rendering each request on demand.
///
/// Used only by `--serve-only`. The GUI path keeps using `serve_html`, which is
/// untouched.
pub async fn serve_dir<F>(
    root: PathBuf,
    index: PathBuf,
    port: Option<u16>,
    render: F,
) -> Result<Server>
where
    F: Fn(&Path) -> Result<String> + Send + Sync + 'static,
{
    let listener = TcpListener::bind(("127.0.0.1", port.unwrap_or(0))).await?;
    let bound = listener.local_addr()?.port();
    let html: HtmlStore = Arc::new(RwLock::new(Arc::new(String::new())));
    let render = Arc::new(render);

    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let root = root.clone();
            let index = index.clone();
            let render = render.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]).to_string();
                let target = head
                    .lines()
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .unwrap_or("/")
                    .to_string();

                let resp = match route_request(&target, &root) {
                    Route::BadRequest => http_response(
                        "400 Bad Request",
                        "text/plain; charset=utf-8",
                        b"malformed request target",
                    ),
                    Route::NotFound => {
                        http_response("404 Not Found", "text/plain; charset=utf-8", b"not found")
                    }
                    Route::Index => render_response(render.as_ref(), &index),
                    Route::Markdown(p) => render_response(render.as_ref(), &p),
                };
                let _ = stream.write_all(&resp).await;
                let _ = stream.flush().await;
            });
        }
    });

    Ok(Server {
        port: bound,
        html,
        _handle: handle,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mdv-route-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("fixtures")).unwrap();
        std::fs::write(dir.join("fixtures/gfm.md"), "# GFM\n").unwrap();
        dir.canonicalize().unwrap()
    }

    #[test]
    fn routes_root_to_index() {
        let root = tmp_root();
        assert_eq!(route_request("/", &root), Route::Index);
    }

    #[test]
    fn routes_markdown_under_root() {
        let root = tmp_root();
        match route_request("/fixtures/gfm.md", &root) {
            Route::Markdown(p) => assert!(p.ends_with("gfm.md")),
            other => panic!("expected Markdown, got {other:?}"),
        }
    }

    #[test]
    fn strips_query_string() {
        let root = tmp_root();
        assert!(matches!(
            route_request("/fixtures/gfm.md?v=1", &root),
            Route::Markdown(_)
        ));
    }

    #[test]
    fn rejects_non_markdown() {
        let root = tmp_root();
        assert_eq!(route_request("/secret.txt", &root), Route::NotFound);
    }

    #[test]
    fn rejects_missing_file() {
        let root = tmp_root();
        assert_eq!(route_request("/fixtures/nope.md", &root), Route::NotFound);
    }

    // The traversal target is percent-encoded so it survives any client-side
    // normalization, and points at a file that really exists outside the root
    // — otherwise the test would pass merely because the file was missing.
    #[test]
    fn rejects_encoded_parent_traversal_to_real_file() {
        let root = tmp_root();
        let secret = root.parent().unwrap().join("mdv-secret.md");
        std::fs::write(&secret, "# TOP SECRET\n").unwrap();
        assert!(
            secret.exists(),
            "traversal target must exist to be a real test"
        );
        assert_eq!(
            route_request("/%2e%2e/mdv-secret.md", &root),
            Route::NotFound
        );
        assert_eq!(route_request("/../mdv-secret.md", &root), Route::NotFound);
        let _ = std::fs::remove_file(&secret);
    }

    #[test]
    fn rejects_malformed_percent_encoding() {
        let root = tmp_root();
        assert_eq!(route_request("/%zz.md", &root), Route::BadRequest);
        assert_eq!(route_request("/%2.md", &root), Route::BadRequest);
    }

    // A sibling directory sharing the root's name prefix must not be reachable.
    // A string `starts_with` containment check would wrongly accept this.
    #[test]
    fn rejects_sibling_dir_with_shared_prefix() {
        let root = tmp_root();
        let evil = PathBuf::from(format!("{}-evil", root.display()));
        std::fs::create_dir_all(&evil).unwrap();
        std::fs::write(evil.join("x.md"), "# evil\n").unwrap();
        let name = evil.file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(
            route_request(&format!("/../{name}/x.md"), &root),
            Route::NotFound
        );
        let _ = std::fs::remove_dir_all(&evil);
    }
}
