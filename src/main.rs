mod app;
mod core;
mod login;
mod pages;

use app::*;
use leptos::mount_to_body;
use leptos::view;

fn main() {
    console_error_panic_hook::set_once();

    wasm_logger::init(wasm_logger::Config::default());

    log::info!("Starting Memo App");

    mount_to_body(|| {
        view! {
            <App/>
        }
    })
}
