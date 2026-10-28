use dioxus::prelude::*;
use crate::api::matrix;

#[component]
pub fn PlayerDialog(
    token: String,
    on_cancel: EventHandler<()>,
    on_created: EventHandler<()>,
) -> Element {
    let mut firstname = use_signal(String::new);
    let mut lastname = use_signal(String::new);
    let mut number = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        div { class: "dialog-backdrop",
            div { class: "dialog-panel", role: "dialog", "aria-modal": "true",
                div { class: "dialog-header",
                    div {
                        h2 { "Ajouter une joueuse" }
                        p { "Elle apparaîtra sur une nouvelle ligne du tableau." }
                    }
                    button { class: "icon-button", r#type: "button", "aria-label": "Fermer",
                        disabled: *busy.read(),
                        onclick: move |_| on_cancel.call(()),
                        "×"
                    }
                }
                form {
                    onsubmit: move |event| {
                        event.prevent_default();
                        if *busy.read() { return; }
                        let first = firstname.read().trim().to_owned();
                        let last = lastname.read().trim().to_owned();
                        let number_value = number.read().trim().parse::<u32>();
                        if first.is_empty() || last.is_empty() {
                            error.set(Some("Renseignez le prénom et le nom.".into()));
                            return;
                        }
                        let Ok(number_value) = number_value else {
                            error.set(Some("Le numéro doit être un entier positif ou nul.".into()));
                            return;
                        };
                        let token = token.clone();
                        busy.set(true);
                        error.set(None);
                        spawn(async move {
                            match matrix::create_player(&token, &first, &last, number_value).await {
                                Ok(()) => on_created.call(()),
                                Err(message) => error.set(Some(message)),
                            }
                            busy.set(false);
                        });
                    },
                    div { class: "dialog-body",
                        div { class: "field-grid",
                            div { class: "field",
                                label { "Prénom" }
                                input { r#type: "text", placeholder: "Ex. Camille",
                                    value: "{firstname}", maxlength: "80", required: true,
                                    oninput: move |e| firstname.set(e.value()) }
                            }
                            div { class: "field",
                                label { "Nom" }
                                input { r#type: "text", placeholder: "Ex. Martin",
                                    value: "{lastname}", maxlength: "80", required: true,
                                    oninput: move |e| lastname.set(e.value()) }
                            }
                        }
                        div { class: "field",
                            label { "Numéro" }
                            input { r#type: "number", min: "0", max: "9999", step: "1",
                                placeholder: "Ex. 12", value: "{number}", required: true,
                                oninput: move |e| number.set(e.value()) }
                            small { "Utilisé pour classer les joueuses dans le tableau." }
                        }
                        if let Some(message) = error.read().as_ref() {
                            p { class: "form-error", role: "alert", "{message}" }
                        }
                    }
                    div { class: "dialog-footer",
                        button { class: "button quiet", r#type: "button",
                            disabled: *busy.read(), onclick: move |_| on_cancel.call(()), "Annuler" }
                        button { class: "button solid", r#type: "submit", disabled: *busy.read(),
                            if *busy.read() { "Création…" } else { "Ajouter la joueuse" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ActivityDialog(
    token: String,
    on_cancel: EventHandler<()>,
    on_created: EventHandler<()>,
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
                        h2 { "Nouvelle activité" }
                        p { "Une colonne sera ajoutée pour toutes les joueuses." }
                    }
                    button { class: "icon-button", r#type: "button", "aria-label": "Fermer",
                        disabled: *busy.read(),
                        onclick: move |_| on_cancel.call(()),
                        "×"
                    }
                }
                form {
                    onsubmit: move |event| {
                        event.prevent_default();
                        if *busy.read() { return; }
                        let name_value = name.read().trim().to_owned();
                        let date_value = date.read().trim().to_owned();
                        if name_value.is_empty() || date_value.is_empty() {
                            error.set(Some("Renseignez un nom et une date.".into()));
                            return;
                        }
                        let token = token.clone();
                        busy.set(true);
                        error.set(None);
                        spawn(async move {
                            match matrix::create_activity(&token, &name_value, &date_value).await {
                                Ok(()) => on_created.call(()),
                                Err(message) => error.set(Some(message)),
                            }
                            busy.set(false);
                        });
                    },
                    div { class: "dialog-body",
                        div { class: "field",
                            label { "Nom de l'activité" }
                            input { r#type: "text", placeholder: "Ex. Entraînement du mercredi",
                                maxlength: "120", required: true,
                                value: "{name}", oninput: move |e| name.set(e.value()) }
                        }
                        div { class: "field",
                            label { "Date" }
                            input { r#type: "date", required: true,
                                value: "{date}", oninput: move |e| date.set(e.value()) }
                        }
                        p { class: "field-hint", "Les résultats pourront être renseignés directement dans les cellules." }
                        if let Some(message) = error.read().as_ref() {
                            p { class: "form-error", role: "alert", "{message}" }
                        }
                    }
                    div { class: "dialog-footer",
                        button { class: "button quiet", r#type: "button", disabled: *busy.read(),
                            onclick: move |_| on_cancel.call(()), "Annuler" }
                        button { class: "button solid", r#type: "submit", disabled: *busy.read(),
                            if *busy.read() { "Création…" } else { "Créer l'activité" }
                        }
                    }
                }
            }
        }
    }
}
