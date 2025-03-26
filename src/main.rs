mod components;

use dioxus::prelude::*;
use crate::components::login::*;

#[component]
fn App() -> Element {
    rsx! {
        LoginPage {}
    }
}

fn main() {
    dioxus::launch(App);
}