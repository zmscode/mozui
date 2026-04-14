use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;

/// Serves local files over the `mozui://` custom protocol.
///
/// All paths are jailed to the asset root. Path traversal attempts return 404.
pub struct AssetServer {
    root: PathBuf,
    mime_overrides: HashMap<String, String>,
    csp: Option<String>,
}

impl AssetServer {
    /// Create a new asset server rooted at the given directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            mime_overrides: HashMap::new(),
            csp: None,
        }
    }

    /// Add a MIME type override for a file extension (without the dot).
    pub fn mime_override(mut self, extension: &str, content_type: &str) -> Self {
        self.mime_overrides
            .insert(extension.to_lowercase(), content_type.to_string());
        self
    }

    /// Set CSP header to inject on HTML responses.
    pub fn with_csp(mut self, csp: impl Into<String>) -> Self {
        self.csp = Some(csp.into());
        self
    }

    /// Serve a request. Returns an HTTP response with the file contents or 404.
    pub fn serve(&self, request: &http::Request<Vec<u8>>) -> http::Response<Cow<'static, [u8]>> {
        let path = request.uri().path();
        let relative = path.trim_start_matches('/');

        // Empty path → serve index.html
        let relative = if relative.is_empty() {
            "index.html"
        } else {
            relative
        };

        // Reject obvious traversal attempts before filesystem access
        if relative.contains("..") {
            return self.not_found();
        }

        // Resolve and canonicalize
        let candidate = self.root.join(relative);
        let resolved = match candidate.canonicalize() {
            Ok(p) => p,
            Err(_) => return self.not_found(),
        };

        // Jail check: resolved path must be under root
        let canonical_root = match self.root.canonicalize() {
            Ok(r) => r,
            Err(_) => return self.not_found(),
        };
        if !resolved.starts_with(&canonical_root) {
            return self.not_found();
        }

        // Don't serve .ts source files (TypeScript sources should not be exposed)
        if let Some(ext) = resolved.extension() {
            if ext == "ts" && !resolved.to_string_lossy().ends_with(".d.ts") {
                return self.not_found();
            }
        }

        // Read file
        let bytes = match std::fs::read(&resolved) {
            Ok(b) => b,
            Err(_) => return self.not_found(),
        };

        let mime = self.detect_mime(&resolved);
        let mut builder = http::Response::builder()
            .header("Content-Type", &mime)
            .header("Cache-Control", "no-cache")
            .header("X-Content-Type-Options", "nosniff");

        // Inject CSP on HTML responses
        if mime.starts_with("text/html") {
            if let Some(csp) = &self.csp {
                builder = builder.header("Content-Security-Policy", csp.as_str());
            }
        }

        builder
            .body(Cow::Owned(bytes))
            .unwrap_or_else(|_| self.not_found())
    }

    fn not_found(&self) -> http::Response<Cow<'static, [u8]>> {
        http::Response::builder()
            .status(404)
            .header("Content-Type", "text/plain")
            .body(Cow::Borrowed(b"Not Found" as &[u8]))
            .unwrap()
    }

    fn detect_mime(&self, path: &std::path::Path) -> String {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check overrides first
        if let Some(mime) = self.mime_overrides.get(&ext) {
            return mime.clone();
        }

        match ext.as_str() {
            "html" | "htm" => "text/html; charset=utf-8",
            "css" => "text/css; charset=utf-8",
            "js" | "mjs" => "application/javascript; charset=utf-8",
            "json" => "application/json; charset=utf-8",
            "wasm" => "application/wasm",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "ico" => "image/x-icon",
            "webp" => "image/webp",
            "avif" => "image/avif",
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "ttf" => "font/ttf",
            "otf" => "font/otf",
            "txt" => "text/plain; charset=utf-8",
            "xml" => "application/xml; charset=utf-8",
            "pdf" => "application/pdf",
            "mp3" => "audio/mpeg",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            _ => "application/octet-stream",
        }
        .to_string()
    }
}
