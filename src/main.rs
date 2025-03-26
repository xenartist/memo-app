mod components;

use dioxus::prelude::*;
use crate::components::login::*;

static MAIN_CSS: Asset = asset!("assets/main.css");
static LOGIN_CSS: Asset = asset!("assets/login.css");

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        document::Stylesheet { href: LOGIN_CSS }
        LoginPage {}
    }
}

fn main() {
    dioxus::launch(App);
}