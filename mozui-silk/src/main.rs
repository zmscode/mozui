fn main() {
    let project_root = std::path::PathBuf::from(
        std::env::var("SILK_PROJECT")
            .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/silk-app").to_string()),
    );
    silk_runtime::boot(project_root);
}
