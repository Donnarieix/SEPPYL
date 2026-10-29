use dioxus::prelude::*;
use crate::api::catalog::{self, Activity};
use super::{Route,layout::Header,rich_editor::RichTextEditor};

#[component]
pub fn Activities()->Element{
    let session=use_context::<Signal<Option<String>>>();
    let nav=use_navigator();
    let token=session.read().clone();
    let fetched=token.clone();
    let mut data=use_resource(move||{let token=fetched.clone();async move {
        match token {Some(token)=>catalog::list(&token).await,None=>Err("Connexion requise.".into())}
    }});
    let mut creating=use_signal(||false);
    let mut error=use_signal(||None::<String>);
    if token.is_none(){return rsx!{div{class:"centered",p{"Connectez-vous."}
        button{class:"button solid",onclick:move |_|{nav.push(Route::Login{});},"Connexion"}}};}
    let token=token.unwrap_or_default();
    rsx!{
        div{class:"app",
            Header{}
            main{class:"page",
                div{class:"page-heading",
                    div{h1{"Bibliothèque d'activités"} p{"Définissez vos exercices une fois, puis réutilisez-les dans les entraînements et le tableur."}}
                    div { class:"heading-actions",
                        Link { to: Route::Tactics {}, class:"button outline", "Tableau tactique" }
                        button{class:"button solid",onclick:move |_|creating.set(true),"+ Nouvelle activité"}
                    }
                }
                if let Some(message)=error.read().as_ref(){div{class:"form-error",role:"alert","{message}"}}
                match data.read().clone(){
                    Some(Ok(activities))=>rsx!{
                        if activities.is_empty(){
                            div{class:"empty-state dashboard-empty",
                                h2{"Aucune activité"}
                                p{"Créez une activité avec ses consignes, son image et son matériel."}
                                button{class:"button solid",onclick:move |_|creating.set(true),"Créer une activité"}
                            }
                        }else{
                            div{class:"catalog-grid",
                                for item in activities{
                                    ActivityCard{key:"{item.id}",activity:item,token:token.clone(),
                                        on_saved:move |_|data.restart(),
                                        on_error:move |message|error.set(Some(message))}
                                }
                            }
                        }
                    },
                    Some(Err(message))=>rsx!{div{class:"load-state",p{"{message}"}button{class:"button outline",onclick:move |_|data.restart(),"Réessayer"}}},
                    None=>rsx!{div{class:"load-state","Chargement des activités…"}},
                }
            }
            if *creating.read(){
                ActivityEditor{token:token.clone(),activity:None,
                    on_cancel:move |_|creating.set(false),
                    on_saved:move |_|{creating.set(false);data.restart();},
                    on_error:move |message|error.set(Some(message))}
            }
        }
    }
}
#[component]
fn ActivityCard(activity:Activity,token:String,on_saved:EventHandler<()>,on_error:EventHandler<String>)->Element{
    let mut editing=use_signal(||false);
    let mut confirm=use_signal(||false);
    let mut busy=use_signal(||false);
    let mut mat_name=use_signal(String::new);
    let mut quantity=use_signal(||"1".to_string());
    rsx!{
        article{class:"catalog-card",
            if let Some(url)=activity.image_url.as_deref(){
                img{class:"catalog-thumbnail",src:"{catalog::image_src(url)}",alt:"Illustration de {activity.name}"}
            }else{
                div{class:"catalog-thumbnail catalog-no-image","Sans image"}
            }
            div{class:"catalog-content",
                h2{"{activity.name}"}
                if let Some(html)=activity.description_html.as_deref() {
                    div { class:"catalog-description rich-description", dangerous_inner_html: html }
                } else if !activity.description.is_empty() {
                    p { class:"catalog-description","{activity.description}" }
                } else {
                    p { class:"catalog-description empty","Aucune description" }
                }
                div{class:"catalog-equipment",
                    strong{"Matériel"}
                    if activity.materials.is_empty(){span{class:"muted-small","Aucun"}}
                    for material in &activity.materials{
                        div{class:"material-row",key:"{material.id}",
                            MaterialEditor{
                                token:token.clone(),activity_id:activity.id.clone(),material:material.clone(),
                                on_saved,on_error
                            }
                            button{class:"remove-link",disabled:*busy.read(),
                                onclick:{
                                    let token=token.clone();let id=activity.id.clone();let material_id=material.id.clone();
                                    move |_|{
                                        let token=token.clone();let id=id.clone();let material_id=material_id.clone();
                                        busy.set(true);
                                        spawn(async move{
                                            match catalog::remove_material(&token,&id,&material_id).await{
                                                Ok(())=>on_saved.call(()),Err(e)=>on_error.call(e),
                                            }
                                            busy.set(false);
                                        });
                                    }
                                },
                                "Retirer"}
                        }
                    }
                    form{class:"add-material",
                        onsubmit:{
                            let token=token.clone();let id=activity.id.clone();
                            move |event|{
                                event.prevent_default();
                                if *busy.read(){return;}
                                let name=mat_name.read().trim().to_string();
                                let count=quantity.read().parse::<i32>().unwrap_or(0);
                                if name.is_empty()||count<=0{on_error.call("Nom de matériel et quantité positive obligatoires.".into());return;}
                                let token=token.clone();let id=id.clone();
                                busy.set(true);
                                spawn(async move{
                                    match catalog::add_material(&token,&id,&name,count).await{
                                        Ok(())=>{mat_name.set(String::new());quantity.set("1".into());on_saved.call(());}
                                        Err(e)=>on_error.call(e),
                                    }
                                    busy.set(false);
                                });
                            }
                        },
                        input{placeholder:"Matériel",value:"{mat_name}",required:true,
                            "aria-label":"Nom du matériel",oninput:move |e|mat_name.set(e.value())}
                        input{r#type:"number",min:"1",value:"{quantity}",
                            "aria-label":"Quantité",oninput:move |e|quantity.set(e.value())}
                        button{class:"button outline small",r#type:"submit",disabled:*busy.read(),"Ajouter"}
                    }
                }
                div{class:"catalog-actions",
                    button{class:"button outline small",onclick:move |_|editing.set(true),"Modifier"}
                    if *confirm.read(){
                        button{class:"button danger small",disabled:*busy.read(),
                            onclick:{
                                let token=token.clone();let id=activity.id.clone();
                                move |_|{
                                    let token=token.clone();let id=id.clone();
                                    busy.set(true);
                                    spawn(async move{
                                        match catalog::remove(&token,&id).await{
                                            Ok(())=>on_saved.call(()),Err(e)=>on_error.call(e),
                                        }
                                        confirm.set(false);busy.set(false);
                                    });
                                }
                            },"Confirmer"}
                        button{class:"button quiet small",onclick:move |_|confirm.set(false),"Annuler"}
                    }else{
                        button{class:"button quiet small",onclick:move |_|confirm.set(true),"Supprimer"}
                    }
                }
            }
        }
        if *editing.read(){
            ActivityEditor{token:token.clone(),activity:Some(activity.clone()),
                on_cancel:move |_|editing.set(false),
                on_saved:move |_id: String|{editing.set(false);on_saved.call(());},
                on_error}
        }
    }
}

#[component]
pub fn ActivityEditor(
    token:String,activity:Option<Activity>,
    on_cancel:EventHandler<()>,on_saved:EventHandler<String>,on_error:EventHandler<String>
)->Element{
    let mut name=use_signal(||activity.as_ref().map(|a|a.name.clone()).unwrap_or_default());
    let mut description=use_signal(||activity.as_ref().map(|a|a.description.clone()).unwrap_or_default());
    let mut description_html=use_signal(||activity.as_ref().and_then(|a|a.description_html.clone()));
    let mut team_id=use_signal(||activity.as_ref().and_then(|a|a.team_id.clone()).unwrap_or_default());
    let team_token=token.clone();
    let available_teams=use_resource(move ||{
        let token=team_token.clone();
        async move{
            let mut teams=Vec::new();
            for club in organization::clubs(&token).await?{
                for team in organization::teams(&token,&club.id).await?{
                    teams.push((team.id,format!("{} · {}",club.name,team.name)));
                }
            }
            Ok::<Vec<(String,String)>,String>(teams)
        }
    });
    let mut image=use_signal(||activity.as_ref().and_then(|a|a.image_url.clone()).unwrap_or_default());
    let mut busy=use_signal(||false);
    let mut uploading=use_signal(||false);
    let mut error=use_signal(||None::<String>);
    let id=activity.as_ref().map(|a|a.id.clone());
    let editing=id.is_some();
    let token_for_submit = token.clone();
    let token_for_upload = token.clone();
    rsx!{
        div{class:"dialog-backdrop",
            div{class:"dialog-panel activity-dialog",role:"dialog","aria-modal":"true",
                div{class:"dialog-header",
                    div{h2{if editing{"Modifier l'activité"}else{"Nouvelle activité"}} p{"Une activité est réutilisable partout dans SEPPYL."}}
                    button{class:"icon-button",onclick:move |_|on_cancel.call(()),"×"}
                }
                form{
                    onsubmit:move |e|{
                        e.prevent_default();
                        if *busy.read()||*uploading.read(){return;}
                        let n=name.read().trim().to_string();
                        let d=description.read().clone();
                        let html=description_html.read().clone();
                        let selected_team=team_id.read().clone();
                        let i=image.read().trim().to_string();
                        if n.is_empty(){error.set(Some("Le nom de l'activité est obligatoire.".into()));return;}
                        let token=token_for_submit.clone();let id=id.clone();
                        busy.set(true);
                        spawn(async move{
                            match catalog::upsert(&token,id.as_deref(),&n,&d,html.as_deref(),if i.is_empty(){None}else{Some(&i)},if selected_team.is_empty(){None}else{Some(&selected_team)}).await{
                                Ok(id)=>on_saved.call(id),
                                Err(e)=>{error.set(Some(e.clone()));on_error.call(e);}
                            }
                            busy.set(false);
                        });
                    },
                    div{class:"dialog-body",
                        div{class:"field",
                            label{"Nom de l'activité"}
                            input{value:"{name}",required:true,maxlength:"150",oninput:move |e|name.set(e.value())}
                        }
                        div{class:"field",
                            label{"Bibliothèque partagée"}
                            select{value:"{team_id}",onchange:move |e|team_id.set(e.value()),
                                option{value:"","Activité personnelle"}
                                if let Some(Ok(teams))=available_teams.read().clone(){
                                    for (id,label) in teams{option{value:"{id}","{label}"}}
                                }
                            }
                            small{"Une activité liée à une équipe est visible par ses membres."}
                        }
                        div{class:"field",
                            label{"Description et consignes"}
                            RichTextEditor {
                                initial_html: activity.as_ref().and_then(|a|a.description_html.clone()),
                                initial_plain: activity.as_ref().map(|a|a.description.clone()).unwrap_or_default(),
                                on_html: move |html: String| description_html.set(Some(html)),
                                on_plain: move |text: String| description.set(text),
                            }
                            small { "Mise en forme directe : gras, italique, titres et listes." }
                        }
                        div{class:"field",
                            label{"Image de couverture"}
                            if !image.read().is_empty(){
                                img{class:"editor-preview",src:"{catalog::image_src(&image.read())}",alt:"Aperçu"}
                            }
                            input{r#type:"url",placeholder:"https://exemple.fr/image.png",
                                value:if image.read().starts_with("http"){"{image}"}else{""},
                                oninput:move |e|image.set(e.value())}
                            small{"Ou téléversez une image PNG, JPG, WebP ou GIF de 4 Mo maximum."}
                            input{r#type:"file",accept:".png,.jpg,.jpeg,.webp,.gif,image/png,image/jpeg,image/webp,image/gif",
                                disabled:*uploading.read(),
                                onchange:{
                                    let token=token_for_upload.clone();
                                    move |event| {
                                        let token=token.clone();
                                        async move {
                                            for file in event.files(){
                                                uploading.set(true);
                                                let result=match file.read_bytes().await{
                                                    Ok(bytes)=>catalog::upload(&token,bytes.to_vec()).await,
                                                    Err(e)=>Err(format!("Lecture du fichier impossible : {e}")),
                                                };
                                                match result {
                                                    Ok(url)=>{image.set(url);error.set(None);}
                                                    Err(e)=>error.set(Some(e)),
                                                }
                                                uploading.set(false);
                                            }
                                        }
                                    }
                                }
                            }
                            if *uploading.read(){small{"Téléversement en cours…"}}
                        }
                        if let Some(message)=error.read().as_ref(){p{class:"form-error",role:"alert","{message}"}}
                    }
                    div{class:"dialog-footer",
                        button{class:"button quiet",r#type:"button",onclick:move |_|on_cancel.call(()),"Annuler"}
                        button{class:"button solid",r#type:"submit",disabled:*busy.read()||*uploading.read(),
                            if *busy.read(){"Enregistrement…"}else{"Enregistrer"}}
                    }
                }
            }
        }
    }
}


#[component]
fn MaterialEditor(
    token: String,
    activity_id: String,
    material: catalog::Material,
    on_saved: EventHandler<()>,
    on_error: EventHandler<String>,
) -> Element {
    let mut editing = use_signal(|| false);
    let mut name = use_signal(|| material.name.clone());
    let mut quantity = use_signal(|| material.quantity.to_string());
    let mut busy = use_signal(|| false);
    rsx! {
        if !*editing.read() {
            span { "{material.quantity} × {material.name}" }
            button { class:"remove-link", title:"Modifier le matériel",
                onclick:move |_|editing.set(true), "Modifier" }
        } else {
            form { class:"material-edit-form",
                onsubmit:{
                    let token=token.clone();
                    let activity_id=activity_id.clone();
                    let material_id=material.id.clone();
                    move |event|{
                        event.prevent_default();
                        if *busy.read(){return;}
                        let label=name.read().trim().to_owned();
                        let count=quantity.read().parse::<i32>().unwrap_or(0);
                        if label.is_empty() || !(1..=100000).contains(&count) {
                            on_error.call("Nom et quantité du matériel invalides.".into());
                            return;
                        }
                        let token=token.clone();
                        let activity_id=activity_id.clone();
                        let material_id=material_id.clone();
                        busy.set(true);
                        spawn(async move {
                            match catalog::update_material(&token,&activity_id,&material_id,&label,count).await{
                                Ok(())=>{editing.set(false);on_saved.call(());}
                                Err(err)=>on_error.call(err),
                            }
                            busy.set(false);
                        });
                    }
                },
                input { "aria-label":"Nom du matériel", value:"{name}",
                    maxlength:"150", required:true, oninput:move |e|name.set(e.value()) }
                input { "aria-label":"Quantité", r#type:"number", min:"1", value:"{quantity}",
                    oninput:move |e|quantity.set(e.value()) }
                button { class:"button outline small", r#type:"submit", disabled:*busy.read(), "OK" }
                button { class:"button quiet small", r#type:"button", onclick:move |_|editing.set(false), "Annuler" }
            }
        }
    }
}
