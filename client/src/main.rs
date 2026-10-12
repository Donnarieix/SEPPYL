use dioxus::prelude::*;

use crate::api::auth::Api;

mod routes;
mod api;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_context_provider(|| {
        Api::new("http://127.0.0.1".into())
    });

    rsx! {
        Router::<routes::Route> {}
    }
}
