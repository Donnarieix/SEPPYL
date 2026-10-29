use dioxus::prelude::*;
use crate::api::auth::Api;
mod routes;
mod api;
fn main() { dioxus::launch(App); }
#[component]
fn App() -> Element {
    use_context_provider(|| Api::new(if cfg!(debug_assertions) {
        "http://127.0.0.1:3000".into()
    } else {
        "/api".into()
    }));
    rsx! { Router::<routes::Route> {} }
}
