use dioxus::prelude::*;
use crate::pages::NotFound as NotFoundPage;

#[component]
pub fn NotFound(route: Vec<String>) -> Element {
    rsx! {
        NotFoundPage { route: route }
    }
} 