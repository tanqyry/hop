fn main() {
    slint_build::compile("ui/app.slint").unwrap();
    #[cfg(windows)]
    {
        let mut r = winresource::WindowsResource::new();
        r.set_icon("icon.ico");
        r.set("ProductName", "hop");
        r.set("FileDescription", "hop");
        let _ = r.compile();
    }
}
