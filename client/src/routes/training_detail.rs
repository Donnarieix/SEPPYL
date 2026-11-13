use dioxus::prelude::*;
use std::collections::BTreeMap;

use crate::api::trainings::{self, Activity, Detail, Material};
use crate::api::catalog;
use crate::api::preferences::{self, AttendanceView};
use super::{Route, dialogs::PlayerDialog, layout::Header, rich_editor::RichTextEditor};

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
    // A global, account-specific preference, not tied to the current training.
    let preference_token = token.clone();
    let preferences_resource = use_resource(move || {
        let token = preference_token.clone();
        async move {
            match token {
                Some(token) => preferences::load(&token).await,
                None => Err("Connexion requise.".to_owned()),
            }
        }
    });
    let mut preference_override = use_signal(|| None::<AttendanceView>);
    let mut saving_preference = use_signal(|| false);
    let mut preference_error = use_signal(|| None::<String>);

    if token.is_none() {
        return rsx! {
            div { class: "centered",
                p { "Veuillez vous connecter." }
                button { class: "button solid", onclick: move |_| { navigator.push(Route::Login {}); }, "Connexion" }
            }
        };
    }
    let token = token.unwrap_or_default();
    let current_view = preference_override.read().as_ref().copied()
        .or_else(|| preferences_resource.read().clone().and_then(Result::ok))
        .unwrap_or_default();
    let grid_token = token.clone();
    let list_token = token.clone();
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
                                p{"Durée prévue : {detail.training.duration_minutes.unwrap_or(0)} min"}
                            }
                            Link { to: Route::TrainingPrint { id: id.clone() }, class: "button outline print-link",
                                "Fiche imprimable ↗"
                            }
                        }

                        TrainingDuration{token:token.clone(),training_id:id.clone(),training:detail.training.clone(),
                            on_saved:move |_|data.restart(),on_error:move |e|notice.set(Some(e))}
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
                                    div { class: "attendance-head-actions",
                                        div { class: "attendance-view-switch",
                                            role: "group",
                                            "aria-label": "Mode d'affichage des joueuses",
                                            button {
                                                class: if current_view == AttendanceView::Grid { "attendance-view-button active" } else { "attendance-view-button" },
                                                r#type: "button",
                                                title: "Vue grille",
                                                "aria-label": "Afficher les joueuses en grille",
                                                "aria-pressed": if current_view == AttendanceView::Grid { "true" } else { "false" },
                                                disabled: *saving_preference.read(),
                                                onclick: move |_| {
                                                    if *saving_preference.read() || current_view == AttendanceView::Grid { return; }
                                                    preference_override.set(Some(AttendanceView::Grid));
                                                    preference_error.set(None);
                                                    saving_preference.set(true);
                                                    let token = grid_token.clone();
                                                    spawn(async move {
                                                        if let Err(error) = preferences::save(&token, AttendanceView::Grid).await {
                                                            preference_override.set(None);
                                                            preference_error.set(Some(error));
                                                        }
                                                        saving_preference.set(false);
                                                    });
                                                },
                                                span { class: "attendance-grid-icon", "aria-hidden": "true",
                                                    span {}
                                                    span {}
                                                    span {}
                                                    span {}
                                                }
                                            }
                                            button {
                                                class: if current_view == AttendanceView::List { "attendance-view-button active" } else { "attendance-view-button" },
                                                r#type: "button",
                                                title: "Vue liste",
                                                "aria-label": "Afficher les joueuses en liste",
                                                "aria-pressed": if current_view == AttendanceView::List { "true" } else { "false" },
                                                disabled: *saving_preference.read(),
                                                onclick: move |_| {
                                                    if *saving_preference.read() || current_view == AttendanceView::List { return; }
                                                    preference_override.set(Some(AttendanceView::List));
                                                    preference_error.set(None);
                                                    saving_preference.set(true);
                                                    let token = list_token.clone();
                                                    spawn(async move {
                                                        if let Err(error) = preferences::save(&token, AttendanceView::List).await {
                                                            preference_override.set(None);
                                                            preference_error.set(Some(error));
                                                        }
                                                        saving_preference.set(false);
                                                    });
                                                },
                                                span { class: "attendance-list-icon", "aria-hidden": "true",
                                                    span {}
                                                    span {}
                                                    span {}
                                                }
                                            }
                                        }
                                        button { class: "button outline small", onclick: move |_| adding_player.set(true),
                                            "+ Joueuse"
                                        }
                                    }
                                }
                                if let Some(message) = preference_error.read().as_ref() {
                                    p { class: "attendance-preference-error", role: "alert", "{message}" }
                                } else if let Some(Err(message)) = preferences_resource.read().clone() {
                                    p { class: "attendance-preference-error", role: "alert",
                                        "La préférence d'affichage n'a pas pu être chargée : {message}"
                                    }
                                }
                                if detail.players.is_empty() {
                                    div { class: "panel-empty", "Ajoutez d'abord les joueuses de l'équipe." }
                                } else {
                                    if current_view == AttendanceView::List {
                                        div { class: "attendance-list-heading",
                                            span { "Joueuse" }
                                            span { "Présente" }
                                        }
                                    }
                                    div {
                                        class: if current_view == AttendanceView::List {
                                            "attendance-list list-mode"
                                        } else {
                                            "attendance-list"
                                        },
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
                        "data-seppyl-searchable":"true",
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
                            let current_comment=if activity.is_empty(){
                                detail.attendance.iter().find(|a|a.player_id==player_id)
                                    .and_then(|a|a.score_comment.clone()).unwrap_or_default()
                            }else{
                                detail.scores.iter().find(|s|s.player_id==player_id&&s.activity_id==activity)
                                    .and_then(|s|s.comment.clone()).unwrap_or_default()
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
                                    ScoreComment{
                                        key:"{key}",token:token.clone(),training_id:training_id.clone(),
                                        player_id:player_id.clone(),activity_id:activity_id.clone(),
                                        initial:current_comment,score,on_saved,on_error
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
fn ScoreComment(
    token:String,training_id:String,player_id:String,activity_id:Option<String>,
    initial:String,score:Option<i32>,on_saved:EventHandler<()>,
    on_error:EventHandler<String>,
)->Element{
    let mut comment=use_signal(||initial);
    let mut busy=use_signal(||false);
    rsx!{
        div{class:"score-comment-field",
            input{value:"{comment}",maxlength:"4000",
                placeholder:"Informations complémentaires",
                "aria-label":"Informations complémentaires",
                oninput:move |e|comment.set(e.value())}
            button{class:"button quiet small",r#type:"button",
                disabled:*busy.read()||score.is_none(),
                onclick:move |_|{
                    let t=token.clone();let id=training_id.clone();let p=player_id.clone();
                    let activity=activity_id.clone();let text=comment.read().clone();
                    busy.set(true);
                    spawn(async move{
                        match trainings::save_score_comment(&t,&id,&p,activity.as_deref(),score,Some(&text)).await{
                            Ok(())=>on_saved.call(()),Err(e)=>on_error.call(e)
                        }
                        busy.set(false);
                    });
                },"Enregistrer"}
        }
    }
}

#[component]
fn TrainingDuration(
    token:String,training_id:String,training:trainings::Training,
    on_saved:EventHandler<()>,on_error:EventHandler<String>,
)->Element{
    let mut duration=use_signal(||training.duration_minutes.map(|n|n.to_string()).unwrap_or_default());
    let mut busy=use_signal(||false);
    rsx!{
        div{class:"training-duration-editor",
            label{"Durée (minutes)"}
            input{r#type:"number",min:"1",max:"1440",value:"{duration}",
                oninput:move |e|duration.set(e.value())}
            button{class:"button outline small",disabled:*busy.read(),
                onclick:move |_|{
                    let t=token.clone();let id=training_id.clone();
                    let name=training.name.clone();let date=training.date.get(..10).unwrap_or("").to_string();
                    let notes=training.notes.clone();
                    let minutes=duration.read().parse::<i32>().ok().filter(|m|(1..=1440).contains(m));
                    busy.set(true);
                    spawn(async move{
                        match trainings::update(&t,&id,&name,&date,&notes,minutes).await{
                            Ok(())=>on_saved.call(()),Err(e)=>on_error.call(e)
                        }
                        busy.set(false);
                    });
                },"Enregistrer"}
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
    let token_for_submit = token.clone();
    let training_for_submit = training_id.clone();
    let catalog_token = token.clone();
    let catalog_entries = use_resource(move || {
        let token = catalog_token.clone();
        async move { catalog::list(&token).await }
    });
    let mut selected = use_signal(String::new);
    let drag_source = use_signal(|| None::<String>);

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
                    button { class: "button outline small", onclick: move |_| {
                        let next = !*adding.read();
                        adding.set(next);
                    }, "+ Activité" }
                }
                div { class: "catalog-picker",
                    div { class: "picker-heading",
                        strong { "Réutiliser une activité" }
                        Link { to: Route::Activities {}, "Gérer la bibliothèque →" }
                    }
                    match catalog_entries.read().clone() {
                        Some(Ok(available)) => rsx! {
                            div { class: "picker-fields",
                                select {
                                    value: "{selected}",
                                    "data-seppyl-searchable":"true",
                                    onchange: move |e| selected.set(e.value()),
                                    option { value: "", "Choisir une activité existante…" }
                                    for item in available {
                                        option { value: "{item.id}", "{item.name}" }
                                    }
                                }
                                button { class: "button outline small",
                                    disabled: selected.read().is_empty() || *busy.read(),
                                    onclick: {
                                        let token = token.clone();
                                        let training_id = training_id.clone();
                                        move |_| {
                                            let chosen = selected.read().clone();
                                            if chosen.is_empty() || *busy.read() { return; }
                                            let token = token.clone();
                                            let training_id = training_id.clone();
                                            busy.set(true);
                                            spawn(async move {
                                                match catalog::attach_to_training(&token, &training_id, &chosen).await {
                                                    Ok(()) => { selected.set(String::new()); on_saved.call(()); },
                                                    Err(e) => on_error.call(e),
                                                }
                                                busy.set(false);
                                            });
                                        }
                                    },
                                    "Ajouter à la séance"
                                }
                            }
                        },
                        Some(Err(message)) => rsx! { p { class: "muted-small", "{message}" } },
                        None => rsx! { p { class: "muted-small", "Chargement de la bibliothèque…" } },
                    }
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
                            let token = token_for_submit.clone();
                            let id = training_for_submit.clone();
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
                            small { "Pour téléverser une image, vous pouvez ensuite modifier cette activité dans la bibliothèque." }
                        }
                        button { class: "button solid", r#type: "submit", disabled: *busy.read(),
                            if *busy.read() { "Enregistrement…" } else { "Ajouter cette activité" }
                        }
                    }
                }

                if detail.activities.is_empty() {
                    div { class: "panel-empty", "Aucune activité. Ajoutez un exercice pour préparer la séance." }
                }
                for (position, activity) in detail.activities.iter().enumerate() {
                    SortableActivity {
                        key: "{activity.id}",
                        activity: activity.clone(),
                        materials: detail.materials.iter().filter(|m| m.activity_id == activity.id).cloned().collect(),
                        all_ids: detail.activities.iter().map(|a| a.id.clone()).collect(),
                        position,
                        drag_source,
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

fn updated_order(ids: &[String], source: &str, target: &str) -> Option<Vec<String>> {
    let from = ids.iter().position(|id| id == source)?;
    let to = ids.iter().position(|id| id == target)?;
    if from == to { return None; }
    let mut moved = ids.to_vec();
    let item = moved.remove(from);
    moved.insert(to, item);
    Some(moved)
}
fn request_reorder(
    mut busy: Signal<bool>,
    ids: Vec<String>,
    source: String,
    target: String,
    token: String,
    training_id: String,
    on_saved: EventHandler<()>,
    on_error: EventHandler<String>,
) {
    if *busy.read() { return; }
    let Some(next) = updated_order(&ids, &source, &target) else { return; };
    busy.set(true);
    spawn(async move {
        match trainings::reorder_activities(&token, &training_id, &next).await {
            Ok(()) => on_saved.call(()),
            Err(message) => on_error.call(message),
        }
        busy.set(false);
    });
}

#[component]
fn SortableActivity(
    activity: Activity,
    materials: Vec<Material>,
    all_ids: Vec<String>,
    position: usize,
    mut drag_source: Signal<Option<String>>,
    token: String,
    training_id: String,
    on_saved: EventHandler<()>,
    on_error: EventHandler<String>,
) -> Element {
    let busy = use_signal(|| false);
    let activity_id_for_drag = activity.id.clone();
    let activity_id_for_drop = activity.id.clone();
    let activity_id_for_up = activity.id.clone();
    let activity_id_for_down = activity.id.clone();
    let total = all_ids.len();
    let order_for_drop = all_ids.clone();
    let order_for_up = all_ids.clone();
    let order_for_down = all_ids.clone();
    let token_drop = token.clone();
    let token_up = token.clone();
    let token_down = token.clone();
    let training_drop = training_id.clone();
    let training_up = training_id.clone();
    let training_down = training_id.clone();

    rsx! {
        div {
            class: "sortable-exercise",
            draggable: "true",
            ondragstart: move |_| drag_source.set(Some(activity_id_for_drag.clone())),
            ondragend: move |_| drag_source.set(None),
            ondragover: move |event| event.prevent_default(),
            ondrop: move |event| {
                event.prevent_default();
                let from = drag_source.read().clone();
                drag_source.set(None);
                if let Some(from) = from {
                    request_reorder(
                        busy, order_for_drop.clone(), from, activity_id_for_drop.clone(),
                        token_drop.clone(), training_drop.clone(), on_saved, on_error,
                    );
                }
            },
            div { class: "sortable-controls",
                span { class: "sort-grip", title: "Glisser pour réordonner", "aria-hidden": "true", "⋮⋮" }
                span { class: "sort-index", "{position + 1}" }
                span { class: "sort-help", "Glisser pour réordonner" }
                button {
                    class: "button quiet small", r#type: "button",
                    title: "Monter l'activité", "aria-label": "Monter l'activité",
                    disabled: position == 0 || *busy.read(),
                    onclick: move |_| {
                        if position > 0 {
                            request_reorder(
                                busy, order_for_up.clone(), activity_id_for_up.clone(),
                                order_for_up[position - 1].clone(), token_up.clone(),
                                training_up.clone(), on_saved, on_error,
                            );
                        }
                    },
                    "↑"
                }
                button {
                    class: "button quiet small", r#type: "button",
                    title: "Descendre l'activité", "aria-label": "Descendre l'activité",
                    disabled: position + 1 >= total || *busy.read(),
                    onclick: move |_| {
                        if position + 1 < order_for_down.len() {
                            request_reorder(
                                busy, order_for_down.clone(), activity_id_for_down.clone(),
                                order_for_down[position + 1].clone(), token_down.clone(),
                                training_down.clone(), on_saved, on_error,
                            );
                        }
                    },
                    "↓"
                }
            }
            ActivityCard {
                activity: activity.clone(),
                materials: materials.clone(),
                token: token.clone(),
                training_id: training_id.clone(),
                on_saved,
                on_error,
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
    let mut editing=use_signal(||false);
    let mut edit_name=use_signal(||activity.name.clone());
    let mut edit_description=use_signal(||activity.description.clone());
    let mut edit_html=use_signal(||activity.description_html.clone());
    let mut edit_image=use_signal(||activity.image_url.clone().unwrap_or_default());
    rsx! {
        article { class: "exercise",
            div { class: "exercise-head",
                h3 { "{activity.name}" }
                div { class:"exercise-head-actions",
                    button { class:"button outline small",r#type:"button",
                        disabled:*busy.read(),onclick:move |_|editing.set(true),
                        "Modifier"
                    }
                button { class: "remove-link", title: "Retirer cette activité de l’entraînement", disabled: *busy.read(),
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
                    "Retirer de l’entraînement"
                }
                }
            }
            if let Some(html)=activity.description_html.as_deref() {
                div { class: "exercise-description rich-description", dangerous_inner_html: html }
            } else if !activity.description.trim().is_empty() {
                p { class: "exercise-description", "{activity.description}" }
            }
            if let Some(url) = activity.image_url.as_ref() {
                img { class: "exercise-image", src: "{catalog::image_src(url)}", alt: "Illustration de {activity.name}" }
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
        if *editing.read(){
            div { class:"dialog-backdrop",
                div { class:"dialog-panel activity-dialog",role:"dialog","aria-modal":"true",
                    div { class:"dialog-header",
                        div { h2 { "Modifier l'activité de la séance" }
                            p { "Cette modification concerne uniquement cette séance, pas la bibliothèque." }
                        }
                        button { class:"icon-button",r#type:"button",
                            onclick:move |_|editing.set(false),"×" }
                    }
                    form { onsubmit:{
                        let token=token.clone();let training_id=training_id.clone();
                        let activity_id=activity.id.clone();
                        move |event|{
                            event.prevent_default();
                            if *busy.read(){return;}
                            let name=edit_name.read().trim().to_string();
                            if name.is_empty(){on_error.call("Le nom est obligatoire.".into());return;}
                            let description=edit_description.read().clone();
                            let html=edit_html.read().clone();
                            let image=edit_image.read().trim().to_string();
                            let token=token.clone();let training_id=training_id.clone();
                            let activity_id=activity_id.clone();
                            busy.set(true);
                            spawn(async move{
                                let result=trainings::update_activity(
                                    &token,&training_id,&activity_id,&name,&description,
                                    html.as_deref(),if image.is_empty(){None}else{Some(&image)}
                                ).await;
                                match result{
                                    Ok(())=>{editing.set(false);on_saved.call(());},
                                    Err(error)=>on_error.call(error),
                                }
                                busy.set(false);
                            });
                        }
                    },
                        div { class:"dialog-body",
                            div { class:"field",label { "Nom" }
                                input { value:"{edit_name}",required:true,maxlength:"150",
                                    oninput:move |e|edit_name.set(e.value()) }
                            }
                            div { class:"field",label { "Description et consignes" }
                                RichTextEditor{
                                    initial_html:activity.description_html.clone(),
                                    initial_plain:activity.description.clone(),
                                    on_html:move |v:String|edit_html.set(Some(v)),
                                    on_plain:move |v:String|edit_description.set(v),
                                }
                            }
                            div { class:"field",label { "URL de l'image (facultatif)" }
                                input { value:"{edit_image}",placeholder:"https://…",
                                    oninput:move |e|edit_image.set(e.value()) }
                            }
                        }
                        div { class:"dialog-footer",
                            button { class:"button quiet",r#type:"button",
                                onclick:move |_|editing.set(false),"Annuler" }
                            button { class:"button solid",r#type:"submit",disabled:*busy.read(),
                                if *busy.read(){"Enregistrement…"}else{"Enregistrer les modifications"}
                            }
                        }
                    }
                }
            }
        }
    }
}
