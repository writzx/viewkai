#[test]
fn debug_cwd() {
    let cwd = std::env::current_dir().unwrap();
    eprintln!("Current working directory: {}", cwd.display());
    
    let pdfium_path = std::env::var("PDFIUM_DYLIB_PATH").unwrap_or_else(|_| "NOT SET".to_string());
    eprintln!("PDFIUM_DYLIB_PATH: {}", pdfium_path);
    
    // Try to check if the path exists
    if let Ok(path) = std::env::var("PDFIUM_DYLIB_PATH") {
        let exists = std::path::Path::new(&path).exists();
        eprintln!("Path exists: {}", exists);
    }
}
