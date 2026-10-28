use dioxus::prelude::*;
use crate::api::matrix;

#[component]
pub fn Login() -> Element {
    let mut session = use_context::<Signal<Option<String>>>();
    let navigator = use_navigator();
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        div { class: "login-screen",
            div { class: "login-card",
                div { class: "brand login-brand", span { class: "brand-symbol", "S" } strong { "SEPPYL" } }
                div { class: "eyebrow", "ESPACE DE SUIVI" }
                h1 { "Content de vous revoir" }
                p { "Connectez-vous pour accéder au tableau des résultats." }
                form {
                    onsubmit: move |event| {
                        event.prevent_default();
                        let username_value = username.read().clone();
                        let password_value = password.read().clone();
                        busy.set(true);
                        error.set(None);
                        spawn(async move {
                            match matrix::login(&username_value, &password_value).await {
                                Ok(token) => {
                                    session.set(Some(token));
                                    navigator.push(crate::routes::Route::Home {});
                                }
                                Err(message) => error.set(Some(message)),
                            }
                            busy.set(false);
                        });
                    },
                    label { "Identifiant" }
                    input { r#type: "text", value: "{username}", required: true, autocomplete: "username",
                        placeholder: "Votre identifiant", oninput: move |e| username.set(e.value()) }
                    label { "Mot de passe" }
                    input { r#type: "password", value: "{password}", required: true, autocomplete: "current-password",
                        placeholder: "Votre mot de passe", oninput: move |e| password.set(e.value()) }
                    if let Some(message) = error.read().as_ref() { p { class: "login-error", "{message}" } }
                    button { class: "button primary login-submit", r#type: "submit", disabled: *busy.read(),
                        if *busy.read() { "Connexion…" } else { "Se connecter" }
                    }
                }
            }
        }
    }
}
