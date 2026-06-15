fn main() {
    // 僅在 Windows 目標平台上編譯資源資訊
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        // 設置應用程式圖示
        res.set_icon("icons/icon.ico");
        // 設置檔案詳細屬性
        res.set("ProductName", "Agnes AI");
        res.set("FileDescription", "Agnes AI High-Defense Desktop Engine");
        res.set("CompanyName", "masteryee-labs");
        res.set("LegalCopyright", "Copyright © 2026 masteryee-labs");
        // 從 Cargo.toml 讀取版本號並寫入
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        
        if let Err(e) = res.compile() {
            eprintln!("Failed to compile Windows resources: {}", e);
            std::process::exit(1);
        }
    }
}
