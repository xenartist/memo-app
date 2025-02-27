use dioxus::prelude::*;
use dioxus_router::prelude::*;

// Define application routes
#[derive(Routable, Clone)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/about")]
    About {},
    #[route("/settings")]
    Settings {},
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

// Home page component
pub fn Home() -> Element {
    let mut count = use_signal(|| 0);
    
    rsx! {
        div {
            class: "container",
            h1 { "Home Page" }
            p { "Welcome to the cross-platform Dioxus app!" }
            div {
                class: "counter",
                p { "Count: {count}" }
                button {
                    onclick: move |_| count += 1,
                    "Increment"
                }
                button {
                    onclick: move |_| count -= 1,
                    "Decrement"
                }
            }
            nav {
                Link { to: Route::About {}, "Go to About" }
                Link { to: Route::Settings {}, "Go to Settings" }
            }
        }
    }
}

// About page component
pub fn About() -> Element {
    rsx! {
        div {
            class: "container",
            h1 { "About Page" }
            p { "This is a cross-platform application built with Rust and Dioxus." }
            p { "It can run on Windows, Linux, macOS, iOS, Android, and the Web!" }
            nav {
                Link { to: Route::Home {}, "Back to Home" }
            }
        }
    }
}

// Settings page component
pub fn Settings() -> Element {
    rsx! {
        div {
            class: "container",
            h1 { "Settings Page" }
            p { "Here you would configure app settings." }
            nav {
                Link { to: Route::Home {}, "Back to Home" }
            }
        }
    }
}

// 404 page component
#[component]
pub fn NotFound(route: Vec<String>) -> Element {
    rsx! {
        div {
            class: "container",
            h1 { "Page Not Found" }
            p { "We couldn't find the page: {route:?}" }
            nav {
                Link { to: Route::Home {}, "Back to Home" }
            }
        }
    }
} 