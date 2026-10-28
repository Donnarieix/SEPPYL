use dioxus::prelude::*;
use crate::api::{
    account,
    preferences::{self, AttendanceView, Density, Theme, UserPreferences},
};
use super::{Route, layout::Header};
use crate::theme_transition;

#[component]
pub fn AccountPage() -> Element {
    let session = use_context::<Signal<Option<String>>>();
    let mut appearance = use_context::<Signal<UserPreferences>>();
    let navigator = use_navigator();
    let token = session.read().clone();
    let request_token = token.clone();
    let mut account_data = use_resource(move || {
        let token = request_token.clone();
        async move {
            match token {
                Some(token) => account::get(&token).await,
                None => Err("Connectez-vous pour gérer votre compte.".to_owned()),
            }
        }
    });

    let mut username = use_signal(|| None::<String>);
    let mut current_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut password_confirm = use_signal(String::new);
    let mut identity_busy = use_signal(|| false);
    let mut password_busy = use_signal(|| false);
    let mut appearance_busy = use_signal(|| false);
    let mut identity_feedback = use_signal(|| None::<(bool, String)>);
    let mut password_feedback = use_signal(|| None::<(bool, String)>);
    let mut appearance_feedback = use_signal(|| None::<(bool, String)>);
    let mut active_tab = use_signal(|| "appearance".to_string());

    if token.is_none() {
        return rsx! {
            div { class: "centered",
                h1 { "Mon compte" }
                p { "Connectez-vous pour accéder aux paramètres." }
                button { class: "button solid", onclick: move |_| { navigator.push(Route::Login {}); }, "Se connecter" }
            }
        };
    }
    let token = token.unwrap_or_default();
    let prefs = *appearance.read();
    let theme_token_system = token.clone();
    let theme_token_light = token.clone();
    let theme_token_dark = token.clone();
    let density_token_comfortable = token.clone();
    let density_token_compact = token.clone();
    let attendance_grid_token = token.clone();
    let attendance_list_token = token.clone();

    rsx! {
        div { class: "app",
            Header {}
            main { class: "page settings-page",
                div { class: "page-heading",
                    div {
                        h1 { "Mon compte" }
                        p { "Informations personnelles et préférences d'utilisation." }
                    }
                }
                nav { class: "settings-tabs", role: "tablist", "aria-label": "Rubriques du compte",
                    button {
                        class: if active_tab.read().as_str() == "appearance" { "settings-tab active" } else { "settings-tab" },
                        role: "tab", "aria-selected": if active_tab.read().as_str() == "appearance" { "true" } else { "false" },
                        onclick: move |_| active_tab.set("appearance".into()),
                        "Préférences"
                    }
                    button {
                        class: if active_tab.read().as_str() == "profile" { "settings-tab active" } else { "settings-tab" },
                        role: "tab", "aria-selected": if active_tab.read().as_str() == "profile" { "true" } else { "false" },
                        onclick: move |_| active_tab.set("profile".into()),
                        "Informations"
                    }
                    button {
                        class: if active_tab.read().as_str() == "security" { "settings-tab active" } else { "settings-tab" },
                        role: "tab", "aria-selected": if active_tab.read().as_str() == "security" { "true" } else { "false" },
                        onclick: move |_| active_tab.set("security".into()),
                        "Sécurité"
                    }
                }
                div { class: "settings-layout settings-tab-content",
                    if active_tab.read().as_str() == "appearance" {
                    section { class: "settings-panel",
                        div { class: "settings-panel-heading",
                            h2 { "Apparence" }
                            p { "Ces préférences vous suivent sur vos appareils lorsque vous utilisez le même compte." }
                        }
                        div { class: "settings-panel-content",
                            div { class: "settings-group",
                                div { class: "settings-group-label",
                                    strong { "Thème" }
                                    span { "Adaptez les couleurs à votre environnement." }
                                }
                                div { class: "settings-choice-group", role: "group", "aria-label": "Thème",
                                    button {
                                        class: if prefs.theme == Theme::System { "settings-choice selected" } else { "settings-choice" },
                                        "aria-pressed": if prefs.theme == Theme::System { "true" } else { "false" },
                                        disabled: *appearance_busy.read(),
                                        onclick: move |event: MouseEvent| {
                                            let before = *appearance.read();
                                            if before.theme == Theme::System || *appearance_busy.read() { return; }
                                            let coords = event.data.client_coordinates();
                                            appearance_busy.set(true);
                                            appearance_feedback.set(None);
                                            let token = theme_token_system.clone();
                                            spawn(async move {
                                                theme_transition::reveal_theme(appearance, Theme::System, coords.x, coords.y).await;
                                                match preferences::save_appearance(&token, Theme::System, before.density).await {
                                                    Ok(saved) => appearance.set(saved),
                                                    Err(error) => {
                                                        appearance.set(before);
                                                        appearance_feedback.set(Some((false, error)));
                                                    }
                                                }
                                                appearance_busy.set(false);
                                            });
                                        },
                                        span { class: "choice-icon", "◐" }
                                        span { "Système" }
                                    }
                                    button {
                                        class: if prefs.theme == Theme::Light { "settings-choice selected" } else { "settings-choice" },
                                        "aria-pressed": if prefs.theme == Theme::Light { "true" } else { "false" },
                                        disabled: *appearance_busy.read(),
                                        onclick: move |event: MouseEvent| {
                                            let before = *appearance.read();
                                            if before.theme == Theme::Light || *appearance_busy.read() { return; }
                                            let coords = event.data.client_coordinates();
                                            appearance_busy.set(true);
                                            appearance_feedback.set(None);
                                            let token = theme_token_light.clone();
                                            spawn(async move {
                                                theme_transition::reveal_theme(appearance, Theme::Light, coords.x, coords.y).await;
                                                match preferences::save_appearance(&token, Theme::Light, before.density).await {
                                                    Ok(saved) => appearance.set(saved),
                                                    Err(error) => {
                                                        appearance.set(before);
                                                        appearance_feedback.set(Some((false, error)));
                                                    }
                                                }
                                                appearance_busy.set(false);
                                            });
                                        },
                                        span { class: "choice-icon", "☀" }
                                        span { "Clair" }
                                    }
                                    button {
                                        class: if prefs.theme == Theme::Dark { "settings-choice selected" } else { "settings-choice" },
                                        "aria-pressed": if prefs.theme == Theme::Dark { "true" } else { "false" },
                                        disabled: *appearance_busy.read(),
                                        onclick: move |event: MouseEvent| {
                                            let before = *appearance.read();
                                            if before.theme == Theme::Dark || *appearance_busy.read() { return; }
                                            let coords = event.data.client_coordinates();
                                            appearance_busy.set(true);
                                            appearance_feedback.set(None);
                                            let token = theme_token_dark.clone();
                                            spawn(async move {
                                                theme_transition::reveal_theme(appearance, Theme::Dark, coords.x, coords.y).await;
                                                match preferences::save_appearance(&token, Theme::Dark, before.density).await {
                                                    Ok(saved) => appearance.set(saved),
                                                    Err(error) => {
                                                        appearance.set(before);
                                                        appearance_feedback.set(Some((false, error)));
                                                    }
                                                }
                                                appearance_busy.set(false);
                                            });
                                        },
                                        span { class: "choice-icon", "☾" }
                                        span { "Sombre" }
                                    }
                                }
                            }
                            div { class: "settings-group",
                                div { class: "settings-group-label",
                                    strong { "Densité d'affichage" }
                                    span { "L'affichage compact réduit l'espacement des tableaux et des listes." }
                                }
                                div { class: "settings-choice-group", role: "group", "aria-label": "Densité d'affichage",
                                    button {
                                        class: if prefs.density == Density::Comfortable { "settings-choice selected" } else { "settings-choice" },
                                        "aria-pressed": if prefs.density == Density::Comfortable { "true" } else { "false" },
                                        disabled: *appearance_busy.read(),
                                        onclick: move |_| {
                                            let before = *appearance.read();
                                            if before.density == Density::Comfortable { return; }
                                            appearance.set(UserPreferences { density: Density::Comfortable, ..before });
                                            appearance_busy.set(true);
                                            appearance_feedback.set(None);
                                            let token = density_token_comfortable.clone();
                                            spawn(async move {
                                                match preferences::save_appearance(&token, before.theme, Density::Comfortable).await {
                                                    Ok(saved) => appearance.set(saved),
                                                    Err(error) => {
                                                        appearance.set(before);
                                                        appearance_feedback.set(Some((false, error)));
                                                    }
                                                }
                                                appearance_busy.set(false);
                                            });
                                        },
                                        "Confortable"
                                    }
                                    button {
                                        class: if prefs.density == Density::Compact { "settings-choice selected" } else { "settings-choice" },
                                        "aria-pressed": if prefs.density == Density::Compact { "true" } else { "false" },
                                        disabled: *appearance_busy.read(),
                                        onclick: move |_| {
                                            let before = *appearance.read();
                                            if before.density == Density::Compact { return; }
                                            appearance.set(UserPreferences { density: Density::Compact, ..before });
                                            appearance_busy.set(true);
                                            appearance_feedback.set(None);
                                            let token = density_token_compact.clone();
                                            spawn(async move {
                                                match preferences::save_appearance(&token, before.theme, Density::Compact).await {
                                                    Ok(saved) => appearance.set(saved),
                                                    Err(error) => {
                                                        appearance.set(before);
                                                        appearance_feedback.set(Some((false, error)));
                                                    }
                                                }
                                                appearance_busy.set(false);
                                            });
                                        },
                                        "Compact"
                                    }
                                }
                            }
                            div { class: "settings-group",
                                div { class: "settings-group-label",
                                    strong { "Affichage des présences" }
                                    span { "Choisissez la vue affichée par défaut à l’ouverture de l’onglet Présences. Vous pourrez toujours basculer entre grille et liste depuis l’entraînement." }
                                }
                                div { class: "settings-choice-group", role: "group", "aria-label": "Affichage des présences",
                                    button {
                                        class: if prefs.attendance_view == AttendanceView::Grid { "settings-choice selected" } else { "settings-choice" },
                                        "aria-pressed": if prefs.attendance_view == AttendanceView::Grid { "true" } else { "false" },
                                        disabled: *appearance_busy.read(),
                                        onclick: move |_| {
                                            let before = *appearance.read();
                                            if before.attendance_view == AttendanceView::Grid { return; }
                                            appearance.set(UserPreferences { attendance_view: AttendanceView::Grid, ..before });
                                            appearance_busy.set(true);
                                            appearance_feedback.set(None);
                                            let token = attendance_grid_token.clone();
                                            spawn(async move {
                                                if let Err(error) = preferences::save(&token, AttendanceView::Grid).await {
                                                    appearance.set(before);
                                                    appearance_feedback.set(Some((false, error)));
                                                }
                                                appearance_busy.set(false);
                                            });
                                        },
                                        "Grille"
                                    }
                                    button {
                                        class: if prefs.attendance_view == AttendanceView::List { "settings-choice selected" } else { "settings-choice" },
                                        "aria-pressed": if prefs.attendance_view == AttendanceView::List { "true" } else { "false" },
                                        disabled: *appearance_busy.read(),
                                        onclick: move |_| {
                                            let before = *appearance.read();
                                            if before.attendance_view == AttendanceView::List { return; }
                                            appearance.set(UserPreferences { attendance_view: AttendanceView::List, ..before });
                                            appearance_busy.set(true);
                                            appearance_feedback.set(None);
                                            let token = attendance_list_token.clone();
                                            spawn(async move {
                                                if let Err(error) = preferences::save(&token, AttendanceView::List).await {
                                                    appearance.set(before);
                                                    appearance_feedback.set(Some((false, error)));
                                                }
                                                appearance_busy.set(false);
                                            });
                                        },
                                        "Liste"
                                    }
                                }
                            }
                            if let Some((success, message)) = appearance_feedback.read().as_ref() {
                                p { class: if *success { "settings-feedback success" } else { "settings-feedback error" }, role: "status", "{message}" }
                            }
                        }
                    }
                    }
                    if active_tab.read().as_str() == "profile" {
                    section { class: "settings-panel",
                        div { class: "settings-panel-heading",
                            h2 { "Informations du compte" }
                            p { "Gérez l'identifiant utilisé pour vous connecter à SEPPYL." }
                        }
                        match account_data.read().clone() {
                            Some(Ok(account)) => rsx! {
                                form {
                                    class: "settings-panel-content",
                                    onsubmit: {
                                        let token = token.clone();
                                        let current_username = account.username.clone();
                                        move |event| {
                                            event.prevent_default();
                                            if *identity_busy.read() { return; }
                                            let next = username.read().clone().unwrap_or_else(|| current_username.clone());
                                            let token = token.clone();
                                            identity_busy.set(true);
                                            identity_feedback.set(None);
                                            spawn(async move {
                                                match account::update_username(&token, next.trim()).await {
                                                    Ok(saved) => {
                                                        username.set(Some(saved.username));
                                                        identity_feedback.set(Some((true, "Identifiant enregistré.".into())));
                                                        account_data.restart();
                                                    }
                                                    Err(error) => identity_feedback.set(Some((false, error))),
                                                }
                                                identity_busy.set(false);
                                            });
                                        }
                                    },
                                    div { class: "field",
                                        label { "Identifiant" }
                                        input { r#type: "text", minlength: "3", maxlength: "80", required: true,
                                            autocomplete: "username",
                                            value: "{username.read().clone().unwrap_or_else(|| account.username.clone())}",
                                            oninput: move |event| username.set(Some(event.value())) }
                                    }
                                    div { class: "settings-account-meta",
                                        span { "Connexion" }
                                        strong { "Compte local SEPPYL" }
                                    }
                                    if let Some((success, message)) = identity_feedback.read().as_ref() {
                                        p { class: if *success { "settings-feedback success" } else { "settings-feedback error" }, role: "status", "{message}" }
                                    }
                                    button { class: "button solid", r#type: "submit", disabled: *identity_busy.read(),
                                        if *identity_busy.read() { "Enregistrement…" } else { "Enregistrer l'identifiant" }
                                    }
                                }
                            },
                            Some(Err(error)) => rsx! {
                                div { class: "settings-panel-content",
                                    p { class: "settings-feedback error", role: "alert", "{error}" }
                                    button { class: "button outline", onclick: move |_| account_data.restart(), "Réessayer" }
                                }
                            },
                            None => rsx! { div { class: "settings-panel-content", "Chargement du compte…" } },
                        }
                    }
                    }
                    if active_tab.read().as_str() == "security" {
                    section { class: "settings-panel",
                        div { class: "settings-panel-heading",
                            h2 { "Sécurité" }
                            p { "Changez le mot de passe de votre compte local." }
                        }
                        form {
                            class: "settings-panel-content",
                            onsubmit: {
                                let token = token.clone();
                                move |event| {
                                    event.prevent_default();
                                    if *password_busy.read() { return; }
                                    let old = current_password.read().clone();
                                    let new = new_password.read().clone();
                                    let confirmation = password_confirm.read().clone();
                                    if new != confirmation {
                                        password_feedback.set(Some((false, "Les nouveaux mots de passe ne correspondent pas.".into())));
                                        return;
                                    }
                                    if new.len() < 12 {
                                        password_feedback.set(Some((false, "Utilisez au moins 12 caractères.".into())));
                                        return;
                                    }
                                    let token = token.clone();
                                    password_busy.set(true);
                                    password_feedback.set(None);
                                    spawn(async move {
                                        match account::change_password(&token, &old, &new).await {
                                            Ok(()) => {
                                                current_password.set(String::new());
                                                new_password.set(String::new());
                                                password_confirm.set(String::new());
                                                password_feedback.set(Some((true, "Mot de passe modifié.".into())));
                                            }
                                            Err(error) => password_feedback.set(Some((false, error))),
                                        }
                                        password_busy.set(false);
                                    });
                                }
                            },
                            div { class: "field",
                                label { "Mot de passe actuel" }
                                input { r#type: "password", required: true, autocomplete: "current-password",
                                    value: "{current_password}",
                                    oninput: move |event| current_password.set(event.value()) }
                            }
                            div { class: "field",
                                label { "Nouveau mot de passe" }
                                input { r#type: "password", required: true, minlength: "12",
                                    autocomplete: "new-password", value: "{new_password}",
                                    oninput: move |event| new_password.set(event.value()) }
                            }
                            div { class: "field",
                                label { "Confirmer le nouveau mot de passe" }
                                input { r#type: "password", required: true, minlength: "12",
                                    autocomplete: "new-password", value: "{password_confirm}",
                                    oninput: move |event| password_confirm.set(event.value()) }
                            }
                            if let Some((success, message)) = password_feedback.read().as_ref() {
                                p { class: if *success { "settings-feedback success" } else { "settings-feedback error" }, role: "status", "{message}" }
                            }
                            button { class: "button solid", r#type: "submit", disabled: *password_busy.read(),
                                if *password_busy.read() { "Modification…" } else { "Changer le mot de passe" }
                            }
                            p { class: "settings-note", "Votre session actuelle et les autres sessions déjà ouvertes restent valides jusqu'à leur expiration. La gestion centralisée des sessions sera réalisée avec le futur SSO." }
                        }
                    }
                    }
                }
                p { class: "settings-footnote",
                    "Ces réglages sont associés à votre compte. La gestion des identités sera transférée au SSO Donnarieix lorsqu'il sera disponible."
                }
            }
        }
    }
}
