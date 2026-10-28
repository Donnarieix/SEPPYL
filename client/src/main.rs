use dioxus::prelude::*;

mod api;
mod routes;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(None::<String>));
    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        Router::<routes::Route> {}
    }
}
