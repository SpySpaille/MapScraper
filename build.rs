fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("app.ico");
        res.compile().unwrap();
    }
}
