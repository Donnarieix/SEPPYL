use dioxus::prelude::*;
use crate::api::{account,changelog};

#[component]
pub fn ChangelogHost()->Element{
    let open=use_context::<Signal<bool>>();
    let session=use_context::<Signal<Option<String>>>();
    let token=session.read().clone();
    let entries_token=token.clone();
    let mut entries=use_resource(move ||{
        let token=entries_token.clone();
        async move{
            match token{Some(token)=>changelog::list(&token).await,
                None=>Ok(Vec::new())}
        }
    });
    let status_token=token.clone();
    let mut unread_status=use_resource(move ||{
        let token=status_token.clone();
        async move{
            match token{Some(token)=>changelog::status(&token).await,
                None=>Ok(changelog::Status{version:"v0.1".into(),unread:false})}
        }
    });
    let role_token=token.clone();
    let user=use_resource(move ||{
        let token=role_token.clone();
        async move{match token{Some(token)=>account::get(&token).await,
            None=>Err("Connexion requise.".into())}}
    });
    let mut prompted=use_signal(||None::<String>);
    let mut selected=use_signal(||"v0.1".to_string());
    let mut editing=use_signal(||false);
    let mut title=use_signal(String::new);
    let mut body=use_signal(String::new);
    let mut creating=use_signal(||false);
    let mut new_version=use_signal(String::new);
    let mut busy=use_signal(||false);
    let mut notice=use_signal(||None::<String>);
    let prompt_token=token.clone();
    use_effect(move ||{
        if let (Some(token),Some(Ok(status)))=(prompt_token.as_ref(),unread_status.read().clone()){
            if status.unread && prompted.read().as_deref()!=Some(token.as_str()){
                prompted.set(Some(token.clone()));
                selected.set(status.version.clone());
                open.set(true);
            }
        }
    });

    let history=entries.read().as_ref().and_then(|v|v.as_ref().ok()).cloned()
        .unwrap_or_default();
    let active=history.iter().find(|item|item.version==*selected.read())
        .or_else(||history.first()).cloned();
    let is_maintainer=matches!(user.read().as_ref(),Some(Ok(value)) if value.role=="maintainer");
    let unread=unread_status.read().as_ref()
        .is_some_and(|value|value.as_ref().is_ok_and(|v|v.unread));
    rsx!{
        if *open.read() && token.is_some(){
            div{class:"changelog-backdrop",
                div{class:"changelog-modal",role:"dialog","aria-modal":"true",
                    "aria-label":"Historique des versions",
                    div{class:"changelog-header",
                        div{
                            span{class:"changelog-eyebrow","SEPPYL · HISTORIQUE"}
                            h2{"Nouveautés et versions"}
                        }
                        button{class:"icon-button changelog-close",r#type:"button",
                            "aria-label":"Fermer l'historique",
                            onclick:{
                                let token=token.clone();
                                move |_|{
                                    open.set(false);
                                    editing.set(false);creating.set(false);
                                    if unread{
                                        if let Some(token)=token.clone(){
                                            spawn(async move{
                                                if changelog::read(&token,"v0.1").await.is_ok(){
                                                    unread_status.restart();
                                                }
                                            });
                                        }
                                    }
                                }
                            },
                            "×"
                        }
                    }
                    div{class:"changelog-layout",
                        aside{class:"changelog-versions","aria-label":"Versions disponibles",
                            div{class:"changelog-versions-label","Versions"}
                            for item in &history{
                                button{
                                    key:"{item.version}",
                                    class:if active.as_ref().is_some_and(|v|v.version==item.version){"changelog-version active"}else{"changelog-version"},
                                    r#type:"button",
                                    onclick:{
                                        let item=item.clone();
                                        move |_|{
                                            selected.set(item.version.clone());
                                            title.set(item.title.clone());body.set(item.body.clone());
                                            editing.set(false);creating.set(false);notice.set(None);
                                        }
                                    },
                                    strong{"{item.version}"}
                                    span{"{item.title}"}
                                }
                            }
                            if is_maintainer{
                                button{class:"changelog-new",r#type:"button",
                                    onclick:move |_|{
                                        new_version.set(String::new());
                                        title.set(String::new());body.set(String::new());
                                        editing.set(true);creating.set(true);notice.set(None);
                                    },"+ Nouvelle version"}
                            }
                        }
                        section{class:"changelog-detail",
                            if let Some(Err(error))=entries.read().clone(){
                                p{class:"changelog-error","{error}"}
                            }
                            if let Some(message)=notice.read().as_ref(){
                                p{class:"changelog-status",role:"status","{message}"}
                            }
                            if *editing.read() && is_maintainer{
                                form{class:"changelog-editor",
                                    onsubmit:{
                                        let token=token.clone();
                                        move |event|{
                                            event.prevent_default();
                                            if *busy.read(){return;}
                                            let version=if *creating.read(){new_version.read().clone()}
                                                else{selected.read().clone()};
                                            let label=title.read().clone();
                                            let description=body.read().clone();
                                            if version.is_empty()||label.trim().is_empty()||description.trim().is_empty(){
                                                notice.set(Some("Renseignez la version, le titre et le contenu.".into()));
                                                return;
                                            }
                                            let Some(token)=token.clone() else{
                                                notice.set(Some("Connexion requise.".into()));return;
                                            };
                                            busy.set(true);notice.set(None);
                                            spawn(async move{
                                                match changelog::save(&token,&version,&label,&description).await{
                                                    Ok(_)=>{
                                                        selected.set(version);
                                                        entries.restart();
                                                        editing.set(false);creating.set(false);
                                                        notice.set(Some("Historique mis à jour.".into()));
                                                    }
                                                    Err(error)=>notice.set(Some(error)),
                                                }
                                                busy.set(false);
                                            });
                                        }
                                    },
                                    if *creating.read(){
                                        div{class:"field",label{"Version"},
                                            input{value:"{new_version}",required:true,maxlength:"32",
                                                placeholder:"Ex. v0.2",oninput:move |e|new_version.set(e.value())}}
                                    }else{
                                        span{class:"changelog-tag","{selected}"}
                                    }
                                    div{class:"field",label{"Titre"},
                                        input{value:"{title}",required:true,maxlength:"150",
                                            oninput:move |e|title.set(e.value())}}
                                    div{class:"field",label{"Notes de version"},
                                        textarea{rows:"14",value:"{body}",required:true,maxlength:"30000",
                                            placeholder:"Décrivez les changements de cette version…",
                                            oninput:move |e|body.set(e.value())}}
                                    div{class:"changelog-editor-actions",
                                        button{class:"button outline",r#type:"button",
                                            onclick:move |_|{editing.set(false);creating.set(false);},"Annuler"}
                                        button{class:"button solid",r#type:"submit",disabled:*busy.read(),
                                            if *busy.read(){"Enregistrement…"}else{"Enregistrer"}}
                                    }
                                }
                            }else if let Some(item)=active{
                                div{class:"changelog-detail-heading",
                                    span{class:"changelog-tag","{item.version}"}
                                    h3{"{item.title}"}
                                    if is_maintainer{
                                        button{class:"button outline small",r#type:"button",
                                            onclick:{
                                                let item=item.clone();
                                                move |_|{
                                                    title.set(item.title.clone());
                                                    body.set(item.body.clone());
                                                    editing.set(true);creating.set(false);
                                                    notice.set(None);
                                                }
                                            },"Modifier"}
                                    }
                                }
                                div{class:"changelog-body","{item.body}"}
                            }else{
                                div{class:"panel-empty","Aucune version disponible."}
                            }
                        }
                    }
                }
            }
        }
    }
}
