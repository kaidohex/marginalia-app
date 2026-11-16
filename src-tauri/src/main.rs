// The Marginalia shell.
//
// The whole single-page app is compiled into this executable (rust-embed picks
// up ../dist/client, which the Tauri build step produces first) and served
// from a loopback HTTP server inside the process. A real origin keeps
// absolute asset paths, localStorage, and the history API working exactly
// like the web build, with no port jump and no data folder.
//
// Feed and article requests cannot come from the renderer directly (the web
// is cross-origin), so the same server exposes /proxy?url=... which fetches
// server-side - the same job the TanStack Start server function does on the
// web build.
mod proxy;

use rust_embed::RustEmbed;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Keep one profile name everywhere: localStorage is keyed by origin, and the
/// origin is fixed on purpose (http://127.0.0.1:43117), so saved feeds survive
/// every launch. It also matches the Electron build's origin.
const APP_PORT: u16 = 43117;

const WINDOW_TITLE: &str = "Marginalia — Your daily reading desk";

#[derive(RustEmbed)]
#[folder = "../dist/client"]
struct Assets;

const MIME_TYPES: &[(&str, &str)] = &[
    ("css", "text/css; charset=utf-8"),
    ("html", "text/html; charset=utf-8"),
    ("ico", "image/x-icon"),
    ("jpeg", "image/jpeg"),
    ("jpg", "image/jpeg"),
    ("js", "text/javascript; charset=utf-8"),
    ("json", "application/json; charset=utf-8"),
    ("map", "application/json; charset=utf-8"),
    ("png", "image/png"),
    ("svg", "image/svg+xml"),
    ("txt", "text/plain; charset=utf-8"),
    ("webp", "image/webp"),
    ("woff", "font/woff"),
    ("woff2", "font/woff2"),
];

fn mime_type(extension: &str) -> &str {
    MIME_TYPES
        .iter()
        .find(|(candidate, _)| *candidate == extension)
        .map(|(_, mime)| *mime)
        .unwrap_or("application/octet-stream")
}

/// Serves the embedded SPA. Unknown routes fall back to the app shell, which
/// is how TanStack Start's SPA build is meant to be hosted.
fn serve_asset(app: &AppHandle, relative_path: &str) -> (Vec<u8>, String) {
    let _ = app; // kept for symmetry with future request hooks

    let mut candidates: Vec<&str> = vec![relative_path, "_shell.html"];
    if relative_path == "/" || relative_path.is_empty() {
        candidates = vec!["/_shell.html", "_shell.html"];
    }

    for candidate in candidates.iter().map(|p| p.trim_start_matches('/')) {
        if candidate.contains("..") {
            continue;
        }
        if let Some(bytes) = Assets::get(candidate) {
            let extension = Path::new(candidate)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            return (bytes.data.to_vec(), mime_type(&extension).to_string());
        }
    }

    (b"Not found".to_vec(), "text/plain; charset=utf-8".to_string())
}

fn start_server(app: AppHandle, port: u16) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;

    // One connection at a time is plenty: the renderer loads the bundle once
    // and then only fetches feed proxies through here.
    tauri::async_runtime::spawn_blocking(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || handle_connection(app, stream));
        }
    });

    Ok(())
}

