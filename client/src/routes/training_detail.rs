use dioxus::prelude::*;
use std::collections::BTreeMap;

use crate::api::trainings::{self, Activity, Detail, Material};
use crate::api::catalog;
use crate::api::preferences::{self, AttendanceView};
use super::{Route, dialogs::PlayerDialog, layout::Header,rich_editor::RichEditor};


/// Move an existing entry before the target. No-op for invalid or equal IDs.
fn reorder_before(order:&[String],item:&str,target:&str)->Option<Vec<String>>{
    if item==target{return None}
    let mut items=order.to_vec();
    let from=items.iter().position(|id|id==item)?;
    items.remove(from);
    let at=items.iter().position(|id|id==target)?;
    items.insert(at,item.to_string());
    if items==order {None}else{Some(items)}
}
fn reorder_after(order:&[String],item:&str,target:&str)->Option<Vec<String>>{
    if item==target{return None}
    let mut items=order.to_vec();
    let from=items.iter().position(|id|id==item)?;
    items.remove(from);
    let at=items.iter().position(|id|id==target)?;
    items.insert(at+1,item.to_string());
    if items==order {None}else{Some(items)}
}

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
    let token_for_submit = token.clone();
    let training_for_submit = training_id.clone();
    let catalog_token = token.clone();
    let catalog_entries = use_resource(move || {
        let token = catalog_token.clone();
        async move { catalog::list(&token).await }
    });
    let mut selected = use_signal(String::new);
    let mut dragged = use_signal(|| None::<String>);

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
                            RichEditor {
                                initial: String::new(),
                                on_change:move |html:String|description.set(html),
                            }
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
                for (index,activity) in detail.activities.iter().enumerate() {
                    {
                        let item_id=activity.id.clone();
                        let order:Vec<String>=detail.activities.iter().map(|a|a.id.clone()).collect();
                        let token_for_drag=token.clone();
                        let training_for_drag=training_id.clone();
                        let token_for_move=token.clone();
                        let training_for_move=training_id.clone();
                        let target_id=item_id.clone();
                        let previous=index.checked_sub(1).and_then(|i|order.get(i)).cloned();
                        let next=order.get(index+1).cloned();
                        rsx! {
                            div {
                                class:"sortable-activity",
                                key:"{item_id}",
                                draggable:"true",
                                ondragstart: {
                                    let item_id=item_id.clone();
                                    move |_|dragged.set(Some(item_id.clone()))
                                },
                                ondragover: move |event|event.prevent_default(),
                                ondrop: move |event| {
                                    event.prevent_default();
                                    let Some(from)=dragged.read().clone() else {return;};
                                    dragged.set(None);
                                    let Some(reordered)=reorder_before(&order,&from,&target_id) else {return;};
                                    let token=token_for_drag.clone();
                                    let training_id=training_for_drag.clone();
                                    spawn(async move {
                                        match trainings::reorder(&token,&training_id,&reordered).await{
                                            Ok(())=>on_saved.call(()),
                                            Err(e)=>on_error.call(e),
                                        }
                                    });
                                },
                                div { class:"activity-order-controls",
                                    span { class:"drag-handle",title:"Glisser pour déplacer", "aria-hidden":"true", "⠿" }
                                    span { class:"muted-small", "Glisser pour réorganiser" }
                                    button { class:"button outline small", disabled:previous.is_none(),
                                        "aria-label":"Monter l'activité",
                                        onclick:{
                                            let target=previous.clone();
                                            let order=order.clone();
                                            let id=item_id.clone();
                                            let token=token_for_move.clone();
                                            let training_id=training_for_move.clone();
                                            move |_| {
                                                let Some(target)=target.as_ref() else{return;};
                                                let Some(reordered)=reorder_before(&order,&id,target) else{return;};
                                                let token=token.clone();
                                                let training_id=training_id.clone();
                                                spawn(async move{
                                                    match trainings::reorder(&token,&training_id,&reordered).await{
                                                        Ok(())=>on_saved.call(()),
                                                        Err(e)=>on_error.call(e),
                                                    }
                                                });
                                            }
                                        },"↑" }
                                    button { class:"button outline small", disabled:next.is_none(),
                                        "aria-label":"Descendre l'activité",
                                        onclick:{
                                            let target=next.clone();
                                            let order=order.clone();
                                            let id=item_id.clone();
                                            let token=token.clone();
                                            let training_id=training_id.clone();
                                            move |_| {
                                                let Some(target)=target.as_ref() else{return;};
                                                let Some(reordered)=reorder_after(&order,&id,target) else{return;};
                                                let token=token.clone();
                                                let training_id=training_id.clone();
                                                spawn(async move{
                                                    match trainings::reorder(&token,&training_id,&reordered).await{
                                                        Ok(())=>on_saved.call(()),
                                                        Err(e)=>on_error.call(e),
                                                    }
                                                });
                                            }
                                        },"↓" }
                                }
                                ActivityCard {
                                    activity:activity.clone(),
                                    materials: detail.materials.iter().filter(|m| m.activity_id == activity.id).cloned().collect(),
                                    token:token.clone(),
                                    training_id:training_id.clone(),
                                    on_saved,
                                    on_error,
                                }
                            }
                        }
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
                div { class: "exercise-description rich-render", dangerous_inner_html: "{activity.description}" }
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
    }
}
