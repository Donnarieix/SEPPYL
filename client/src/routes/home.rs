use dioxus::prelude::*;
use crate::api::matrix::{self, Matrix};

#[component]
pub fn Home() -> Element {
    let session = use_context::<Signal<Option<String>>>();
    let navigator = use_navigator();
    let mut search = use_signal(String::new);
    let token = session.read().clone();

    let mut data = use_resource(move || {
        let token = token.clone();
        async move {
            match token {
                Some(token) => matrix::load(&token).await,
                None => Err("Connectez-vous pour consulter le tableau.".to_string()),
            }
        }
    });

    if session.read().is_none() {
        return rsx! {
            div { class: "center-message",
                h1 { "Bienvenue sur SEPPYL" }
                p { "Connectez-vous pour accéder aux résultats des joueuses." }
                button { class: "button primary", onclick: move |_| { navigator.push(crate::routes::Route::Login {}); }, "Se connecter" }
            }
        };
    }

    let snapshot = data.read().clone();

    rsx! {
        div { class: "app-shell",
            aside { class: "sidebar",
                div { class: "brand", span { class: "brand-symbol", "S" } div { strong { "SEPPYL" } small { "Espace de suivi" } } }
                div { class: "nav-section", "ESPACE DE TRAVAIL" }
                div { class: "nav-active", "▦   Tableau des résultats" }
                div { class: "sidebar-bottom",
                    span { class: "status-dot" }
                    "Espace connecté"
                    button { class: "logout", onclick: move |_| session.set(None), "Déconnexion" }
                }
            }
            main { class: "workspace",
                header { class: "page-header",
                    div {
                        div { class: "eyebrow", "VUE D'ENSEMBLE  /  RÉSULTATS" }
                        h1 { "Tableau des résultats" }
                        p { "Une ligne par joueuse, une colonne par activité. Modifiez une cellule pour enregistrer son résultat." }
                    }
                    button { class: "button secondary", onclick: move |_| data.restart(), "↻  Actualiser" }
                }
                match snapshot {
                    Some(Ok(matrix)) => rsx! {
                        div { class: "summary-row",
                            div { class: "metric", span { "JOUEUSES" } strong { "{matrix.players.len()}" } }
                            div { class: "metric", span { "ACTIVITÉS" } strong { "{matrix.activities.len()}" } }
                            div { class: "metric", span { "RÉSULTATS SAISIS" } strong { "{matrix.results.iter().filter(|r| r.result.as_ref().is_some_and(|value| !value.is_empty())).count()}" } }
                        }
                        div { class: "table-panel",
                            div { class: "table-toolbar",
                                div { h2 { "Feuille des activités" } span { class: "muted", "Modifications enregistrées à la sortie du champ" } }
                                input {
                                    class: "search",
                                    r#type: "search",
                                    placeholder: "Rechercher une joueuse…",
                                    value: "{search}",
                                    oninput: move |e| search.set(e.value()),
                                }
                            }
                            if matrix.players.is_empty() || matrix.activities.is_empty() {
                                div { class: "empty-state",
                                    h3 { "Le tableau est encore vide" }
                                    p { "Ajoutez des joueuses et des activités dans les données du projet pour les voir ici." }
                                }
                            } else {
                                MatrixTable {
                                    matrix,
                                    filter: search.read().clone(),
                                    token: session.read().clone().unwrap_or_default(),
                                    on_saved: move |_| data.restart(),
                                }
                            }
                        }
                        div { class: "footnote", "Les cases vides indiquent qu'aucun résultat n'a été enregistré. Les données proviennent de l'API SEPPYL." }
                    },
                    Some(Err(message)) => rsx! {
                        div { class: "error-panel",
                            h2 { "Impossible d'afficher les résultats" }
                            p { "{message}" }
                            button { class: "button secondary", onclick: move |_| data.restart(), "Réessayer" }
                        }
                    },
                    None => rsx! { div { class: "loading-panel", "Chargement des résultats…" } },
                }
            }
        }
    }
}

#[component]
fn MatrixTable(matrix: Matrix, filter: String, token: String, on_saved: EventHandler<()>) -> Element {
    let mut players = matrix.players.clone();
    players.sort_by(|a, b| a.number.cmp(&b.number).then_with(|| a.lastname.cmp(&b.lastname)));
    let mut activities = matrix.activities.clone();
    activities.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.name.cmp(&b.name)));
    let query = filter.trim().to_lowercase();
    let visible: Vec<_> = players.into_iter().filter(|p| {
        format!("{} {} {}", p.firstname, p.lastname, p.number).to_lowercase().contains(&query)
    }).collect();

    rsx! {
        div { class: "table-scroll",
            table { class: "results-table",
                thead {
                    tr {
                        th { class: "sticky-player", "JOUEUSE" }
                        for activity in &activities {
                            th { key: "{activity.id}",
                                span { class: "column-name", "{activity.name}" }
                                span { class: "column-date", "{activity.date.get(..10).unwrap_or(&activity.date)}" }
                            }
                        }
                    }
                }
                tbody {
                    for player in &visible {
                        tr { key: "{player.id}",
                            th { class: "sticky-player",
                                div { class: "player-info",
                                    span { class: "player-number", "{player.number:02}" }
                                    div {
                                        strong { "{player.firstname} {player.lastname}" }
                                        small { "Joueuse #{player.number}" }
                                    }
                                }
                            }
                            for activity in &activities {
                                td { key: "{activity.id}",
                                    ResultCell {
                                        player_id: player.id.clone(),
                                        activity_id: activity.id.clone(),
                                        token: token.clone(),
                                        initial: matrix.results.iter()
                                            .find(|r| r.player_id == player.id && r.activity_id == activity.id)
                                            .and_then(|r| r.result.clone())
                                            .unwrap_or_default(),
                                        on_saved: move |_| on_saved.call(()),
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if visible.is_empty() {
                div { class: "empty-state", "Aucune joueuse ne correspond à votre recherche." }
            }
        }
    }
}

#[component]
fn ResultCell(
    player_id: String,
    activity_id: String,
    token: String,
    initial: String,
    on_saved: EventHandler<()>,
) -> Element {
    let mut value = use_signal(|| initial.clone());
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        div { class: "cell-wrapper",
            input {
                class: if error.read().is_some() { "result-input invalid" } else { "result-input" },
                "aria-label": "Résultat",
                placeholder: "—",
                value: "{value}",
                disabled: *saving.read(),
                oninput: move |event| value.set(event.value()),
                onchange: move |_| {
                    let updated = value.read().clone();
                    if updated == initial { return; }
                    let token = token.clone();
                    let player_id = player_id.clone();
                    let activity_id = activity_id.clone();
                    saving.set(true);
                    error.set(None);
                    spawn(async move {
                        match matrix::save(&token, &player_id, &activity_id, &updated).await {
                            Ok(()) => on_saved.call(()),
                            Err(message) => error.set(Some(message)),
                        }
                        saving.set(false);
                    });
                },
            }
            if *saving.read() { span { class: "cell-indicator", "…" } }
            if let Some(message) = error.read().as_ref() {
                span { class: "cell-error", title: "{message}", "!" }
            }
        }
    }
}
