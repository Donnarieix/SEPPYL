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
    // The server preference supplies the initial DOM theme. During a click,
    // JavaScript manages data-theme for the View Transition; the Dioxus
    // signal still updates the selected button immediately.
    let initial_theme = preferences.read().clone()
        .and_then(Result::ok)
        .unwrap_or_default()
        .theme;
    rsx! {
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }
        document::Stylesheet { href: asset!("/assets/main.css") }
        document::Script { src: asset!("/assets/theme-transition.js") }
        document::Script { src: asset!("/assets/auth-guard.js") }
        document::Script { src: asset!("/assets/audit-dates.js") }
        document::Script { src: asset!("/assets/rich-editor.js") }
        document::Script { src: asset!("/assets/tactics-board.js") }
        document::Script { src: asset!("/assets/tactics-fullscreen.js") }
        document::Script { src: asset!("/assets/select-overlay.js") }
        document::Script { src: asset!("/assets/logo-crop.js") }
        document::Script { src: asset!("/assets/toast.js") }
        div {
            class: "site-root",
            "data-theme": initial_theme.as_str(),
            "data-density": current.density.as_str(),
            Router::<routes::Route> {}
        }
    }
}
