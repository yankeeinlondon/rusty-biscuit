fn main() {
    // Ensure `frontend/dist/` exists so the `RustEmbed` proc-macro compiles
    // even when the frontend hasn't been built yet (`pnpm build`).
    let dist = std::path::Path::new("frontend/dist");
    if !dist.exists() {
        std::fs::create_dir_all(dist).expect("failed to create frontend/dist placeholder");
    }
}
