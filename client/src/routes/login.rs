use dioxus::prelude::*;
use crate::api::matrix;

#[component]
pub fn Login() -> Element {
    let mut session = use_context::<Signal<Option<String>>>();
    let navigator = use_navigator();
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut is_registration = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        div { class: "login-screen",
            div { class: "login-card",
                div { class: "login-brand", "SEPPYL" }
                h1 { if *is_registration.read() { "Créer un compte" } else { "Connexion" } }
                p { "Suivez et préparez vos entraînements." }
                form {
                    onsubmit: move |event| {
                        event.prevent_default();
                        if *busy.read() { return; }
                        let user = username.read().trim().to_string();
                        let pass = password.read().clone();
                        if user.len() < 3 || pass.len() < 8 && *is_registration.read() {
                            error.set(Some("Pour un nouveau compte : 3 caractères minimum pour l'identifiant, 8 pour le mot de passe.".into()));
                            return;
                        }
                        let registering = *is_registration.read();
                        busy.set(true);
                        error.set(None);
                        spawn(async move {
                            let result = async {
                                if registering {
                                    matrix::register(&user, &pass).await?;
                                }
                                matrix::login(&user, &pass).await
                            }.await;
                            match result {
                                Ok(token) => {
                                    crate::session::save(&token);
                                    session.set(Some(token));
                                    navigator.push(crate::routes::Route::Trainings {});
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
                    input { r#type: "password", value: "{password}", required: true, autocomplete: if *is_registration.read() { "new-password" } else { "current-password" },
                        placeholder: "Votre mot de passe", oninput: move |e| password.set(e.value()) }
                    if let Some(message) = error.read().as_ref() {
                        p { class: "login-error", role: "alert", "{message}" }
                    }
                    button { class: "button solid login-submit", r#type: "submit", disabled: *busy.read(),
                        if *busy.read() { "Veuillez patienter…" } else if *is_registration.read() { "Créer le compte" } else { "Se connecter" }
                    }
                }
                button { class: "registration-toggle",
                    disabled: *busy.read(),
                    onclick: move |_| {
                        error.set(None);
                        is_registration.set(!*is_registration.read());
                    },
                    if *is_registration.read() { "Déjà un compte ? Se connecter" } else { "Créer un nouveau compte" }
                }
                Link { to: crate::routes::Route::PublicDirectory {},
                    class:"public-login-link","Explorer les clubs publics sans compte" }
                if *is_registration.read() {
                    p { class: "registration-note", "Après inscription, votre compte est en lecture seule jusqu’à son rattachement à un club ou une équipe et l’attribution des droits adaptés." }
                }
            }
        }
    }
}
