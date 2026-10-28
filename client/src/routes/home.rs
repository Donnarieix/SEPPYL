use dioxus::prelude::*;
use crate::api::matrix::{self, Matrix};
use crate::api::catalog;
use super::{activities::ActivityEditor, dialogs::PlayerDialog, layout::Header};

#[derive(Clone, Copy, PartialEq)]
enum CreationKind {
    Player,
    Activity,
}

#[component]
pub fn Home() -> Element {
    let session = use_context::<Signal<Option<String>>>();
    let navigator = use_navigator();
    let mut search = use_signal(String::new);
    let mut dialog = use_signal(|| None::<CreationKind>);
    let token = session.read().clone();
    let fetch_token = token.clone();

    let mut data = use_resource(move || {
        let token = fetch_token.clone();
        async move {
            match token {
                Some(token) => matrix::load(&token).await,
                None => Err("Connectez-vous pour consulter le tableau.".to_string()),
            }
        }
    });

    if token.is_none() {
        return rsx! {
            div { class: "signed-out",
                h1 { "SEPPYL" }
                p { "Connectez-vous pour accéder aux résultats." }
                button { class: "button solid", onclick: move |_| { navigator.push(crate::routes::Route::Login {}); },
                    "Se connecter"
                }
            }
        };
    }

    let auth_token = token.unwrap_or_default();
    let catalog_token = auth_token.clone();
    let mut catalog_list = use_resource(move || {
        let token = catalog_token.clone();
        async move { catalog::list(&token).await }
    });
    let mut catalog_selection = use_signal(String::new);
    let mut catalog_error = use_signal(|| None::<String>);
    let token_for_select = auth_token.clone();
    let snapshot = data.read().clone();
    let token_for_created = auth_token.clone();

    rsx! {
        div { class: "app",
            Header {}
            main { class: "page",
                div { class: "page-heading",
                    div {
                        h1 { "Tableau des résultats" }
                        p { "Suivi des joueuses par activité" }
                    }
                }

                if let Some(message) = catalog_error.read().as_ref() {
                    p { class: "form-error", role: "alert", "{message}" }
                }
                div { class: "catalog-picker matrix-picker",
                    div { class: "picker-heading",
                        strong { "Ajouter une colonne à partir de la bibliothèque" }
                        span { class: "muted-small", "Les activités créées dans les entraînements sont disponibles ici." }
                    }
                    match catalog_list.read().clone() {
                        Some(Ok(items)) => rsx! {
                            div { class: "picker-fields",
                                select {
                                    value: "{catalog_selection}",
                                    onchange: move |e| catalog_selection.set(e.value()),
                                    option { value: "", "Choisir une activité…" }
                                    for item in items {
                                        option { value: "{item.id}", "{item.name}" }
                                    }
                                }
                                button { class: "button solid small", disabled: catalog_selection.read().is_empty(),
                                    onclick: {
                                        let token = token_for_select.clone();
                                        move |_| {
                                            let token = token.clone();
                                            let id = catalog_selection.read().clone();
                                            spawn(async move {
                                                match catalog::select_for_table(&token, &id).await {
                                                    Ok(()) => { catalog_error.set(None); data.restart(); },
                                                    Err(e) => catalog_error.set(Some(e)),
                                                }
                                            });
                                        }
                                    },
                                    "Ajouter au tableur"
                                }
                            }
                        },
                        Some(Err(e)) => rsx! { p { class: "muted-small", "{e}" } },
                        None => rsx! { p { class: "muted-small", "Chargement des activités…" } },
                    }
                }
                match snapshot {
                    Some(Ok(matrix)) => rsx! {
                        div { class: "sheet",
                            div { class: "sheet-toolbar",
                                div { class: "sheet-label",
                                    strong { "Résultats" }
                                    span { "{matrix.players.len()} joueuses" }
                                    span { class: "separator", "·" }
                                    span { "{matrix.activities.len()} activités" }
                                }
                                div { class: "sheet-tools",
                                    if !matrix.players.is_empty() {
                                        input {
                                            class: "search",
                                            r#type: "search",
                                            "aria-label": "Rechercher une joueuse",
                                            placeholder: "Rechercher une joueuse",
                                            value: "{search}",
                                            oninput: move |event| search.set(event.value()),
                                        }
                                    }
                                    button { class: "refresh", title: "Recharger les données",
                                        onclick: move |_| data.restart(), "Actualiser" }
                                }
                            }

                            MatrixTable {
                                matrix: matrix.clone(),
                                filter: search.read().clone(),
                                token: auth_token.clone(),
                                on_add_player: move |_| dialog.set(Some(CreationKind::Player)),
                                on_add_activity: move |_| dialog.set(Some(CreationKind::Activity)),
                            }
                            if matrix.players.is_empty() || matrix.activities.is_empty() {
                                p { class: "grid-empty-note",
                                    "Utilisez les boutons + dans le tableau pour ajouter une joueuse ou une activité."
                                }
                            }
                        }
                        div { class: "help-line",
                            span { "Cliquez dans une cellule pour saisir un résultat." }
                            span { "Enregistrement automatique lorsque vous quittez la cellule." }
                        }
                    },
                    Some(Err(message)) => rsx! {
                        div { class: "load-state",
                            strong { "Impossible de charger le tableau" }
                            p { "{message}" }
                            button { class: "button outline", onclick: move |_| data.restart(), "Réessayer" }
                        }
                    },
                    None => rsx! { div { class: "load-state", "Chargement des données…" } },
                }
            }

            if *dialog.read() == Some(CreationKind::Player) {
                PlayerDialog {
                    token: auth_token.clone(),
                    on_cancel: move |_| dialog.set(None),
                    on_created: move |_| { dialog.set(None); data.restart(); },
                }
            }
            if *dialog.read() == Some(CreationKind::Activity) {
                ActivityEditor {
                    token: auth_token.clone(),
                    activity: None,
                    on_cancel: move |_| dialog.set(None),
                    on_saved: move |new_id: String| {
                        let token = token_for_created.clone();
                        spawn(async move {
                            match catalog::select_for_table(&token, &new_id).await {
                                Ok(()) => {
                                    catalog_error.set(None);
                                    data.restart();
                                }
                                Err(error) => catalog_error.set(Some(format!(
                                    "L'activité a été créée dans la bibliothèque mais n'a pas pu être ajoutée au tableur : {error}"
                                ))),
                            }
                            dialog.set(None);
                            catalog_list.restart();
                        });
                    },
                    on_error: move |message| catalog_error.set(Some(message)),
                }
            }
        }
    }
}

