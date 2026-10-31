use dioxus::prelude::*;
use crate::api::{tickets,account};
use super::{Route,layout::Header};

fn status_label(value:&str)->&str{
    match value{"open"=>"Ouvert","in_progress"=>"En cours","resolved"=>"Résolu",
        "closed"=>"Fermé",_=>"Inconnu"}
}

#[component]
pub fn Tickets()->Element{
    let session=use_context::<Signal<Option<String>>>();
    let token=session.read().clone().unwrap_or_default();
    let navigator=use_navigator();
    let t=token.clone();
    let mut list=use_resource(move ||{let t=t.clone();async move{tickets::list(&t).await}});
    let mut subject=use_signal(String::new);
    let mut message=use_signal(String::new);
    let mut creating=use_signal(||false);
    let mut busy=use_signal(||false);
    let mut notice=use_signal(||None::<String>);
    rsx!{
        div{class:"app",Header{},
            main{class:"page workspace-page",
                div{class:"page-heading",
                    div{h1{"Mes tickets"}p{"Suivez vos demandes et échangez avec l'équipe de support."}}
                    button{class:"button solid",onclick:move |_|creating.set(true),"+ Nouveau ticket"}
                }
                if let Some(n)=notice.read().as_ref(){p{class:"inline-notice",role:"status","{n}"}}
                section{class:"panel",
                    match list.read().clone(){
                        Some(Ok(items))=>rsx!{
                            if items.is_empty(){p{class:"muted-small","Aucun ticket pour le moment."}}
                            for ticket in items{
                                button{class:"ticket-row",key:"{ticket.id}",r#type:"button",
                                    onclick:{let id=ticket.id.clone();move |_|navigator.push(Route::TicketDetail{id:id.clone()})},
                                    div{strong{"{ticket.subject}"}
                                        span{class:"muted-small"," · {status_label(&ticket.status)}"}
                                    }
                                    if ticket.maintainer_required {
                                        span{class:"ticket-escalation","Maintainer"}
                                    }
                                }
                            }
                        },
                        Some(Err(e))=>rsx!{p{class:"form-error","{e}"}},
                        None=>rsx!{p{"Chargement…"}}
                    }
                }
                if *creating.read(){
                    div{class:"dialog-backdrop",
                        div{class:"dialog-panel",role:"dialog","aria-modal":"true",
                            div{class:"dialog-header",h2{"Créer un ticket"}
                                button{class:"icon-button",onclick:move |_|creating.set(false),"×"}}
                            form{onsubmit:{
                                let token=token.clone();
                                move |event|{
                                    event.prevent_default();
                                    if *busy.read(){return;}
                                    busy.set(true);
                                    let token=token.clone();
                                    let title=subject.read().clone();
                                    let text=message.read().clone();
                                    spawn(async move{
                                        match tickets::create(&token,&title,&text).await{
                                            Ok(created)=>{
                                                creating.set(false);list.restart();
                                                navigator.push(Route::TicketDetail{id:created.id});
                                            },
                                            Err(e)=>notice.set(Some(e))
                                        }
                                        busy.set(false);
                                    });
                                }
                            },
                                div{class:"dialog-body",
                                    div{class:"field",label{"Objet"}
                                        input{value:"{subject}",maxlength:"160",required:true,
                                            oninput:move |e|subject.set(e.value())}}
                                    div{class:"field",label{"Description du problème"}
                                        textarea{value:"{message}",maxlength:"10000",required:true,rows:"6",
                                            oninput:move |e|message.set(e.value())}}
                                }
                                div{class:"dialog-footer",
                                    button{class:"button quiet",r#type:"button",
                                        onclick:move |_|creating.set(false),"Annuler"}
                                    button{class:"button solid",r#type:"submit",disabled:*busy.read(),"Envoyer"}
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
pub fn TicketDetail(id:String)->Element{
    let session=use_context::<Signal<Option<String>>>();
    let token=session.read().clone().unwrap_or_default();
    let t=token.clone();let ref_id=id.clone();
    let mut detail=use_resource(move ||{let t=t.clone();let id=ref_id.clone();
        async move{tickets::get(&t,&id).await}});
    let user_token=token.clone();
    let user=use_resource(move ||{let t=user_token.clone();async move{account::get(&t).await}});
    let is_staff=matches!(user.read().as_ref(),Some(Ok(p)) if p.role=="administrator"||p.role=="maintainer");
    let mut reply=use_signal(String::new);
    let mut status=use_signal(String::new);
    let mut escalation=use_signal(||false);
    let mut busy=use_signal(||false);
    let mut error=use_signal(||None::<String>);
    // A 15-second poll keeps threaded ticket discussions current without
    // a permanent WebSocket, and stops when this component unmounts.
    let mut poll=detail;
    use_future(move || async move{
        loop{
            gloo_timers::future::TimeoutFuture::new(15_000).await;
            poll.restart();
        }
    });
    rsx!{
        div{class:"app",Header{},
            main{class:"page workspace-page ticket-detail-page",
                Link{to:Route::Tickets{},class:"back-link","← Tous les tickets"}
                if let Some(e)=error.read().as_ref(){p{class:"form-error",role:"alert","{e}"}}
                match detail.read().clone(){
                    Some(Ok(current))=>rsx!{
                        div{class:"page-heading",
                            div{h1{"{current.ticket.subject}"}
                                p{"Statut : {status_label(&current.ticket.status)}"}
                            }
                            if current.ticket.maintainer_required {
                                span{class:"ticket-escalation","Maintainer"}
                            }
                        }
                        section{class:"panel",aria_label:"Conversation du ticket",
                            div{class:"ticket-conversation",
                                for msg in current.messages{
                                    article{class:"ticket-message",key:"{msg.id}",
                                        strong{if msg.author_id==current.ticket.author_id{"Demandeur"}else{"Équipe support"}}
                                        p{"{msg.message}"}
                                    }
                                }
                            }
                            form{onsubmit:{
                                let t=token.clone();let id=id.clone();
                                move |event|{
                                    event.prevent_default();
                                    if *busy.read(){return;}
                                    let text=reply.read().clone();
                                    busy.set(true);
                                    let t=t.clone();let id=id.clone();
                                    spawn(async move{
                                        match tickets::reply(&t,&id,&text).await {
                                            Ok(())=>{reply.set(String::new());detail.restart();},
                                            Err(e)=>error.set(Some(e))
                                        }
                                        busy.set(false);
                                    });
                                }
                            },
                                div{class:"field",label{"Répondre au ticket"}
                                    textarea{rows:"4",maxlength:"10000",value:"{reply}",required:true,
                                        oninput:move |e|reply.set(e.value())}}
                                button{class:"button solid",r#type:"submit",disabled:*busy.read(),"Envoyer la réponse"}
                            }
                        }
                        if is_staff {
                            section{class:"panel",
                                div{class:"panel-head",h2{"Traitement du ticket"}}
                                div{class:"field",label{"Statut"}
                                    select{value:"{status}",onchange:move |e|status.set(e.value()),
                                        option{value:"","Conserver le statut actuel"}
                                        option{value:"open","Ouvert"}
                                        option{value:"in_progress","En cours"}
                                        option{value:"resolved","Résolu"}
                                        option{value:"closed","Fermé"}
                                    }
                                }
                                label{class:"field ticket-checkbox",
                                    input{r#type:"checkbox",checked:*escalation.read()||current.ticket.maintainer_required,
                                        onchange:move |e|escalation.set(e.checked())}
                                    "À traiter par un Maintainer"
                                }
                                button{class:"button outline",disabled:*busy.read(),
                                    onclick:{
                                        let t=token.clone();let id=id.clone();
                                        let current_status=current.ticket.status.clone();
                                        let current_flag=current.ticket.maintainer_required;
                                        move |_|{
                                            let t=t.clone();let id=id.clone();
                                            let next=status.read().clone();
                                            let flag=*escalation.read()||current_flag;
                                            busy.set(true);
                                            let actual=if next.is_empty(){current_status.clone()}else{next};
                                            spawn(async move{
                                                match tickets::status(&t,&id,&actual,flag).await{
                                                    Ok(())=>detail.restart(),
                                                    Err(e)=>error.set(Some(e))
                                                }
                                                busy.set(false);
                                            });
                                        }
                                    },"Enregistrer"}
                            }
                        }
                    },
                    Some(Err(e))=>rsx!{p{class:"form-error","{e}"}},
                    None=>rsx!{p{"Chargement du ticket…"}}
                }
            }
        }
    }
}
