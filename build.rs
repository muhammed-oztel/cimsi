fn main() {
    #[cfg(target_os = "windows")]
    {
        winres::WindowsResource::new()
            .set_icon("assets/icons/app-icon.ico")
            .compile()
            .expect("failed to embed Windows exe icon");
    }
}
