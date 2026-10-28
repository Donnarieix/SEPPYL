use dioxus::prelude::*;
use std::collections::BTreeMap;

use crate::api::trainings::{self, Activity, Detail, Material};
use super::{Route, dialogs::PlayerDialog, layout::Header};

#[component]
pub fn TrainingDetail(id: String) -> Element {
    let session = use_context::<Signal<Option<String>>>();
    let navigator = use_navigator();
    let token = session.read().clone();
    let fetch_token = token.clone();
    let fetch_id = id.clone();
    let mut data = use_resource(move || {
        let token = fetch_token.clone();
        let id = fetch_id.clone();
        async move {
            match token {
                Some(token) => trainings::get(&token, &id).await,
                None => Err("Connectez-vous pour consulter cet entraînement.".into()),
            }
        }
    });
    let mut tab = use_signal(|| "presence".to_string());
    let selected_activity = use_signal(String::new);
    let mut selected_override = use_signal(|| None::<Vec<String>>);
    let mut notice = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let mut adding_player = use_signal(|| false);

    if token.is_none() {
        return rsx! {
            div { class: "centered",
                p { "Veuillez vous connecter." }
                button { class: "button solid", onclick: move |_| { navigator.push(Route::Login {}); }, "Connexion" }
            }
        };
    }
    let token = token.unwrap_or_default();
    rsx! {
        div { class: "app",
            Header {}
            main { class: "page training-page",
                Link { to: Route::Trainings {}, class: "back-link", "← Tous les entraînements" }

                match data.read().clone() {
                    Some(Ok(detail)) => rsx! {
                        div { class: "page-heading training-heading",
                            div {
                                h1 { "{detail.training.name}" }
                                p { "{detail.training.date.get(..10).unwrap_or(&detail.training.date)}" }
                            }
                            Link { to: Route::TrainingPrint { id: id.clone() }, class: "button outline print-link",
                                "Fiche imprimable ↗"
                            }
                        }

                        div { class: "tabs", role: "tablist", "aria-label": "Étapes de l'entraînement",
                            button { class: if *tab.read() == "presence" { "tab active" } else { "tab" },
                                onclick: move |_| tab.set("presence".into()), "1. Présences" }
                            button { class: if *tab.read() == "notes" { "tab active" } else { "tab" },
                                onclick: move |_| tab.set("notes".into()), "2. Notes" }
                            button { class: if *tab.read() == "preparation" { "tab active" } else { "tab" },
                                onclick: move |_| tab.set("preparation".into()), "3. Préparation" }
                        }

                        if let Some(message) = notice.read().as_ref() {
                            div { class: "inline-notice", role: "status",
                                span { "{message}" }
                                button { onclick: move |_| notice.set(None), "×" }
                            }
                        }
                        if *tab.read() == "presence" {
                            section { class: "panel",
                                div { class: "panel-head",
                                    div {
                                        h2 { "Qui est présente ?" }
                                        p { "Cochez les joueuses présentes à cet entraînement." }
                                    }
                                    button { class: "button outline small", onclick: move |_| adding_player.set(true),
                                        "+ Joueuse"
                                    }
                                }
                                if detail.players.is_empty() {
                                    div { class: "panel-empty", "Ajoutez d'abord les joueuses de l'équipe." }
                                } else {
                                    div { class: "attendance-list",
                                        for player in &detail.players {
                                            {
                                                let player_id = player.id.clone();
                                                let current = selected_override.read().clone()
                                                    .unwrap_or_else(|| detail.attendance.iter()
                                                        .filter(|a| a.present)
                                                        .map(|a| a.player_id.clone()).collect());
                                                let checked = current.contains(&player_id);
                                                let label = format!("{} {}", player.firstname, player.lastname);
                                                rsx! {
                                                    label { class: "attendance-row", key: "{player.id}",
                                                        input { r#type: "checkbox", checked,
                                                            disabled: *busy.read(),
                                                            onchange: move |event| {
                                                                let mut next = current.clone();
                                                                if event.checked() {
                                                                    if !next.contains(&player_id) { next.push(player_id.clone()); }
                                                                } else {
                                                                    next.retain(|id| id != &player_id);
                                                                }
                                                                selected_override.set(Some(next));
                                                            },
                                                        }
                                                        span { class: "shirt-number", "{player.number}" }
                                                        strong { "{label}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    div { class: "panel-footer",
                                        button { class: "button solid", disabled: *busy.read(),
                                            onclick: {
                                                let id = id.clone();
                                                let token = token.clone();
                                                let chosen = selected_override.read().clone().unwrap_or_else(|| {
                                                    detail.attendance.iter().filter(|a| a.present)
                                                        .map(|a| a.player_id.clone()).collect()
                                                });
                                                move |_| {
                                                    if *busy.read() { return; }
                                                    let id = id.clone();
                                                    let token = token.clone();
                                                    let chosen = chosen.clone();
                                                    busy.set(true);
                                                    spawn(async move {
                                                        match trainings::save_attendance(&token, &id, &chosen).await {
                                                            Ok(()) => {
                                                                selected_override.set(None);
                                                                notice.set(Some("Présences enregistrées.".into()));
                                                                data.restart();
                                                                tab.set("notes".into());
                                                            }
                                                            Err(err) => notice.set(Some(err)),
                                                        }
                                                        busy.set(false);
                                                    });
                                                }
                                            },
                                            if *busy.read() { "Enregistrement…" } else { "Enregistrer et noter →" }
                                        }
                                    }
                                }
                            }
                        } else if *tab.read() == "notes" {
                            Notes {
                                detail: detail.clone(),
                                token: token.clone(),
                                training_id: id.clone(),
                                selected_activity,
                                on_saved: move |_| { notice.set(Some("Note enregistrée.".into())); data.restart(); },
                                on_error: move |message| notice.set(Some(message)),
                            }
                        } else {
                            Preparation {
                                detail: detail.clone(),
                                token: token.clone(),
                                training_id: id.clone(),
                                on_saved: move |_| { notice.set(Some("Modifications enregistrées.".into())); data.restart(); },
                                on_error: move |message| notice.set(Some(message)),
                            }
                        }
                    },
                    Some(Err(message)) => rsx! {
                        div { class: "load-state", role: "alert",
                            p { "{message}" }
                            button { class: "button outline", onclick: move |_| data.restart(), "Réessayer" }
                        }
                    },
                    None => rsx! { div { class: "load-state", "Chargement de l'entraînement…" } },
                }
            }
            if *adding_player.read() {
                PlayerDialog {
                    token: token.clone(),
                    on_cancel: move |_| adding_player.set(false),
                    on_created: move |_| { adding_player.set(false); data.restart(); },
                }
            }
        }
    }
}

#[component]
fn Notes(
    detail: Detail,
    token: String,
    training_id: String,
    mut selected_activity: Signal<String>,
    on_saved: EventHandler<()>,
    on_error: EventHandler<String>,
) -> Element {
    let mut saving = use_signal(|| None::<String>);
    let present: Vec<_> = detail.players.iter()
        .filter(|p| detail.attendance.iter().any(|a| a.player_id == p.id && a.present))
        .collect();
    let activity = selected_activity.read().clone();

    rsx! {
        section { class: "panel",
            div { class: "panel-head",
                div {
                    h2 { "Noter les présentes" }
                    p { "Évaluez chaque joueuse entre 0 et 5. Les notes sont sauvegardées immédiatement." }
                }
            }
            if present.is_empty() {
                div { class: "panel-empty",
                    "Aucune joueuse présente. Commencez par l'onglet Présences."
                }
            } else {
                div { class: "score-toolbar",
                    label { "Type de note" }
                    select { value: "{selected_activity}",
                        onchange: move |event| selected_activity.set(event.value()),
                        option { value: "", "Note générale" }
                        for item in &detail.activities {
                            option { value: "{item.id}", "{item.name}" }
                        }
                    }
                }
                div { class: "score-list",
                    for player in present {
                        {
                            let player_id = player.id.clone();
                            let key = format!("{}|{}", player_id, activity);
                            let score = if activity.is_empty() {
                                detail.attendance.iter().find(|a| a.player_id == player_id).and_then(|a| a.score)
                            } else {
                                detail.scores.iter().find(|s| s.player_id == player_id && s.activity_id == activity).map(|s| s.score)
                            };
                            let token = token.clone();
                            let training_id = training_id.clone();
                            let activity_id = if activity.is_empty() { None } else { Some(activity.clone()) };
                            rsx! {
                                div { class: "score-row", key: "{key}",
                                    div { class: "score-person",
                                        span { class: "shirt-number", "{player.number}" }
                                        strong { "{player.firstname} {player.lastname}" }
                                    }
                                    div { class: "rating", "aria-label": "Note sur 5",
                                        for value in 0..=5 {
                                            button {
                                                key: "{value}",
                                                class: if score == Some(value) { "rating-button selected" } else { "rating-button" },
                                                disabled: saving.read().is_some(),
                                                onclick: {
                                                    let token = token.clone();
                                                    let training_id = training_id.clone();
                                                    let player_id = player_id.clone();
                                                    let activity_id = activity_id.clone();
                                                    let key = key.clone();
                                                    move |_| {
                                                        saving.set(Some(key.clone()));
                                                        let token = token.clone();
                                                        let training_id = training_id.clone();
                                                        let player_id = player_id.clone();
                                                        let activity_id = activity_id.clone();
                                                        spawn(async move {
                                                            match trainings::save_score(&token, &training_id, &player_id, activity_id.as_deref(), Some(value)).await {
                                                                Ok(()) => on_saved.call(()),
                                                                Err(err) => on_error.call(err),
                                                            }
                                                            saving.set(None);
                                                        });
                                                    }
                                                },
                                                "{value}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Preparation(
    detail: Detail,
    token: String,
    training_id: String,
    on_saved: EventHandler<()>,
    on_error: EventHandler<String>,
) -> Element {
    let mut adding = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut image = use_signal(String::new);
    let mut busy = use_signal(|| false);

    let mut totals = BTreeMap::<String, i32>::new();
    for material in &detail.materials {
        *totals.entry(material.name.trim().to_string()).or_default() += material.quantity;
    }

    rsx! {
        div { class: "preparation-layout",
            section { class: "panel preparation-main",
                div { class: "panel-head",
                    div {
                        h2 { "Activités de la séance" }
                        p { "Préparez les exercices et leur matériel avant de rejoindre le terrain." }
                    }
                    button { class: "button outline small", onclick: move |_| adding.set(!*adding.read()), "+ Activité" }
                }
                if *adding.read() {
                    form { class: "editor-form",
                        onsubmit: move |event| {
                            event.prevent_default();
                            if *busy.read() { return; }
                            let name_value = name.read().trim().to_string();
                            if name_value.is_empty() { on_error.call("Donnez un nom à l'activité.".into()); return; }
                            let description_value = description.read().clone();
                            let image_value = image.read().trim().to_string();
                            let token = token.clone();
                            let id = training_id.clone();
                            busy.set(true);
                            spawn(async move {
                                match trainings::add_activity(
                                    &token, &id, &name_value, &description_value,
                                    if image_value.is_empty() { None } else { Some(image_value.as_str()) },
                                ).await {
                                    Ok(()) => {
                                        name.set(String::new());
                                        description.set(String::new());
                                        image.set(String::new());
                                        adding.set(false);
                                        on_saved.call(());
                                    }
                                    Err(err) => on_error.call(err),
                                }
                                busy.set(false);
                            });
                        },
                        div { class: "field",
                            label { "Nom de l'activité" }
                            input { value: "{name}", required: true, maxlength: "150",
                                placeholder: "Ex. Passes en mouvement",
                                oninput: move |e| name.set(e.value()) }
                        }
                        div { class: "field",
                            label { "Déroulement et consignes" }
                            textarea { value: "{description}", rows: "5", maxlength: "20000",
                                placeholder: "Objectifs, consignes, durée, organisation…",
                                oninput: move |e| description.set(e.value()) }
                        }
                        div { class: "field",
                            label { "Lien vers une image (facultatif)" }
                            input { r#type: "url", value: "{image}",
                                placeholder: "https://…",
                                oninput: move |e| image.set(e.value()) }
                            small { "Pour cette version, l'image est référencée par une URL publique (pas de téléversement)." }
                        }
                        button { class: "button solid", r#type: "submit", disabled: *busy.read(),
                            if *busy.read() { "Enregistrement…" } else { "Ajouter cette activité" }
                        }
                    }
                }

                if detail.activities.is_empty() {
                    div { class: "panel-empty", "Aucune activité. Ajoutez un exercice pour préparer la séance." }
                }
                for activity in &detail.activities {
                    ActivityCard {
                        key: "{activity.id}",
                        activity: activity.clone(),
                        materials: detail.materials.iter().filter(|m| m.activity_id == activity.id).cloned().collect(),
                        token: token.clone(),
                        training_id: training_id.clone(),
                        on_saved,
                        on_error,
                    }
                }
            }

            aside { class: "panel materials-overview",
                div { class: "panel-head",
                    div {
                        h2 { "Matériel total" }
                        p { "Somme de toutes les activités" }
                    }
                }
                if totals.is_empty() {
                    div { class: "panel-empty", "Pas encore de matériel renseigné." }
                } else {
                    ul { class: "materials-total",
                        for (material, quantity) in totals {
                            li { span { "{material}" } strong { "{quantity}" } }
                        }
                    }
                }
                Link { to: Route::TrainingPrint { id: training_id.clone() }, class: "print-footer-link",
                    "Ouvrir la fiche imprimable →"
                }
            }
        }
    }
}

#[component]
fn ActivityCard(
    activity: Activity,
    materials: Vec<Material>,
    token: String,
    training_id: String,
    on_saved: EventHandler<()>,
    on_error: EventHandler<String>,
) -> Element {
    let mut material_name = use_signal(String::new);
    let mut quantity = use_signal(|| "1".to_string());
    let mut busy = use_signal(|| false);
    rsx! {
        article { class: "exercise",
            div { class: "exercise-head",
                h3 { "{activity.name}" }
                button { class: "remove-link", title: "Supprimer cet exercice", disabled: *busy.read(),
                    onclick: {
                        let token = token.clone();
                        let training_id = training_id.clone();
                        let activity_id = activity.id.clone();
                        move |_| {
                            if *busy.read() { return; }
                            let token = token.clone();
                            let training_id = training_id.clone();
                            let activity_id = activity_id.clone();
                            busy.set(true);
                            spawn(async move {
                                match trainings::remove_activity(&token, &training_id, &activity_id).await {
                                    Ok(()) => on_saved.call(()),
                                    Err(err) => on_error.call(err),
                                }
                                busy.set(false);
                            });
                        }
                    },
                    "Supprimer"
                }
            }
            if !activity.description.trim().is_empty() {
                p { class: "exercise-description", "{activity.description}" }
            }
            if let Some(url) = activity.image_url.as_ref() {
                img { class: "exercise-image", src: "{url}", alt: "Illustration de {activity.name}" }
            }
            div { class: "material-section",
                strong { "Matériel" }
                if materials.is_empty() {
                    p { class: "muted-small", "Aucun matériel" }
                }
                for material in &materials {
                    div { class: "material-row", key: "{material.id}",
                        span { "{material.name} · {material.quantity}" }
                        button { class: "remove-link", disabled: *busy.read(), title: "Retirer du matériel",
                            onclick: {
                                let token = token.clone();
                                let training_id = training_id.clone();
                                let activity_id = activity.id.clone();
                                let material_id = material.id.clone();
                                move |_| {
                                    let token = token.clone();
                                    let training_id = training_id.clone();
                                    let activity_id = activity_id.clone();
                                    let material_id = material_id.clone();
                                    busy.set(true);
                                    spawn(async move {
                                        match trainings::remove_material(&token, &training_id, &activity_id, &material_id).await {
                                            Ok(()) => on_saved.call(()),
                                            Err(err) => on_error.call(err),
                                        }
                                        busy.set(false);
                                    });
                                }
                            },
                            "Retirer"
                        }
                    }
                }
                form { class: "add-material",
                    onsubmit: {
                        let token = token.clone();
                        let training_id = training_id.clone();
                        let activity_id = activity.id.clone();
                        move |event| {
                            event.prevent_default();
                            if *busy.read() { return; }
                            let name = material_name.read().trim().to_string();
                            let count = quantity.read().parse::<i32>().unwrap_or(0);
                            if name.is_empty() || count <= 0 {
                                on_error.call("Renseignez du matériel et une quantité supérieure à 0.".into());
                                return;
                            }
                            let token = token.clone();
                            let training_id = training_id.clone();
                            let activity_id = activity_id.clone();
                            busy.set(true);
                            spawn(async move {
                                match trainings::add_material(&token, &training_id, &activity_id, &name, count).await {
                                    Ok(()) => {
                                        material_name.set(String::new());
                                        quantity.set("1".into());
                                        on_saved.call(());
                                    }
                                    Err(err) => on_error.call(err),
                                }
                                busy.set(false);
                            });
                        }
                    },
                    input { "aria-label": "Nom du matériel", placeholder: "Ex. Ballons", required: true,
                        value: "{material_name}", maxlength: "150",
                        oninput: move |e| material_name.set(e.value()) }
                    input { "aria-label": "Quantité", r#type: "number", min: "1", max: "100000",
                        value: "{quantity}", oninput: move |e| quantity.set(e.value()) }
                    button { class: "button outline small", r#type: "submit", disabled: *busy.read(), "Ajouter" }
                }
            }
        }
    }
}
