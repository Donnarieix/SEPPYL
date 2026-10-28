use dioxus::prelude::*;
use crate::api::matrix::{self, Matrix};
use super::{dialogs::{ActivityDialog, PlayerDialog}, layout::Header};

#[derive(Clone, Copy, PartialEq)]
enum CreationKind {
    Player,
    Activity,
}

#[component]
pub fn Home() -> Element {
    let mut session = use_context::<Signal<Option<String>>>();
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
    let snapshot = data.read().clone();
    let can_create_activity = matches!(snapshot.as_ref(), Some(Ok(matrix)) if !matrix.players.is_empty());

    rsx! {
        div { class: "app",
            Header {}
            header { class: "app-header legacy-header",
                div { class: "app-header-content",
                    div { class: "wordmark", "SEPPYL" }
                    span { class: "header-divider" }
                    span { class: "header-section", "Résultats" }
                    button { class: "header-logout", onclick: move |_| {
                        crate::session::clear();
                        session.set(None);
                        navigator.push(crate::routes::Route::Login {});
                    }, "Se déconnecter" }
                }
            }

            main { class: "page",
                div { class: "page-heading",
                    div {
                        h1 { "Tableau des résultats" }
                        p { "Suivi des joueuses par activité" }
                    }
                    div { class: "heading-actions",
                        button { class: "button outline", onclick: move |_| dialog.set(Some(CreationKind::Player)),
                            span { class: "plus", "+" }
                            " Ajouter une joueuse"
                        }
                        button { class: "button solid",
                            disabled: !can_create_activity,
                            onclick: move |_| dialog.set(Some(CreationKind::Activity)),
                            span { class: "plus", "+" }
                            " Nouvelle activité"
                        }
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

                            if matrix.players.is_empty() {
                                div { class: "empty-state",
                                    div { class: "empty-title", "Commencez par ajouter une joueuse" }
                                    p { "Les joueuses occupent les lignes du tableau. Vous pourrez ensuite créer des activités et saisir leurs résultats." }
                                    button { class: "button solid", onclick: move |_| dialog.set(Some(CreationKind::Player)),
                                        "Ajouter une joueuse"
                                    }
                                }
                            } else if matrix.activities.is_empty() {
                                div { class: "empty-state",
                                    div { class: "empty-title", "Ajoutez votre première activité" }
                                    p { "La liste des joueuses est prête. Créez une activité pour ajouter une colonne de résultats." }
                                    button { class: "button solid", onclick: move |_| dialog.set(Some(CreationKind::Activity)),
                                        "Créer une activité"
                                    }
                                }
                            } else {
                                MatrixTable {
                                    matrix: matrix.clone(),
                                    filter: search.read().clone(),
                                    token: auth_token.clone(),
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
                ActivityDialog {
                    token: auth_token.clone(),
                    on_cancel: move |_| dialog.set(None),
                    on_created: move |_| { dialog.set(None); data.restart(); },
                }
            }
        }
    }
}

#[component]
fn MatrixTable(matrix: Matrix, filter: String, token: String) -> Element {
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
                                span { class: "activity-date", "{activity.date.get(..10).unwrap_or(&activity.date)}" }
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
                        }
                    }
                }
            }
            if visible.is_empty() {
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
