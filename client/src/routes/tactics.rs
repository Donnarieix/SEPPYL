use dioxus::prelude::*;
use crate::api::tactics::{self,Board,Folder};
use super::{Route,layout::Header};

#[component]
pub fn TacticsPage()->Element{
    let session=use_context::<Signal<Option<String>>>();
    let nav=use_navigator();
    let token=session.read().clone();
    let load=token.clone();
    let mut library=use_resource(move || {
        let token=load.clone();
        async move{match token {Some(t)=>tactics::list(&t).await,
            None=>Err("Connectez-vous pour retrouver vos schémas.".to_owned())}}
    });
    let mut selected=use_signal(||None::<String>);
    let mut adding_folder=use_signal(||false);
    let mut folder_name=use_signal(String::new);
    let mut busy=use_signal(||false);
    let mut error=use_signal(||None::<String>);
    if token.is_none(){return rsx!{
        div { class:"centered",
            h1 { "Tableau tactique" }
            p { "Connectez-vous pour organiser et enregistrer vos schémas." }
            button { class:"button solid",onclick:move |_|{nav.push(Route::Login {});},"Se connecter" }
        }
    };}
    let token=token.unwrap_or_default();
    rsx!{
        div { class:"app",
            Header {}
            main { class:"page tactics-page",
                div { class:"page-heading",
                    div {
                        h1 { "Tableau tactique" }
                        p { "Dessinez vos exercices, organisez-les et exportez-les en image." }
                    }
                    button { class:"button solid",onclick:move |_|selected.set(Some("new".into())),"+ Nouveau schéma" }
                }
                if let Some(message)=error.read().as_ref(){p{class:"form-error",role:"alert","{message}"}}
                match library.read().clone(){
                    Some(Ok(data))=>rsx!{
                        if let Some(ref id)=*selected.read() {
                            TacticEditor {
                                key:"{id}",
                                token:token.clone(),
                                board:data.boards.iter().find(|board|&board.id==id).cloned(),
                                folders:data.folders.clone(),
                                on_cancel:move |_|selected.set(None),
                                on_saved:move |_|{
                                    selected.set(None);error.set(None);library.restart();
                                },
                                on_error:move |e:String|error.set(Some(e)),
                            }
                        } else {
                            div { class:"tactics-library",
                                section { class:"panel",
                                    div { class:"panel-head",
                                        div { h2 { "Dossiers" } p { "Organisez vos exercices par thème." } }
                                        button { class:"button outline small",onclick:move |_|{let next=!*adding_folder.read();adding_folder.set(next);},"+ Dossier" }
                                    }
                                    if *adding_folder.read(){
                                        form { class:"folder-form",
                                            onsubmit:move |e|{
                                                e.prevent_default();
                                                let name=folder_name.read().trim().to_string();
                                                if name.is_empty()||*busy.read(){return;}
                                                let token=token.clone();
                                                busy.set(true);
                                                spawn(async move{
                                                    match tactics::create_folder(&token,&name,None).await {
                                                        Ok(())=>{folder_name.set(String::new());adding_folder.set(false);library.restart();}
                                                        Err(e)=>error.set(Some(e)),
                                                    }
                                                    busy.set(false);
                                                });
                                            },
                                            input { value:"{folder_name}",placeholder:"Nom du dossier",required:true,
                                                oninput:move |e|folder_name.set(e.value()) }
                                            button { class:"button solid small",r#type:"submit",disabled:*busy.read(),"Créer" }
                                        }
                                    }
                                    if data.folders.is_empty(){
                                        p { class:"panel-empty","Aucun dossier. Vos schémas restent accessibles ci-dessous." }
                                    } else {
                                        div { class:"tactics-folders",
                                            for folder in &data.folders {
                                                div { class:"tactics-folder",key:"{folder.id}",
                                                    span { "📁" }
                                                    strong { "{folder.name}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                section { class:"panel",
                                    div { class:"panel-head",
                                        div { h2 { "Mes schémas" } p { "Sélectionnez un schéma pour le modifier." } }
                                    }
                                    if data.boards.is_empty() {
                                        div { class:"panel-empty",
                                            p { "Aucun schéma enregistré." }
                                            button { class:"button solid",onclick:move |_|selected.set(Some("new".into())),"Créer un schéma" }
                                        }
                                    } else {
                                        div { class:"tactics-cards",
                                            for board in &data.boards {
                                                button { class:"tactics-card",key:"{board.id}",
                                                    onclick:{
                                                        let id=board.id.clone();
                                                        move |_|selected.set(Some(id.clone()))
                                                    },
                                                    span { class:"tactics-card-icon","♙" }
                                                    strong { "{board.name}" }
                                                    span { class:"muted-small",
                                                        if board.is_public { "Public" } else { "Privé" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e))=>rsx!{div{class:"load-state",p{"{e}"}button{class:"button outline",onclick:move |_|library.restart(),"Réessayer"}}},
                    None=>rsx!{div{class:"load-state","Chargement des schémas…"}},
                }
            }
        }
    }
}

#[component]
pub fn TacticEditor(
 token:String,board:Option<Board>,folders:Vec<Folder>,
 on_cancel:EventHandler<()>,on_saved:EventHandler<()>,on_error:EventHandler<String>,
)->Element{
    let mut name=use_signal(||board.as_ref().map(|b|b.name.clone()).unwrap_or_else(||"Nouvel exercice".into()));
    let mut folder=use_signal(||board.as_ref().and_then(|b|b.folder_id.clone()).unwrap_or_default());
    let mut published=use_signal(||board.as_ref().map(|b|b.is_public).unwrap_or(false));
    let initial=board.as_ref().map(|b|b.content.clone()).unwrap_or_else(||tactics::EMPTY.into());
    let mut content=use_signal(||initial.clone());
    let mut busy=use_signal(||false);
    let mut confirm=use_signal(||false);
    let existing=board.as_ref().map(|b|b.id.clone());
    let delete_id=existing.clone();
    let token_for_delete=token.clone();
    rsx!{
        div { class:"tactic-edit-page",
            div { class:"tactic-edit-header",
                button { class:"button quiet small",onclick:move |_|on_cancel.call(()),"← Retour" }
                div { class:"field",
                    label { "Nom du schéma" }
                    input { value:"{name}",maxlength:"150",required:true,oninput:move |e|name.set(e.value()) }
                }
                div { class:"field",
                    label { "Dossier" }
                    select { value:"{folder}",onchange:move |e|folder.set(e.value()),
                        option { value:"","Sans dossier" }
                        for f in &folders { option { value:"{f.id}","{f.name}" } }
                    }
                }
                label { class:"tactic-public-check",
                    input { r#type:"checkbox",checked:*published.read(),onchange:move |e|published.set(e.checked()) }
                    "Rendre public"
                }
            }
            div { class:"tactic-toolbar","aria-label":"Outils tactiques",
                button { "data-tactic-tool":"player",class:"active",title:"Joueuse","Joueuse" }
                button { "data-tactic-tool":"opponent",title:"Adversaire","Adversaire" }
                button { "data-tactic-tool":"ball",title:"Ballon","Ballon" }
                button { "data-tactic-tool":"cone",title:"Plot","Plot" }
                button { "data-tactic-tool":"arrow",title:"Flèche","Flèche" }
                button { "data-tactic-tool":"select",title:"Déplacer","Déplacer" }
                button { "data-tactic-tool":"erase",title:"Effacer un élément","Effacer" }
                button { "data-tactic-clear":"true",title:"Tout effacer","Vider" }
                button { "data-tactic-export":"true",title:"Exporter l'image","PNG" }
                select { "data-tactic-pitch":"true","aria-label":"Type de terrain",
                    option { value:"football","Terrain entier" }
                    option { value:"half_pitch","Demi-terrain" }
                    option { value:"blank","Fond vierge" }
                }
            }
            div { class:"tactic-canvas-shell",
                "data-tactics-editor":"true",
                "data-initial":"{initial}",
                canvas { "data-tactic-canvas":"true",width:"960",height:"600","aria-label":"Tableau de dessin tactique" }
                textarea {
                    class:"rich-output","data-tactic-output":"true",
                    value:"{content}",
                    tabindex:"-1",
                    oninput:move |e|content.set(e.value())
                }
            }
            p { class:"muted-small", "Cliquez sur le terrain pour placer des éléments. Avec l'outil Flèche, glissez du départ à l'arrivée. L'outil Déplacer permet de repositionner un élément." }
            div { class:"tactic-actions",
                if existing.is_some() {
                    if *confirm.read() {
                        button { class:"button danger",disabled:*busy.read(),
                            onclick:{
                                let token=token_for_delete.clone();
                                let id=delete_id.clone().unwrap_or_default();
                                move |_|{
                                    let token=token.clone();let id=id.clone();
                                    busy.set(true);
                                    spawn(async move{
                                        match tactics::delete(&token,&id).await{
                                            Ok(())=>on_saved.call(()),
                                            Err(e)=>on_error.call(e),
                                        }
                                        busy.set(false);confirm.set(false);
                                    });
                                }
                            },"Confirmer la suppression" }
                        button { class:"button quiet",onclick:move |_|confirm.set(false),"Annuler" }
                    }else{
                        button { class:"button quiet",onclick:move |_|confirm.set(true),"Supprimer" }
                    }
                }
                button { class:"button solid",disabled:*busy.read(),
                    onclick:{
                        let token=token.clone();
                        let id=existing.clone();
                        move |_|{
                            if *busy.read(){return;}
                            let name_value=name.read().trim().to_string();
                            let folder_value=folder.read().clone();
                            let content_value=content.read().clone();
                            let public_value=*published.read();
                            if name_value.is_empty(){on_error.call("Le nom est obligatoire.".into());return;}
                            let token=token.clone();let id=id.clone();
                            busy.set(true);
                            spawn(async move{
                                match tactics::save(&token,id.as_deref(),&name_value,
                                    if folder_value.is_empty(){None}else{Some(&folder_value)},
                                    &content_value,public_value).await{
                                        Ok(())=>on_saved.call(()),
                                        Err(e)=>on_error.call(e),
                                }
                                busy.set(false);
                            });
                        }
                    },
                    if *busy.read(){"Enregistrement…"}else{"Enregistrer le schéma"}
                }
            }
        }
    }
}
