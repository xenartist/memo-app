use dioxus::prelude::*;
use crate::pages::Home as HomePage;

#[component]
pub fn Home() -> Element {
    rsx! {
        HomePage {}
    }
} 