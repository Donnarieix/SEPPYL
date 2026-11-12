use dioxus::prelude::*;
use crate::api::{tickets,account};
use std::collections::HashMap;
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
    let u=token.clone();
    let mut unread=use_resource(move ||{let u=u.clone();async move{tickets::unread(&u).await}});
    let owner_token=token.clone();
    let account=use_resource(move ||{let t=owner_token.clone();async move{account::get(&t).await}});
    let is_staff=matches!(account.read().as_ref(),Some(Ok(a)) if a.role=="administrator"||a.role=="maintainer");
    let pending=unread.read().as_ref().and_then(|r|r.as_ref().ok())
        .map(|r|r.tickets.iter().map(|item|(item.ticket_id.clone(),item.count))
            .collect::<HashMap<_,_>>()).unwrap_or_default();
    let mut list_poll=list;
    let mut unread_poll=unread;
    use_future(move ||async move{
        loop{
            gloo_timers::future::TimeoutFuture::new(20_000).await;
            list_poll.restart();
            unread_poll.restart();
        }
    });
    let mut subject=use_signal(String::new);
    let mut message=use_signal(String::new);
    let mut creating=use_signal(||false);
    let mut busy=use_signal(||false);
    let mut notice=use_signal(||None::<String>);
    rsx!{
        div{class:"app",Header{},
            main{class:"page workspace-page tickets-page",
                div{class:"page-heading",
                    div{h1{if is_staff{"Tous les tickets"}else{"Mes tickets"}}p{"Suivez vos demandes et échangez avec l'équipe de support."}}
                    button{class:"button solid",onclick:move |_|creating.set(true),"+ Nouveau ticket"}
                }
                if let Some(n)=notice.read().as_ref(){p{class:"inline-notice",role:"status","{n}"}}
                section{class:"panel",
                    match list.read().clone(){
                        Some(Ok(items))=>rsx!{
                            if items.is_empty(){p{class:"muted-small","Aucun ticket pour le moment."}}
                            for ticket in items{
                                button{class:"ticket-row",key:"{ticket.id}",r#type:"button",
                                    onclick:{let id=ticket.id.clone();move |_|{navigator.push(Route::TicketDetail{id:id.clone()});}},
                                    div{strong{"{ticket.subject}"}
                                        span{class:"muted-small"," · {status_label(&ticket.status)}"}
                                    }
                                    div{class:"ticket-row-meta",
                                        if ticket.maintainer_required {
                                            span{class:"ticket-escalation","Maintainer"}
                                        }
                                        if let Some(count)=pending.get(&ticket.id){
                                            span{class:"ticket-row-unread",title:"Messages non lus","{count} non lu(s)"}
                                        }
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
                                    div{class:"field",label{"Premier message"}
                                        textarea{value:"{message}",maxlength:"10000",required:true,rows:"6",
                                            oninput:move |e|message.set(e.value())}}
                                }
                                div{class:"dialog-footer",
                                    button{class:"button quiet",r#type:"button",
                                        onclick:move |_|creating.set(false),"Annuler"}
                                    button{class:"button solid",r#type:"submit",disabled:*busy.read(),"Créer le ticket"}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}


fn format_time(stamp:i64)->String{
    chrono::DateTime::<chrono::Utc>::from_timestamp(stamp,0)
        .map(|time|time.format("%d/%m/%Y · %H:%M UTC").to_string())
        .unwrap_or_default()
}
#[component]
pub fn TicketDetail(id:String)->Element{
    let session=use_context::<Signal<Option<String>>>();
    let token=session.read().clone().unwrap_or_default();
    let t=token.clone();let requested=id.clone();
    let mut detail=use_resource(move ||{let t=t.clone();let id=requested.clone();
        async move{tickets::get(&t,&id).await}});
    let u=token.clone();
    let user=use_resource(move ||{let t=u.clone();async move{account::get(&t).await}});
    let is_staff=matches!(user.read().as_ref(),Some(Ok(p)) if p.role=="administrator"||p.role=="maintainer");
    let current_user=user.read().as_ref().and_then(|v|v.as_ref().ok())
        .map(|u|u.id.clone()).unwrap_or_default();
    let mut reply=use_signal(String::new);
    let mut emoji_open=use_signal(||false);
    let mut peer_typing=use_signal(||None::<String>);
    let mut status=use_signal(String::new);
    let mut escalation=use_signal(||None::<bool>);
    let mut busy=use_signal(||false);
    let mut error=use_signal(||None::<String>);
    let mut fallback_poll=detail;
    // SSE normally refreshes immediately; polling covers proxies that buffer streams.
    use_future(move ||async move{
        loop{
            gloo_timers::future::TimeoutFuture::new(30_000).await;
            fallback_poll.restart();
        }
    });
    let base=crate::api::base_url();
    rsx!{
        div{class:"app",Header{},
            main{class:"page workspace-page ticket-detail-page",
                Link{to:Route::Tickets{},class:"back-link","← Tickets"}
                if let Some(e)=error.read().as_ref(){p{class:"form-error",role:"alert","{e}"}}
                match detail.read().clone(){
                    Some(Ok(current))=>rsx!{
                        div{class:"ticket-chat","data-ticket-id":"{id}","data-ticket-base":"{base}",
                            textarea{
                                class:"ticket-live-input","data-ticket-live-event":"true",
                                "aria-hidden":"true",tabindex:"-1",
                                oninput:{
                                    let my_id=current_user.clone();
                                    move |event|{
                                        if let Ok(value)=serde_json::from_str::<serde_json::Value>(&event.value()){
                                            let kind=value["kind"].as_str().unwrap_or_default();
                                            let actor=value["actor"].as_str().unwrap_or_default();
                                            match kind{
                                                "message"=>detail.restart(),
                                                "typing" if actor!=my_id=>{
                                                    peer_typing.set(Some(value["username"].as_str()
                                                        .filter(|v|!v.is_empty()).unwrap_or("Support").to_string()));
                                                }
                                                "typing_end" if actor!=my_id=>peer_typing.set(None),
                                                _=>{}
                                            }
                                        }
                                    }
                                }
                            }
                            div{class:"ticket-chat-heading",
                                div{class:"ticket-chat-topic",
                                    span{class:"ticket-chat-eyebrow","Conversation d'assistance"}
                                    h1{"{current.ticket.subject}"}
                                    div{class:"ticket-chat-tags",
                                        span{class:"chat-state-pill","{status_label(&current.ticket.status)}"}
                                        if current.ticket.maintainer_required{
                                            span{class:"ticket-escalation","Maintainer"}
                                        }
                                    }
                                }
                                div{class:"ticket-chat-presence",
                                    span{class:"ticket-presence-dot"}
                                    span{"data-ticket-live-status":"true","Connexion…"}
                                }
                            }
                            div{class:"ticket-chat-feed",role:"log","aria-label":"Messages du ticket","aria-live":"polite",
                                for msg in current.messages{
                                    article{key:"{msg.id}",
                                        class:if msg.author_id==current_user{"ticket-chat-message own"}else{"ticket-chat-message"},
                                        div{class:"ticket-chat-avatar",
                                            if msg.author_id==current_user{"V"}else if is_staff{"D"}else{"S"}
                                        }
                                        div{class:"ticket-chat-content",
                                            div{class:"ticket-chat-meta",
                                                strong{
                                                    if msg.author_id==current_user{"Vous"}
                                                    else if is_staff{"Demandeur"}
                                                    else{"Équipe support"}
                                                }
                                                time{"{format_time(msg.created_at)}"}
                                            }
                                            div{class:"ticket-chat-bubble",
                                                if !msg.message.is_empty(){
                                                    div{class:"ticket-chat-markdown","data-ticket-markdown":"true",
                                                        "data-ticket-raw":"{msg.message}","{msg.message}"}
                                                }
                                                if let Some(filename)=msg.attachment_name.as_ref(){
                                                    div{class:"ticket-chat-attachment",
                                                        img{"data-ticket-image":format!("/tickets/{}/images/{}",id,filename),
                                                            alt:"Image jointe au message",loading:"lazy"}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            div{class:"ticket-chat-composer",
                                if let Some(name)=peer_typing.read().as_ref(){
                                    div{class:"ticket-typing-indicator",role:"status",
                                        span{class:"typing-dots",i{} i{} i{}}
                                        "{name} est en train d'écrire…"
                                    }
                                }
                                if current.ticket.status!="closed" || is_staff{
                                    form{class:"ticket-message-form",
                                        onsubmit:{
                                            let token=token.clone();let id=id.clone();
                                            move |event|{
                                                event.prevent_default();
                                                let value=reply.read().trim().to_owned();
                                                if value.is_empty()||*busy.read(){return;}
                                                busy.set(true);
                                                let t=token.clone();let tid=id.clone();
                                                spawn(async move{
                                                    match tickets::reply(&t,&tid,&value).await {
                                                        Ok(())=>{reply.set(String::new());detail.restart();error.set(None);},
                                                        Err(e)=>error.set(Some(e)),
                                                    }
                                                    busy.set(false);
                                                });
                                            }
                                        },
                                        div{class:"ticket-composer-surface",
                                            textarea{rows:"3",maxlength:"10000","aria-label":"Écrire un message",
                                                "data-ticket-composer":"true",
                                                placeholder:"Écrire un message…",
                                                value:"{reply}",
                                                oninput:move |e|reply.set(e.value())}
                                            div{class:"ticket-composer-toolbar",
                                                div{class:"ticket-compose-tools",
                                                    label{class:"ticket-chat-tool",title:"Joindre une image",
                                                        "aria-label":"Joindre une image",
                                                        span{"＋"}
                                                        input{r#type:"file","data-ticket-attach-input":"true",
                                                            accept:"image/png,image/jpeg,image/webp,image/gif"}
                                                    }
                                                    button{r#type:"button",class:"ticket-chat-tool",title:"Émojis",
                                                        "aria-label":"Insérer un émoji",
                                                        onclick:move |_|emoji_open.set(!*emoji_open.read()),"☺"}
                                                    button{r#type:"button",class:"ticket-chat-tool",title:"Gras Markdown",
                                                        onclick:move |_|reply.set(format!("{}**texte**",reply.read().as_str())),"B"}
                                                    button{r#type:"button",class:"ticket-chat-tool",title:"Italique Markdown",
                                                        onclick:move |_|reply.set(format!("{}*texte*",reply.read().as_str())),"I"}
                                                }
                                                button{class:"button solid ticket-send-button",r#type:"submit",
                                                    disabled:*busy.read()||reply.read().trim().is_empty(),
                                                    if *busy.read(){"Envoi…"}else{"Envoyer"}
                                                }
                                            }
                                        }
                                        if *emoji_open.read(){
                                            div{class:"ticket-emoji-picker",role:"group","aria-label":"Choisir un émoji",
                                                for symbol in ["😀","😂","😊","❤️","👍","👏","🎉","🙏","👀","✅","🔥","⚽"]{
                                                    button{r#type:"button",class:"ticket-emoji",key:"{symbol}",
                                                        onclick:move |_|{
                                                            reply.set(format!("{}{}",reply.read().as_str(),symbol));
                                                            emoji_open.set(false);
                                                        },
                                                        "{symbol}"
                                                    }
                                                }
                                            }
                                        }
                                        p{class:"ticket-composer-hint",
                                            "Markdown et émojis acceptés · Images PNG, JPEG, WebP ou GIF (4 Mo maximum)"}
                                        p{class:"ticket-upload-feedback",role:"status","data-chat-upload-feedback":"true"}
                                    }
                                }else{
                                    div{class:"ticket-conversation-closed","Ce ticket est fermé."}
                                }
                            }
                        }
                        if is_staff{
                            section{class:"panel ticket-moderation",
                                div{class:"panel-head",
                                    div{h2{"Gestion du ticket"}p{"Suivi visible par le demandeur."}}
                                }
                                div{class:"ticket-moderation-controls",
                                    div{class:"field",
                                        label{"Statut"}
                                        select{value:"{status}",onchange:move |e|status.set(e.value()),
                                            option{value:"","Conserver : {status_label(&current.ticket.status)}"}
                                            option{value:"open","Ouvert"}
                                            option{value:"in_progress","En cours"}
                                            option{value:"resolved","Résolu"}
                                            option{value:"closed","Fermé"}
                                        }
                                    }
                                    label{class:"seppyl-switch-row",
                                        span{strong{"Intervention Maintainer"}
                                            small{"Signaler que ce ticket nécessite un Maintainer."}}
                                        input{r#type:"checkbox",
                                            checked:escalation.read().unwrap_or(current.ticket.maintainer_required),
                                            onchange:move |e|escalation.set(Some(e.checked()))}
                                    }
                                    button{class:"button outline",r#type:"button",disabled:*busy.read(),
                                        onclick:{
                                            let token=token.clone();let id=id.clone();
                                            let current_status=current.ticket.status.clone();
                                            let current_flag=current.ticket.maintainer_required;
                                            move |_|{
                                                if *busy.read(){return;}
                                                let t=token.clone();let id=id.clone();
                                                let next=status.read().clone();
                                                let flag=escalation.read().unwrap_or(current_flag);
                                                let actual=if next.is_empty(){current_status.clone()}else{next};
                                                busy.set(true);
                                                spawn(async move{
                                                    match tickets::status(&t,&id,&actual,flag).await{
                                                        Ok(())=>{detail.restart();error.set(None);},
                                                        Err(e)=>error.set(Some(e))
                                                    }
                                                    busy.set(false);
                                                });
                                            }
                                        },"Enregistrer"
                                    }
                                }
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
