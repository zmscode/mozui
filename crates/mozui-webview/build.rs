fn main() {
    println!("cargo:rerun-if-changed=src/bridge.js");

    let source =
        std::fs::read_to_string("src/bridge.js").expect("bridge.js not found");

    // Simple minification: strip single-line comments and collapse whitespace.
    // We avoid a full JS minifier to keep build deps minimal and avoid parser bugs.
    let mut output = String::with_capacity(source.len());
    for line in source.lines() {
        let trimmed = line.trim();
        // Skip blank lines and full-line comments
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        // Strip trailing comments (only when not inside a string)
        let code = strip_trailing_comment(trimmed);
        if !code.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(code);
        }
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = std::path::PathBuf::from(out_dir).join("bridge.min.js");

    let existing = std::fs::read_to_string(&out_path).unwrap_or_default();
    if existing != output {
        std::fs::write(&out_path, &output).expect("failed to write bridge.min.js");
    }
}

/// Strip trailing `// comment` from a line, respecting string literals.
fn strip_trailing_comment(line: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    let mut prev = '\0';
    let bytes = line.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        let ch = b as char;
        match ch {
            '\'' if !in_double && prev != '\\' => in_single = !in_single,
            '"' if !in_single && prev != '\\' => in_double = !in_double,
            '/' if !in_single && !in_double && i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                return line[..i].trim_end();
            }
            _ => {}
        }
        prev = ch;
    }
    line
}
