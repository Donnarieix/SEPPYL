use dioxus::prelude::*;
use crate::api::tactics::{self, DrawingObject};
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
    let mut objects_json = use_signal(|| "[]".to_string());
    let mut busy = use_signal(|| false);
    let mut deletion_confirm = use_signal(|| false);
    let mut notice = use_signal(|| None::<String>);
    let mut revision = use_signal(|| 0_u64);

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
    let draft_key = format!("{}:{}",draft_id,*revision.read());
    let initial_objects = objects_json.read().clone();

    rsx! {
        div { class: "app",
            Header {}
            main { class: "page tactics-page",id:"tactics-studio",
                div { class: "page-heading",
                    div {
                        h1 { "Tableau tactique" }
                        p { "Construisez vos exercices sur un terrain, organisez les joueurs et tracez leurs déplacements." }
                    }
                    div { class:"tactics-heading-actions",
                        if let Some(message)=notice.read().as_ref(){
                            span { class:"tactics-save-notice",role:"status","{message}" }
                        }

                            button { class:"button solid tactics-save", disabled:*busy.read(),
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
                                    let layout_value="football".to_string();
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
                                if *busy.read() { "Enregistrement…" } else { "Enregistrer" }
                            }
                    button { class: "button solid",
                        onclick: move |_| {
                            selected_id.set(None);
                            name.set("Nouveau schéma".into());
                            objects_json.set("[]".into());
                            let next=*revision.read()+1;
                            revision.set(next);
                            deletion_confirm.set(false);
                            notice.set(None);
                        },
                        "+ Nouveau schéma"
                    }
                    button { class: "button outline tactics-fullscreen-toggle",
                        r#type:"button",
                        "data-tactics-fullscreen":"true",
                        "aria-pressed":"false",
                        title:"Afficher le studio sur tout l'écran (F pour basculer, Échap pour quitter)",
                        "Plein écran"
                    }
                    if let Some(id) = selected_id.read().clone() {
                        div { class:"tactics-delete-compact",
                            button {
                                class:"button outline small tactics-delete-trigger",
                                r#type:"button",
                                disabled:*busy.read(),
                                "aria-expanded":if *deletion_confirm.read(){"true"}else{"false"},
                                onclick:move |_|deletion_confirm.set(!*deletion_confirm.read()),
                                "Supprimer"
                            }
                            if *deletion_confirm.read() {
                                div { class:"tactics-delete-confirm",role:"alertdialog",
                                    "aria-label":"Confirmer la suppression du schéma",
                                    strong { "Supprimer définitivement ce schéma ?" }
                                    div { class:"tactics-delete-confirm-actions",
                                        button { class:"button quiet small",r#type:"button",
                                            onclick:move |_|deletion_confirm.set(false),"Annuler" }
                                        button { class:"button danger small",r#type:"button",disabled:*busy.read(),
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
                                                                let next=*revision.read()+1;
                                                                revision.set(next);
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
                                    }
                                }
                            }
                        }
                    }
                    }
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
                                            objects_json.set(
                                                serde_json::to_string(&diagram.objects)
                                                    .unwrap_or_else(|_| "[]".into())
                                            );
                                            let next=*revision.read()+1;
                                            revision.set(next);
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

                        }
                        div { class: "seppyl-tactics-board",
                            key: "{draft_key}",
                            "data-tactics-initial": "{initial_objects}",
                            "data-tactics-revision": "{revision}",

                            div { class:"tactics-tools",role:"toolbar","aria-label":"Outils de dessin",
                                div { class:"tactic-toolbar-section",
                                    span { class:"tactic-toolbar-label","Mode" }
                                    button { class:"selected",r#type:"button","data-tactic-tool":"select",
                                        title:"Sélectionner — S","Sélection" kbd { "S" } }
                                    button { r#type:"button","data-tactic-tool":"erase",
                                        title:"Gomme — E","Gomme" kbd { "E" } }
                                    button { r#type:"button","data-tactic-remove":"true",
                                        title:"Supprimer la sélection — Suppr","Supprimer" }
                                }
                                div { class:"tactic-toolbar-section",
                                    span { class:"tactic-toolbar-label","Éléments" }
                                    button { r#type:"button","data-tactic-tool":"player","Joueuse" kbd { "J" } }
                                    button { r#type:"button","data-tactic-tool":"ball","Ballon" kbd { "B" } }
                                    button { r#type:"button","data-tactic-tool":"cone","Plot" kbd { "P" } }
                                    button { r#type:"button","data-tactic-tool":"goal","But" kbd { "G" } }
                                }
                                div { class:"tactic-toolbar-section",
                                    span { class:"tactic-toolbar-label","Trajectoires" }
                                    button { r#type:"button","data-tactic-tool":"arrow","Droite" kbd { "A" } }
                                    button { r#type:"button","data-tactic-tool":"curve","Courbe" kbd { "C" } }
                                }
                                div { class:"tactic-toolbar-section tactic-line-style",
                                    span { class:"tactic-toolbar-label","Ligne" }
                                    button { r#type:"button","data-tactic-dash":"solid",class:"selected",
                                        title:"Trait continu","Continu" }
                                    button { r#type:"button","data-tactic-dash":"dashed",
                                        title:"Trait discontinu (tirets)","Tirets" }

                                }
                            }
                            div { class:"tactic-colors",
                                div { class:"tactic-colors-heading",
                                    strong { "Couleur des objets" }
                                    span { "S'applique à la sélection ou aux prochains objets." }
                                }
                                div { class:"tactic-palette",
                                    for color in ["#2455b6","#c6403c","#f8f8f5","#ee9228","#21a179","#7d4cc2","#f2c94c","#202f43"] {
                                        button { key:"{color}",class:"tactic-swatch",r#type:"button",
                                            "data-tactic-color":"{color}",style:"--swatch:{color}",
                                            title:"Couleur {color}","aria-label":"Couleur {color}" }
                                    }
                                    label { class:"tactic-custom-color",
                                        span { "Personnalisée" }
                                        input { r#type:"color",value:"#2455b6",
                                            "data-tactic-custom":"true","aria-label":"Couleur personnalisée" }
                                    }
                                }
                            }
                            div { class:"tactic-sizes",
                                div { class:"tactic-colors-heading",
                                    strong { "Taille" }
                                    span { "Modifier la sélection ou la taille des nouveaux objets." }
                                }
                                input { r#type:"range",min:"8",max:"65",value:"22",
                                    "data-tactic-size":"true","aria-label":"Taille des objets" }
                                output { "data-tactic-size-label":"true", "22" }
                            }
                            div { class:"tactic-editor-hint",
                                span { class:"tactic-selection-count","data-selection-count":"true","0 sélectionné(s)" }
                                span { "Clic droit : couleurs" }
                                span { "Molette : taille" }
                                span { "Suppr / Retour : effacer" }
                                span { "Échap : désélectionner" }
                                span { "Maj + clic : multi-sélection" }
                            }
                            div { class:"tactic-shortcuts-help",
                                details { summary { "Raccourcis clavier" }
                                    div { class:"tactic-shortcuts-grid",
                                        span { "S — sélectionner" } span { "J — joueuse" }
                                        span { "B — ballon" } span { "P — plot" }
                                        span { "G — mini-but" } span { "A — flèche droite" }
                                        span { "C — courbe" } span { "F — plein écran" } span { "E — gomme" }
                                        span { "Suppr / Retour — supprimer" } span { "Échap — annuler la sélection" }
                                    }
                                }
                            }
                            svg { class:"tactics-canvas",
                                "data-tactics-svg":"true",
                                view_box:"0 0 800 500",
                                tabindex:"0",
                                "aria-label":"Terrain tactique interactif",
                            }
                            div { class:"tactic-size-feedback","data-tactic-size-feedback":"true",
                                "aria-hidden":"true",
                                span { "Taille" }
                                strong { "22" }
                            }
                            div { class:"tactic-context-style","data-tactic-line-menu":"true",
                                span { class:"tactic-context-palette-head","Type de trait" }
                                button { r#type:"button","data-tactic-context-dash":"solid","Continu" }
                                button { r#type:"button","data-tactic-context-dash":"dashed","Tirets" }
                            }
                            div { class:"tactic-context-palette","data-tactic-menu":"true",
                                role:"group","aria-label":"Couleur de l'objet",
                                div { class:"tactic-context-palette-head","Couleur" }
                                for color in ["#2455b6","#c6403c","#f8f8f5","#ee9228","#21a179","#7d4cc2","#f2c94c","#202f43"]{
                                    button { r#type:"button",class:"tactic-swatch",
                                        "data-tactic-quick-color":"{color}",style:"--swatch:{color}",
                                        title:"Appliquer {color}","aria-label":"Appliquer {color}" }
                                }
                            }
                            textarea { class:"rich-hidden-input", "data-tactics-output":"true",
                                "aria-hidden":"true", tabindex:"-1",
                                oninput:move |event|objects_json.set(event.value()) }
                        }
                    }
                }
            }
        }
    }
}
