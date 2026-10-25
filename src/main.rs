mod app;
mod curve;
mod network;
mod rpc;
mod wallet;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    leptos::mount_to_body(app::App);
}