fn handle_connection(app: AppHandle, mut stream: TcpStream) {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];
    // Read only until the end of the headers: bodies are never sent by the
    // renderer to this server.
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buffer.extend_from_slice(&chunk[..n]);
                if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
                if buffer.len() > 64 * 1024 {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let text = String::from_utf8_lossy(&buffer);
    let request_line = text.lines().next().unwrap_or("");
    let raw_path = request_line.split_whitespace().nth(1).unwrap_or("/");
    let parsed = match url::Url::parse(&format!("http://127.0.0.1:{APP_PORT}{raw_path}")) {
        Ok(parsed) => parsed,
        Err(_) => return,
    };

    let mut response_head_status = 200u16;
    let (body, content_type): (Vec<u8>, String) = if parsed.path() == "/proxy" {
        match proxy::fetch_remote(parsed.query()) {
            Ok((status, remote_type, bytes)) => {
                response_head_status = status;
                (bytes, remote_type)
            }
            Err(message) => {
                response_head_status = 502;
                let body = format!("Feed fetch failed: {message}").into_bytes();
                (body, "text/plain; charset=utf-8".to_string())
            }
        }
    } else {
        let relative_path = if parsed.path() == "/" {
            "/".to_string()
        } else {
            parsed.path().to_string()
        };
        let (bytes, mime) = serve_asset(&app, &relative_path);
        if bytes == b"Not found" {
            response_head_status = 404;
        }
        (bytes, mime)
    };

    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        response_head_status,
        if response_head_status == 200 { "OK" } else if response_head_status == 404 { "Not Found" } else { "Bad Gateway" },
        content_type,
        body.len(),
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&body);
    let _ = stream.flush();
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // A second launch should focus the running window instead of
            // fighting over the fixed port.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let handle = app.handle().clone();

            if let Err(error) = start_server(handle.clone(), APP_PORT) {
                use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
                handle
                    .dialog()
                    .message(format!(
                        "Marginalia could not start.\n\nPort {APP_PORT} is already in use \
                         by another program (or another Marginalia copy is starting).\n\n{error}"
                    ))
                    .title("Marginalia could not start")
                    .kind(MessageDialogKind::Error)
                    .buttons(MessageDialogButtons::Ok)
                    .show(|_| std::process::exit(1));
                return Ok(());
            }

            // In `tauri dev` the page comes from the Vite dev server; in the
            // packaged binary it comes from the embedded-SPA server above.
            // dev_url is also populated for plain debug builds, so only trust
            // it when the tauri CLI actually spawned this process.
            let under_tauri_cli = std::env::vars().any(|(name, _)| name.starts_with("TAURI_ENV_"));
            let dev_url = if under_tauri_cli {
                app.config().build.dev_url.clone()
            } else {
                None
            };
            let target = dev_url.unwrap_or_else(|| {
                url::Url::parse(&format!("http://127.0.0.1:{APP_PORT}/")).expect("static loopback url")
            });

            let window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(target.clone()),
            )
            .title(WINDOW_TITLE)
            .inner_size(1440.0, 900.0)
            .min_inner_size(960.0, 640.0)
            .center()
            .visible(false)
            .background_color(tauri::window::Color(0xf7, 0xf5, 0xf2, 0xff))
            // New-tab links are redirected into the same frame, where the
            // navigation hook below sends them to the system browser.
            .initialization_script(
                r#"
                window.open = function (url) { if (url) { location.href = String(url); } return null; };
                // New-tab links (target="_blank") do not go through window.open in
                // WebView2; route them into the same frame so the navigation hook
                // below hands them to the system browser.
                document.addEventListener('click', function (event) {
                    var anchor = event.target && event.target.closest && event.target.closest('a[target="_blank"]');
                    if (anchor && anchor.href) {
                        event.preventDefault();
                        event.stopPropagation();
                        location.href = anchor.href;
                    }
                }, true);
                "#,
            )
            .on_navigation(move |target| {
                let same_origin = target.scheme() == "http"
                    && target.host_str() == Some("127.0.0.1")
                    && target.port() == Some(APP_PORT);
                let is_local = same_origin || dev_like_url(&target);
                if !is_local {
                    let _ = tauri_plugin_opener::OpenerExt::opener(&handle)
                        .open_url(target.as_str(), None::<&str>);
                    return false;
                }
                true
            })
            .on_page_load(|webview, payload| {
                if payload.event() == tauri::webview::PageLoadEvent::Finished {
                    let _ = webview.show();
                }
            });

            window.build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Marginalia");
}

/// The dev server URL can be any localhost port; treat all localhost http
/// origins as trusted in development only (debug builds).
fn dev_like_url(target: &url::Url) -> bool {
    cfg!(debug_assertions)
        && target.host_str() == Some("localhost")
        && target.scheme() == "http"
}
