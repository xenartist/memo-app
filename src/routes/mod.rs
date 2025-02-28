use dioxus_router::prelude::*;
use dioxus::prelude::*;

// Define application routes
#[derive(Routable, Clone)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

// Re-export pages
pub mod home;
pub mod not_found;

pub use home::Home;
pub use not_found::NotFound; 