#[component]
fn MatrixTable(
    matrix: Matrix,
    filter: String,
    token: String,
    on_add_player: EventHandler<()>,
    on_add_activity: EventHandler<()>,
) -> Element {
    let mut players = matrix.players.clone();
    players.sort_by(|a, b| a.number.cmp(&b.number).then_with(|| a.lastname.cmp(&b.lastname)));
    let mut activities = matrix.activities.clone();
    activities.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.name.cmp(&b.name)));
    let query = filter.trim().to_lowercase();
    let visible: Vec<_> = players.iter().filter(|player| {
        format!("{} {} {}", player.firstname, player.lastname, player.number)
            .to_lowercase()
            .contains(&query)
    }).collect();

    rsx! {
        div { class: "sheet-scroll",
            table { class: "grid-table",
                thead {
                    tr {
                        th { class: "player-column",
                            span { class: "col-label", "Joueuse" }
                        }
                        for activity in &activities {
                            th { key: "{activity.id}", class: "activity-column",
                                span { class: "activity-title", "{activity.name}" }
                                if !activity.date.is_empty() {
                                    span { class: "activity-date", "{activity.date.get(..10).unwrap_or(&activity.date)}" }
                                }
                            }
                        }
                        th { class: "new-activity-column",
                            button {
                                class: "grid-add-button",
                                r#type: "button",
                                "aria-label": "Créer une activité",
                                title: "Créer une activité",
                                onclick: move |_| on_add_activity.call(()),
                                "+"
                            }
                        }
                    }
                }
                tbody {
                    for player in &visible {
                        tr { key: "{player.id}",
                            th { class: "player-column", scope: "row",
                                div { class: "player-line",
                                    span { class: "player-number", "{player.number}" }
                                    span { class: "player-name", "{player.firstname} {player.lastname}" }
                                }
                            }
                            for activity in &activities {
                                td { key: "{activity.id}",
                                    ResultCell {
                                        player_id: player.id.clone(),
                                        activity_id: activity.id.clone(),
                                        token: token.clone(),
                                        label: format!("{} {} : {}", player.firstname, player.lastname, activity.name),
                                        initial: matrix.results.iter()
                                            .find(|result| result.player_id == player.id && result.activity_id == activity.id)
                                            .and_then(|result| result.result.clone())
                                            .unwrap_or_default(),
                                    }
                                }
                            }
                            td { class: "grid-add-spacer" }
                        }
                    }
                    tr { class: "grid-add-row",
                        th { class: "player-column", scope: "row",
                            button {
                                class: "grid-add-player",
                                r#type: "button",
                                title: "Ajouter une joueuse",
                                "aria-label": "Ajouter une joueuse",
                                onclick: move |_| on_add_player.call(()),
                                span { class: "grid-add-button", "+" }
                                span { "Ajouter une joueuse" }
                            }
                        }
                        for activity in &activities {
                            td { key: "{activity.id}", class: "grid-add-spacer" }
                        }
                        td { class: "grid-add-spacer" }
                    }
                }
            }
            if visible.is_empty() && !matrix.players.is_empty() {
                div { class: "no-results", "Aucune joueuse trouvée." }
            }
        }
    }
}

#[component]
fn ResultCell(
    player_id: String,
    activity_id: String,
    token: String,
    label: String,
    initial: String,
) -> Element {
    let mut draft = use_signal(|| initial.clone());
    let mut committed = use_signal(|| initial.clone());
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut saved = use_signal(|| false);

    let persist = move || {
        if *saving.read() { return; }
        let next = draft.read().trim().to_owned();
        if next == committed.read().as_str() { return; }

        let token = token.clone();
        let player_id = player_id.clone();
        let activity_id = activity_id.clone();

        saving.set(true);
        error.set(None);
        saved.set(false);

        spawn(async move {
            match matrix::save(&token, &player_id, &activity_id, &next).await {
                Ok(()) => {
                    committed.set(next);
                    saved.set(true);
                }
                Err(message) => error.set(Some(message)),
            }
            saving.set(false);
        });
    };

    let mut on_change = persist.clone();
    let mut on_retry = persist.clone();

    rsx! {
        div { class: "cell",
            input {
                class: if error.read().is_some() { "cell-input has-error" } else { "cell-input" },
                "aria-label": "{label}",
                placeholder: "—",
                value: "{draft}",
                disabled: *saving.read(),
                oninput: move |event| {
                    draft.set(event.value());
                    saved.set(false);
                    error.set(None);
                },
                onchange: move |_| on_change(),
            }
            if *saving.read() {
                span { class: "cell-status", "Enregistrement…" }
            } else if let Some(message) = error.read().as_ref() {
                button { class: "cell-retry", title: "{message}", "aria-label": "Réessayer l'enregistrement",
                    onclick: move |_| on_retry(), "Réessayer"
                }
            } else if *saved.read() {
                span { class: "cell-status is-saved", "Enregistré" }
            }
        }
    }
}
