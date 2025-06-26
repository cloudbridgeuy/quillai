use std::collections::HashMap;
use std::convert::Infallible;
use std::path::Path;
use tokio::fs;
use warp::http::HeaderValue;
use warp::reply::Response;
use warp::{Filter, Reply};

#[tokio::main]
async fn main() {
    // Change working directory to crates/parchment
    let parchment_dir = std::path::Path::new("crates/parchment");
    if parchment_dir.exists() {
        std::env::set_current_dir(parchment_dir).expect("Failed to change to crates/parchment directory");
        println!("🚀 Starting Parchment WASM Demo Server");
        println!("📁 Changed working directory to: {:?}", std::env::current_dir().unwrap());
    } else {
        println!("🚀 Starting Parchment WASM Demo Server");
        println!("📁 Working directory: {:?}", std::env::current_dir().unwrap());
        println!("⚠️  Warning: crates/parchment directory not found, serving from current directory");
    }
    println!("📁 Serving files from crates/parchment directory");
    println!("📄 Available examples:");
    println!("   - http://localhost:3000/examples/basic-usage.html");
    println!("   - http://localhost:3000/examples/test-suite.html");
    println!("   - http://localhost:3000/examples/benchmark.html");
    println!("   - http://localhost:3000/examples/editor-demo.html");
    println!("⚠️ Make sure you've run 'wasm-pack build' first!");
    println!("🛑 Press Ctrl+C to stop the server\n");

    let mime_types = get_mime_types();

    let files = warp::get()
        .and(warp::path::tail())
        .and_then(move |tail: warp::path::Tail| {
            let mime_types = mime_types.clone();
            let path = tail.as_str().to_string();
            serve_file(path, mime_types)
        });

    let index = warp::get()
        .and(warp::path::end())
        .and_then(|| serve_directory_listing());

    let routes = index.or(files).with(
        warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type"])
            .allow_methods(vec!["GET", "POST", "OPTIONS"]),
    );

    warp::serve(routes).run(([127, 0, 0, 1], 3000)).await;
}

fn get_mime_types() -> HashMap<&'static str, &'static str> {
    let mut mime_types = HashMap::new();
    mime_types.insert("html", "text/html; charset=utf-8");
    mime_types.insert("js", "text/javascript; charset=utf-8");
    mime_types.insert("wasm", "application/wasm");
    mime_types.insert("css", "text/css; charset=utf-8");
    mime_types.insert("json", "application/json; charset=utf-8");
    mime_types.insert("md", "text/markdown; charset=utf-8");
    mime_types.insert("ts", "text/typescript; charset=utf-8");
    mime_types
}

async fn serve_directory_listing() -> Result<impl Reply, Infallible> {
    let html_content = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Parchment WASM Examples</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        h1 { color: #333; }
        .example { margin: 10px 0; padding: 10px; border: 1px solid #ddd; border-radius: 5px; }
        .example a { text-decoration: none; color: #007bff; font-weight: bold; }
        .example a:hover { text-decoration: underline; }
        .description { color: #666; margin-top: 5px; }
    </style>
</head>
<body>
    <h1>🦀 Parchment WASM Examples</h1>
    <p>Choose an example to explore the Rust/WebAssembly implementation of Parchment:</p>
    
    <div class="example">
        <a href="examples/basic-usage.html">📝 Basic Usage</a>
        <div class="description">Demonstrates fundamental TextBlot and ScrollBlot operations, DOM integration, and performance testing.</div>
    </div>
    
    <div class="example">
        <a href="examples/test-suite.html">🧪 Test Suite</a>
        <div class="description">Complete test suite with multiple editor instances, mutation observers, and comprehensive validation.</div>
    </div>
    
    <div class="example">
        <a href="examples/benchmark.html">⚡ Performance Benchmark</a>
        <div class="description">Comprehensive performance testing and benchmarking suite for WASM operations.</div>
    </div>
    
    <div class="example">
        <a href="examples/editor-demo.html">🎛️ Editor Demo</a>
        <div class="description">Interactive rich text editor demonstration with real-time editing capabilities.</div>
    </div>
    
    <div style="margin-top: 30px; color: #666; font-size: 0.9em;">
        <p>📦 <strong>WASM Package:</strong> <a href="pkg/">View generated package files</a></p>
        <p>📚 <strong>Documentation:</strong> <a href="README.md">README.md</a></p>
    </div>
</body>
</html>"#;

    let mut response = Response::new(html_content.into());
    response
        .headers_mut()
        .insert("content-type", HeaderValue::from_static("text/html; charset=utf-8"));
    response
        .headers_mut()
        .insert("access-control-allow-origin", HeaderValue::from_static("*"));
    Ok(response)
}

async fn serve_file(
    path: String,
    mime_types: HashMap<&'static str, &'static str>,
) -> Result<impl Reply, Infallible> {
    // Remove leading slash
    let path = path.strip_prefix('/').unwrap_or(&path).to_string();

    match fs::read(&path).await {
        Ok(contents) => {
            let extension = Path::new(&path)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("txt");

            let mime_type = mime_types
                .get(extension)
                .unwrap_or(&"application/octet-stream");

            let mut response = Response::new(contents.into());
            response
                .headers_mut()
                .insert("content-type", HeaderValue::from_static(mime_type));
            response
                .headers_mut()
                .insert("access-control-allow-origin", HeaderValue::from_static("*"));

            Ok(response)
        }
        Err(_) => {
            let error_body = format!("File not found: {}", path);
            let mut response = Response::new(error_body.into());
            *response.status_mut() = warp::http::StatusCode::NOT_FOUND;
            response
                .headers_mut()
                .insert("content-type", HeaderValue::from_static("text/plain"));
            Ok(response)
        }
    }
}
