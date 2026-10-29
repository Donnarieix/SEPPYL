use dioxus::prelude::*;
use crate::api::trainings;
use super::{Route, layout::Header};

#[component]
pub fn Trainings() -> Element {
    let session = use_context::<Signal<Option<String>>>();
    let token = session.read().clone();
    let navigator = use_navigator();
    let fetch_token = token.clone();
    let mut list = use_resource(move || {
        let token = fetch_token.clone();
        async move {
            match token {
                Some(token) => trainings::list(&token).await,
                None => Err("Vous devez vous connecter.".to_string()),
            }
        }
    });
    let mut creating = use_signal(|| false);

    if token.is_none() {
        return rsx! {
            div { class: "centered",
                h1 { "Entraînements" }
                p { "Connectez-vous pour accéder à vos séances." }
                button { class: "button solid", onclick: move |_| { navigator.push(Route::Login {}); }, "Connexion" }
            }
        };
    }
    let token = token.unwrap_or_default();
    rsx! {
        div { class: "app",
            Header {}
            main { class: "page",
                div { class: "page-heading",
                    div {
                        h1 { "Entraînements" }
                        p { "Préparez vos séances et retrouvez les notes prises sur le terrain." }
                    }
                    button { class: "button solid", onclick: move |_| creating.set(true), "+ Nouvel entraînement" }
                }
                match list.read().clone() {
                    Some(Ok(items)) => rsx! {
                        if items.is_empty() {
                            div { class: "empty-state dashboard-empty",
                                h2 { "Aucun entraînement" }
                                p { "Créez votre première séance pour préparer ses activités, ses présences et ses évaluations." }
                                button { class: "button solid", onclick: move |_| creating.set(true), "Créer un entraînement" }
                            }
                        } else {
                            div { class: "training-list",
                                for training in items {
                                    Link {
                                        key: "{training.id}",
                                        class: "training-row",
                                        to: Route::TrainingDetail { id: training.id.clone() },
                                        div {
                                            strong { "{training.name}" }
                                            span { "{training.date.get(..10).unwrap_or(&training.date)}" }
                                        }
                                        span { class: "training-row-arrow", "Ouvrir →" }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(message)) => rsx! {
                        div { class: "load-state", role: "alert",
                            p { "{message}" }
                            button { class: "button outline", onclick: move |_| list.restart(), "Réessayer" }
                        }
                    },
                    None => rsx! { div { class: "load-state", "Chargement des entraînements…" } },
                }
            }
            if *creating.read() {
                NewTraining {
                    token: token.clone(),
                    on_cancel: move |_| creating.set(false),
                    on_created: move |id: String| {
                        creating.set(false);
                        navigator.push(Route::TrainingDetail { id });
                    },
                }
            }
        }
    }
}

#[component]
pub fn NewTraining(
    token: String,
    on_cancel: EventHandler<()>,
    on_created: EventHandler<String>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut date = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    rsx! {
        div { class: "dialog-backdrop",
            div { class: "dialog-panel", role: "dialog", "aria-modal": "true",
                div { class: "dialog-header",
                    div {
                        h2 { "Nouvel entraînement" }
                        p { "Un nom et une date suffisent pour commencer." }
                    }
                    button { class: "icon-button", disabled: *busy.read(), onclick: move |_| on_cancel.call(()), "×" }
                }
                form {
                    onsubmit: move |event| {
                        event.prevent_default();
                        if *busy.read() { return; }
                        let name_value = name.read().trim().to_string();
                        let date_value = date.read().clone();
                        if name_value.is_empty() || date_value.is_empty() {
                            error.set(Some("Indiquez un nom et une date.".into()));
                            return;
                        }
                        busy.set(true);
                        error.set(None);
                        let token = token.clone();
                        spawn(async move {
                            match trainings::create(&token, &name_value, &date_value).await {
                                Ok(id) => on_created.call(id),
                                Err(message) => error.set(Some(message)),
                            }
                            busy.set(false);
                        });
                    },
                    div { class: "dialog-body",
                        div { class: "field",
                            label { "Nom" }
                            input { r#type: "text", value: "{name}", required: true, maxlength: "150",
                                placeholder: "Ex. Entraînement du mardi", oninput: move |e| name.set(e.value()) }
                        }
                        div { class: "field",
                            label { "Date" }
                            input { r#type: "date", value: "{date}", required: true, oninput: move |e| date.set(e.value()) }
                        }
                        if let Some(message) = error.read().as_ref() {
                            p { class: "form-error", role: "alert", "{message}" }
                        }
                    }
                    div { class: "dialog-footer",
                        button { class: "button quiet", r#type: "button", disabled: *busy.read(),
                            onclick: move |_| on_cancel.call(()), "Annuler" }
                        button { class: "button solid", r#type: "submit", disabled: *busy.read(),
                            if *busy.read() { "Création…" } else { "Créer l'entraînement" }
                        }
                    }
                }
            }
        }
    }
}
