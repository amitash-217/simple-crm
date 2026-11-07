// The binary embeds `web/dist`. Make sure the folder has an index page, so `cargo build` and `cargo test`
// work on a fresh checkout before the frontend is built. A real `npm run build` replaces this page.
use std::{fs, path::Path};

const PLACEHOLDER: &str = "<!doctype html><meta charset=\"utf-8\"><title>CRM</title>\
<p>The frontend is not built. Run <code>cd web &amp;&amp; npm install &amp;&amp; npm run build</code>, then rebuild.</p>";

fn main() {
    let index = Path::new("web/dist/index.html");
    if !index.exists() {
        fs::create_dir_all("web/dist").expect("create web/dist");
        fs::write(index, PLACEHOLDER).expect("write placeholder index.html");
    }
    println!("cargo:rerun-if-changed=web/dist");
    println!("cargo:rerun-if-changed=build.rs");
}
