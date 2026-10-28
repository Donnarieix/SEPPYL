use dioxus::prelude::*;

mod api;
mod routes;
mod session;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(session::restore()));
    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        Router::<routes::Route> {}
    }
}
