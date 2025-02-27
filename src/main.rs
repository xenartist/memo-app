#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use dioxus_web::launch::launch_cfg;

mod router;
mod wallet;
mod components;
mod storage;
mod config;
mod rpc;
mod encrypt;

use router::{Route, Home, NotFound};

// Initialize logging based on platform
#[cfg(feature = "web")]
use wasm_logger;

#[cfg(feature = "simple-logger")]
use simple_logger;

fn main() {
    // Initialize different logging systems based on target platform
    #[cfg(feature = "web")]
    wasm_logger::init(wasm_logger::Config::default());
    
    #[cfg(feature = "simple-logger")]
    simple_logger::init_with_level(log::Level::Debug).expect("Failed to initialize logger");

    // Launch the app using the appropriate platform
    #[cfg(feature = "web")]
    launch_cfg(App, dioxus_web::Config::default());
    
    #[cfg(feature = "desktop")]
    dioxus_desktop::launch_cfg(
        App,
        dioxus_desktop::Config::new().with_window(
            dioxus_desktop::WindowBuilder::new()
                .with_title("My Cross Platform App")
                .with_inner_size(dioxus_desktop::LogicalSize::new(800.0, 600.0))
        )
    );
    
    #[cfg(feature = "mobile")]
    dioxus_mobile::launch(App);
}

// Main application component
fn App() -> Element {
    rsx! {
        style { {include_str!("styles.css")} }
        Router::<Route> {}
    }
} 