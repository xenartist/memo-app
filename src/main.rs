mod components;

use dioxus::prelude::*;
use crate::components::login::*;

// static MAIN_CSS: Asset = asset!("/assets/main.css");
// static LOGIN_CSS: Asset = asset!("/assets/login.css");

#[component]
fn App() -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: asset!("/assets/main.css")
        }
        document::Link {
            rel: "stylesheet",
            href: asset!("/assets/login.css")
        }
        LoginPage {}
    }
}

fn main() {
    dioxus::launch(App);
}