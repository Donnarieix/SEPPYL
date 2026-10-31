use dioxus::prelude::*;
use crate::api::{announcements,account};
use super::layout::Header;

/// Rendered above the Router to reach both public and protected pages.
/// Normal announcements stay in the dedicated page; only urgent ones display
/// as persistent banners across navigation.
#[component]
pub fn UrgentBanner()->Element{
    let active=use_resource(||async{announcements::active().await});
    rsx!{
        if let Some(Ok(items))=active.read().clone(){
            for item in items.into_iter().filter(|a|a.urgent){
                div{class:"global-urgent-banner",role:"status",key:"{item.id}",
                    strong{"{item.title}"} span{"{item.body}"}
                }
            }
        }
    }
}

#[component]
pub fn AnnouncementAdmin()->Element{
    let session=use_context::<Signal<Option<String>>>();
    let token=session.read().clone().unwrap_or_default();
    let user_token=token.clone();
    let account=use_resource(move ||{let t=user_token.clone();async move{account::get(&t).await}});
    let t=token.clone();
    let mut list=use_resource(move ||{let t=t.clone();async move{announcements::all(&t).await}});
    let admin=matches!(account.read().as_ref(),Some(Ok(a)) if a.role=="maintainer"||a.role=="administrator");
    let mut title=use_signal(String::new);
    let mut body=use_signal(String::new);
    let mut urgent=use_signal(||false);
    let mut published=use_signal(||true);
    let mut starts=use_signal(||"0".to_string());
    let mut ends=use_signal(String::new);
    let mut notice=use_signal(||None::<String>);
    let mut busy=use_signal(||false);
    rsx!{
        div{class:"app",Header{},
            main{class:"page workspace-page",
                div{class:"page-heading",
                    div{h1{"Annonces"}p{"Communications distinctes de l'historique des versions."}}
                }
                if !admin{
                    p{class:"form-error","La gestion des annonces est réservée à l'administration."}
                }else{
                    if let Some(e)=notice.read().as_ref(){p{class:"inline-notice",role:"status","{e}"}}
                    section{class:"panel",
                        div{class:"panel-head",h2{"Nouvelle annonce"}
                            p{"Le bandeau global apparaît uniquement pour les annonces urgentes publiées."}}
                        form{onsubmit:{
                            let t=token.clone();
                            move |event|{
                                event.prevent_default();
                                if *busy.read(){return;}
                                busy.set(true);
                                let t=t.clone();
                                let label=title.read().clone();let content=body.read().clone();
                                let is_urgent=*urgent.read();let is_published=*published.read();
                                let begin=starts.read().parse::<i64>().unwrap_or(0);
                                let until=ends.read().parse::<i64>().ok();
                                spawn(async move{
                                    let result=announcements::create(&t,announcements::Payload{
                                        title:&label,body:&content,urgent:is_urgent,
                                        published:is_published,starts_at:begin,ends_at:until
                                    }).await;
                                    match result{
                                        Ok(())=>{title.set(String::new());body.set(String::new());list.restart();
                                            notice.set(Some("Annonce enregistrée.".into()));},
                                        Err(e)=>notice.set(Some(e))
                                    }
                                    busy.set(false);
                                });
                            }
                        },
                            div{class:"field",label{"Titre"}
                                input{required:true,maxlength:"150",value:"{title}",oninput:move |e|title.set(e.value())}}
                            div{class:"field",label{"Message"}
                                textarea{required:true,maxlength:"10000",rows:"4",value:"{body}",
                                    oninput:move |e|body.set(e.value())}}
                            label{class:"ticket-checkbox",
                                input{r#type:"checkbox",checked:*urgent.read(),onchange:move |e|urgent.set(e.checked())}
                                "Urgent — afficher sur toutes les pages"
                            }
                            label{class:"ticket-checkbox",
                                input{r#type:"checkbox",checked:*published.read(),onchange:move |e|published.set(e.checked())}
                                "Publier l'annonce"
                            }
                            div{class:"form-grid",
                                div{class:"field",label{"Début (timestamp Unix, 0 = immédiat)"}
                                    input{r#type:"number",value:"{starts}",oninput:move |e|starts.set(e.value())}}
                                div{class:"field",label{"Fin (timestamp Unix, facultatif)"}
                                    input{r#type:"number",value:"{ends}",oninput:move |e|ends.set(e.value())}}
                            }
                            button{class:"button solid",r#type:"submit",disabled:*busy.read(),"Publier"}
                        }
                    }
                    section{class:"panel",
                        div{class:"panel-head",h2{"Historique des annonces"}}
                        match list.read().clone(){
                            Some(Ok(items))=>rsx!{
                                for a in items{
                                    div{class:"ticket-row",key:"{a.id}",
                                        div{strong{"{a.title}"}
                                            p{"{a.body}"}
                                            span{class:"muted-small",if a.published{"Publiée"}else{"Brouillon"}}
                                        }
                                        button{class:"button quiet small",r#type:"button",
                                            onclick:{
                                                let t=token.clone();let id=a.id.clone();
                                                move |_|{
                                                    let t=t.clone();let id=id.clone();
                                                    spawn(async move{
                                                        match announcements::remove(&t,&id).await{
                                                            Ok(())=>list.restart(),Err(e)=>notice.set(Some(e))
                                                        }
                                                    });
                                                }
                                            },"Supprimer"}
                                    }
                                }
                            },
                            Some(Err(e))=>rsx!{p{class:"form-error","{e}"}},
                            None=>rsx!{p{"Chargement…"}}
                        }
                    }
                }
            }
        }
    }
}
