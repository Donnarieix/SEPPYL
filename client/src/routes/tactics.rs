use dioxus::prelude::*;
use crate::api::tactics::{self, Diagram, DrawingObject};
use super::{Route, layout::Header};

#[component]
pub fn Tactics() -> Element {
    let session = use_context::<Signal<Option<String>>>();
    let navigator = use_navigator();
    let token = session.read().clone();
    let fetch_token = token.clone();
    let mut diagrams = use_resource(move || {
        let token = fetch_token.clone();
        async move {
            match token {
                Some(token) => tactics::list(&token).await,
                None => Err("Connexion requise.".into()),
            }
        }
    });
    let mut selected_id = use_signal(|| None::<String>);
    let mut name = use_signal(|| "Nouveau schéma".to_string());
    let mut layout = use_signal(|| "football".to_string());
    let mut objects_json = use_signal(|| "[]".to_string());
    let mut busy = use_signal(|| false);
    let mut deletion_confirm = use_signal(|| false);
    let mut notice = use_signal(|| None::<String>);

    if token.is_none() {
        return rsx! {
            div { class: "centered",
                h1 { "Tableau tactique" }
                p { "Connectez-vous pour concevoir vos schémas." }
                button { class: "button solid", onclick: move |_| { navigator.push(Route::Login {}); }, "Connexion" }
            }
        };
    }
    let token = token.unwrap_or_default();
    let token_save = token.clone();
    let token_delete = token.clone();
    let draft_id = selected_id.read().clone().unwrap_or_else(|| "new".to_string());
    let draft_key = format!("{}|{}", draft_id, layout.read());
    let initial_objects = objects_json.read().clone();

    rsx! {
        div { class: "app",
            Header {}
            main { class: "page tactics-page",
                div { class: "page-heading",
                    div {
                        h1 { "Tableau tactique" }
                        p { "Dessinez vos exercices avec des joueuses, adversaires, ballons, plots et déplacements." }
                    }
                    button { class: "button solid",
                        onclick: move |_| {
                            selected_id.set(None);
                            name.set("Nouveau schéma".into());
                            layout.set("football".into());
                            objects_json.set("[]".into());
                            deletion_confirm.set(false);
                            notice.set(None);
                        },
                        "+ Nouveau schéma"
                    }
                }
                if let Some(message) = notice.read().as_ref() {
                    div { class: "inline-notice", role:"status", "{message}" }
                }
                div { class: "tactics-layout",
                    aside { class: "panel tactics-library",
                        div { class: "panel-head",
                            div { h2 { "Mes schémas" }
                                p { "Choisissez un schéma pour le modifier." } }
                        }
                        match diagrams.read().clone() {
                            Some(Ok(items)) => rsx! {
                                if items.is_empty() {
                                    div { class:"panel-empty", "Aucun schéma enregistré." }
                                }
                                for diagram in items {
                                    button {
                                        key: "{diagram.id}",
                                        class: if selected_id.read().as_deref() == Some(diagram.id.as_str()) {
                                            "tactics-list-item active"
                                        } else {
                                            "tactics-list-item"
                                        },
                                        onclick: move |_| {
                                            selected_id.set(Some(diagram.id.clone()));
                                            name.set(diagram.name.clone());
                                            layout.set(diagram.layout.clone());
                                            objects_json.set(
                                                serde_json::to_string(&diagram.objects)
                                                    .unwrap_or_else(|_| "[]".into())
                                            );
                                            deletion_confirm.set(false);
                                            notice.set(None);
                                        },
                                        "{diagram.name}"
                                    }
                                }
                            },
                            Some(Err(message)) => rsx! { div { class: "panel-empty", "{message}" } },
                            None => rsx! { div { class: "panel-empty", "Chargement…" } },
                        }
                    }
                    section { class: "panel tactics-workspace",
                        div { class: "tactics-settings",
                            div { class: "field",
                                label { "Nom du schéma" }
                                input { value:"{name}", maxlength:"150",
                                    oninput:move |e|name.set(e.value()) }
                            }
                            div { class: "field",
                                label { "Terrain" }
                                select { value:"{layout}",
                                    onchange:move |e|layout.set(e.value()),
                                    option { value:"football", "Football" }
                                    option { value:"volleyball", "Volleyball" }
                                    option { value:"blank", "Terrain neutre" }
                                }
                            }
                        }
                        div { class: "seppyl-tactics-board",
                            key: "{draft_key}",
                            "data-tactics-initial": "{initial_objects}",
                            "data-tactics-layout": "{layout}",
                            div { class: "tactics-tools", role:"toolbar", "aria-label":"Outils de dessin",
                                button { class:"selected", r#type:"button", "data-tactic-tool":"player", "Joueuse" }
                                button { r#type:"button", "data-tactic-tool":"opponent", "Adversaire" }
                                button { r#type:"button", "data-tactic-tool":"ball", "Ballon" }
                                button { r#type:"button", "data-tactic-tool":"cone", "Plot" }
                                button { r#type:"button", "data-tactic-tool":"arrow", "Flèche" }
                                button { r#type:"button", "data-tactic-tool":"erase", "Effacer" }
                                button { r#type:"button", "data-tactic-remove":"true", "Supprimer la sélection" }
                            }
                            svg { class:"tactics-canvas",
                                "data-tactics-svg":"true",
                                view_box:"0 0 800 500",
                                "aria-label":"Terrain tactique interactif",
                            }
                            textarea { class:"rich-hidden-input", "data-tactics-output":"true",
                                "aria-hidden":"true", tabindex:"-1",
                                oninput:move |event|objects_json.set(event.value()) }
                        }
                        div { class: "tactics-actions",
                            p { class:"muted-small", "Cliquez pour placer un symbole. Glissez-le pour le déplacer. Pour une flèche, cliquez le départ puis l’arrivée." }
                            button { class:"button solid", disabled:*busy.read(),
                                onclick:move |_|{
                                    if *busy.read(){return;}
                                    let parsed: Result<Vec<DrawingObject>, _> =
                                        serde_json::from_str(&objects_json.read());
                                    let Ok(objects)=parsed else {
                                        notice.set(Some("Le schéma contient des données invalides.".into()));
                                        return;
                                    };
                                    let label=name.read().trim().to_string();
                                    if label.is_empty() {
                                        notice.set(Some("Donnez un nom au schéma.".into()));
                                        return;
                                    }
                                    let layout_value=layout.read().clone();
                                    let id=selected_id.read().clone();
                                    let token=token_save.clone();
                                    busy.set(true);
                                    notice.set(None);
                                    spawn(async move {
                                        match tactics::save(&token,id.as_deref(),&label,&layout_value,&objects).await {
                                            Ok(saved)=>{
                                                selected_id.set(Some(saved.id));
                                                notice.set(Some("Schéma enregistré.".into()));
                                                diagrams.restart();
                                            }
                                            Err(message)=>notice.set(Some(message)),
                                        }
                                        busy.set(false);
                                    });
                                },
                                if *busy.read() { "Enregistrement…" } else { "Enregistrer le schéma" }
                            }
                        }
                        if let Some(id) = selected_id.read().clone() {
                            div { class:"tactics-delete",
                                if *deletion_confirm.read() {
                                    span { "Supprimer définitivement ce schéma ?" }
                                    button { class:"button danger small", disabled:*busy.read(),
                                        onclick:{
                                            let id=id.clone();
                                            let token=token_delete.clone();
                                            move |_|{
                                                let id=id.clone();
                                                let token=token.clone();
                                                busy.set(true);
                                                spawn(async move {
                                                    match tactics::remove(&token,&id).await {
                                                        Ok(())=>{
                                                            selected_id.set(None);
                                                            name.set("Nouveau schéma".into());
                                                            objects_json.set("[]".into());
                                                            layout.set("football".into());
                                                            diagrams.restart();
                                                            notice.set(Some("Schéma supprimé.".into()));
                                                        }
                                                        Err(message)=>notice.set(Some(message)),
                                                    }
                                                    deletion_confirm.set(false);
                                                    busy.set(false);
                                                });
                                            }
                                        },
                                        "Confirmer"
                                    }
                                    button { class:"button quiet small", onclick:move |_|deletion_confirm.set(false), "Annuler" }
                                } else {
                                    button { class:"remove-link", onclick:move |_|deletion_confirm.set(true), "Supprimer le schéma" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
