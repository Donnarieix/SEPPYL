use dioxus::prelude::*;
use api::preferences::{self, UserPreferences};

mod api;
mod routes;
mod session;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let session = use_context_provider(|| Signal::new(session::restore()));
    let mut appearance = use_context_provider(|| Signal::new(UserPreferences::default()));
    let token = session.read().clone();
    let preferences = use_resource(move || {
        let token = token.clone();
        async move {
            match token {
                Some(token) => preferences::get(&token).await,
                None => Ok(UserPreferences::default()),
            }
        }
    });

    // Server is authoritative: whenever an account is restored or switched,
    // use its own saved appearance, not the previous account's local settings.
    use_effect(move || {
        if let Some(Ok(saved)) = preferences.read().clone() {
            appearance.set(saved);
        }
    });

    let current = *appearance.read();
    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        div {
            class: "site-root",
            "data-theme": current.theme.as_str(),
            "data-density": current.density.as_str(),
            Router::<routes::Route> {}
        }
    }
}
