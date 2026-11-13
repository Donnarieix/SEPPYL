use dioxus::prelude::*;
use crate::api::{tickets,account};
use std::collections::HashMap;

/// Group consecutive messages from the same account for at most ten minutes
/// from the group's first message. Anchoring prevents infinite chaining.
#[derive(Clone)]
struct MessageGroup{author_id:String,started_at:i64,messages:Vec<tickets::Message>}
fn group_messages(messages:Vec<tickets::Message>)->Vec<MessageGroup>{
    let mut groups:Vec<MessageGroup>=Vec::new();
    for message in messages{
        let grouped=groups.last().is_some_and(|group|
            group.author_id==message.author_id && normalize_timestamp(message.created_at)>=group.started_at
            && normalize_timestamp(message.created_at)-group.started_at<600);
        if grouped{
            if let Some(group)=groups.last_mut(){group.messages.push(message);}
        }else{
            groups.push(MessageGroup{author_id:message.author_id.clone(),
                started_at:normalize_timestamp(message.created_at),messages:vec![message]});
        }
    }
    groups
}

use super::{Route,layout::Header};

fn normalize_timestamp(stamp:i64)->i64{
    if stamp>100_000_000_000_000{stamp/1_000_000}
    else if stamp>100_000_000_000{stamp/1000}else{stamp}
}
fn status_label(value:&str)->&str{
    match value{"open"=>"Ouvert","in_progress"=>"En cours","resolved"=>"Résolu",
        "closed"=>"Fermé",_=>"Inconnu"}
}

#[component]
fn TicketSidebar(selected:Option<String>)->Element{
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
        aside{class:"ticket-sidebar","aria-label":"Conversations de support",
            div{class:"ticket-sidebar-head",
                div{class:"ticket-sidebar-headings",
                    h2{if is_staff{"Tous les tickets"}else{"Mes tickets"}}
                    p{"Conversations d'assistance"}
                }
                button{r#type:"button",class:"ticket-sidebar-create",
                    title:"Nouveau ticket","aria-label":"Nouveau ticket",
                    onclick:move |_|creating.set(true),
                    svg{width:"20",height:"20",view_box:"0 0 24 24",fill:"none",
                        stroke:"currentColor",stroke_width:"2",
                        stroke_linecap:"round",stroke_linejoin:"round",
                        path{d:"M12 5v14M5 12h14"}
                    }
                }
            }
            if let Some(n)=notice.read().as_ref(){p{class:"inline-notice",role:"status","{n}"}}
            nav{class:"ticket-sidebar-list","aria-label":"Liste des tickets",
                match list.read().clone(){
                    Some(Ok(items))=>rsx!{
                        if items.is_empty(){
                            p{class:"ticket-sidebar-empty","Aucun ticket pour le moment."}
                        }
                        for ticket in items {
                            button{
                                class:if selected.as_ref().is_some_and(|id|id==&ticket.id){
                                    "ticket-row ticket-sidebar-item selected"
                                }else{"ticket-row ticket-sidebar-item"},
                                key:"{ticket.id}",r#type:"button",
                                "aria-current":if selected.as_ref().is_some_and(|id|id==&ticket.id){"page"}else{"false"},
                                onclick:{
                                    let id=ticket.id.clone();
                                    move |_|{navigator.push(Route::TicketDetail{id:id.clone()});}
                                },
                                div{class:"ticket-sidebar-item-main",
                                    strong{"{ticket.subject}"}
                                    div{class:"ticket-sidebar-item-meta",
                                        span{class:"chat-state-pill","{status_label(&ticket.status)}"}
                                        if ticket.maintainer_required {
                                            span{class:"ticket-escalation","Maintainer"}
                                        }
                                        if let Some(count)=pending.get(&ticket.id){
                                            span{class:"ticket-row-unread",title:"Messages non lus",
                                                "{count} non lu(s)"}
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e))=>rsx!{p{class:"form-error","{e}"}},
                    None=>rsx!{p{class:"ticket-sidebar-empty","Chargement des tickets…"}}
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

#[component]
pub fn Tickets()->Element{
    rsx!{
        div{class:"app ticket-app",Header{},
            main{class:"ticket-workspace tickets-page",
                TicketSidebar{selected:None::<String>}
                section{class:"ticket-conversation-area ticket-conversation-empty",
                    div{class:"ticket-empty-state",
                        svg{width:"48",height:"48",view_box:"0 0 24 24",
                            fill:"none",stroke:"currentColor",stroke_width:"1.2",
                            stroke_linecap:"round",stroke_linejoin:"round",
                            path{d:"M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5Z"}
                        }
                        h1{"Vos conversations"}
                        p{"Sélectionnez un ticket à gauche pour afficher les échanges."}
                    }
                }
            }
        }
    }
}


fn format_time(stamp:i64)->String{
    chrono::DateTime::<chrono::Utc>::from_timestamp(normalize_timestamp(stamp),0)
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
    let mut reply_target=use_signal(||None::<tickets::Message>);
    let mut menu_open=use_signal(||None::<String>);
    let mut editing_id=use_signal(||None::<String>);
    let mut editing_text=use_signal(String::new);
    let mut confirm_delete=use_signal(||None::<String>);
    let mut history_for=use_signal(||None::<String>);
    let mut revisions=use_signal(||None::<Vec<tickets::MessageRevision>>);
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
            gloo_timers::future::TimeoutFuture::new(2_000).await;
            fallback_poll.restart();
        }
    });
    let base=crate::api::base_url();
    rsx!{
        div{class:"app ticket-app",Header{},
            main{class:"ticket-workspace ticket-detail-page",
                TicketSidebar{selected:Some(id.clone())}
                section{class:"ticket-conversation-area",
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
                                    p{class:"ticket-chat-interlocutor",
                                        if is_staff{
                                            "Interlocuteur : {current.requester_name}"
                                        }else{
                                            "Interlocuteur : équipe support SEPPYL"
                                        }
                                    }
                                    div{class:"ticket-chat-tags",
                                        span{class:"chat-state-pill","{status_label(&current.ticket.status)}"}
                                        if current.ticket.maintainer_required{
                                            span{class:"ticket-escalation","Maintainer"}
                                        }
                                    }
                                }
                                // Live transport state is exposed to assistive tech, not as a
                                // distracting "En direct" item in the conversation header.
                                span{class:"ticket-live-a11y","data-ticket-live-status":"true","Connexion…"}
                                if is_staff {
                                    div{class:"ticket-chat-admin","aria-label":"Gestion du ticket",
                                        div{class:"ticket-admin-status",
                                            label{r#for:"ticket-status-select","Statut"}
                                            select{id:"ticket-status-select",
                                                value:"{status}",
                                                onchange:move |e|status.set(e.value()),
                                                option{value:"","{status_label(&current.ticket.status)}"}
                                                option{value:"open","Ouvert"}
                                                option{value:"in_progress","En cours"}
                                                option{value:"resolved","Résolu"}
                                                option{value:"closed","Fermé"}
                                            }
                                        }
                                        label{class:"ticket-admin-maintainer",
                                            title:"Signaler que le ticket nécessite un Maintainer",
                                            input{r#type:"checkbox",
                                                checked:escalation.read().unwrap_or(current.ticket.maintainer_required),
                                                onchange:move |e|escalation.set(Some(e.checked()))}
                                            span{"Maintainer"}
                                        }
                                        button{r#type:"button",class:"button solid ticket-admin-save",
                                            disabled:*busy.read(),
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
                                        }
                                            ,"Appliquer"
                                        }
                                    }
                                }
                            }
                            div{class:"ticket-chat-feed",role:"log","aria-label":"Messages du ticket","aria-live":"polite",
                                for group in group_messages(current.messages.clone()){
                                    div{class:if group.author_id==current_user{"ticket-message-group own"}else{"ticket-message-group"},
                                        key:"{group.messages[0].id}",
                                        "data-group-start":"{group.started_at}",
                                        div{class:"ticket-group-author",
                                            if group.author_id==current_user{"Vous"}
                                            else if is_staff{"Demandeur"}
                                            else{"Équipe support"}
                                        }
                                        for msg in group.messages{
                                            article{key:"{msg.id}",
                                                class:if msg.author_id==current_user{"ticket-chat-message ticket-message own"}else{"ticket-chat-message ticket-message"},
                                                "data-message-id":"{msg.id}",
                                                div{class:"ticket-chat-line",
                                                    if editing_id.read().as_ref().is_some_and(|v|v==&msg.id){
                                                        form{class:"ticket-inline-edit",
                                                            onsubmit:{
                                                                let token=token.clone();let tid=id.clone();let mid=msg.id.clone();
                                                                move |event|{
                                                                    event.prevent_default();
                                                                    let next=editing_text.read().trim().to_string();
                                                                    if next.is_empty()||*busy.read(){return;}
                                                                    busy.set(true);
                                                                    let token=token.clone();let tid=tid.clone();let mid=mid.clone();
                                                                    spawn(async move{
                                                                        match tickets::edit_message(&token,&tid,&mid,&next).await{
                                                                            Ok(())=>{editing_id.set(None);detail.restart();error.set(None);},
                                                                            Err(e)=>error.set(Some(e))
                                                                        }
                                                                        busy.set(false);
                                                                    });
                                                                }
                                                            },
                                                            textarea{"data-ticket-edit-composer":"true",
                                                                rows:"3",maxlength:"10000",
                                                                value:"{editing_text}",oninput:move |e|editing_text.set(e.value())}
                                                            div{class:"ticket-edit-actions",
                                                                button{r#type:"button",class:"button quiet",
                                                                    onclick:move |_|editing_id.set(None),"Annuler"}
                                                                button{r#type:"submit",class:"button solid",
                                                                    disabled:*busy.read()||editing_text.read().trim().is_empty(),
                                                                    "Enregistrer la modification"}
                                                            }
                                                        }
                                                    }else{
                                                        div{class:"ticket-chat-bubble",
                                                        if let Some(parent_id)=msg.reply_to_id.as_ref(){
                                                            if let Some(parent)=current.messages.iter().find(|p|&p.id==parent_id){
                                                                button{r#type:"button",class:"ticket-reply-quote",
                                                                    "data-ticket-jump-to":"{parent.id}",
                                                                    title:"Aller au message cité",
                                                                    strong{if parent.author_id==current_user{"Vous"}
                                                                        else if is_staff{"Demandeur"}else{"Support"}}
                                                                    span{if parent.deleted_at.is_some(){"Message supprimé"}
                                                                        else if parent.message.is_empty(){"Image jointe"}
                                                                        else{"{parent.message.chars().take(110).collect::<String>()}"}}
                                                                }
                                                            }else{span{class:"ticket-reply-missing","Message cité indisponible"}}
                                                        }
                                                            if msg.deleted_at.is_some(){
                                                                em{class:"ticket-message-deleted","Message supprimé par un administrateur"}
                                                            }else{
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
                                                    time{class:"ticket-message-hover-time",
                                                        title:"{format_time(msg.created_at)}",
                                                        "{format_time(msg.created_at)}"
                                                    }
                                                    if msg.edited_at.is_some() && msg.deleted_at.is_none(){
                                                        span{class:"ticket-edited-flag",title:"Message modifié","modifié"}
                                                    }
                                                    div{class:if menu_open.read().as_ref().is_some_and(|v|v==&msg.id){
                                                            "ticket-message-actions is-open"
                                                        }else{"ticket-message-actions"},
                                                        button{r#type:"button",class:"ticket-message-menu-trigger",
                                                            title:"Options du message","aria-label":"Options du message",
                                                            "aria-haspopup":"menu",
                                                            "aria-expanded":if menu_open.read().as_ref().is_some_and(|v|v==&msg.id){"true"}else{"false"},
                                                            onclick:{
                                                                let mid=msg.id.clone();
                                                                move |_|{
                                                                    if menu_open.read().as_ref().is_some_and(|v|v==&mid){
                                                                        menu_open.set(None);
                                                                    }else{menu_open.set(Some(mid.clone()));}
                                                                }
                                                            },
                                                            svg{width:"18",height:"18",view_box:"0 0 24 24",fill:"currentColor",
                                                                circle{cx:"5",cy:"12",r:"1.8"}
                                                                circle{cx:"12",cy:"12",r:"1.8"}
                                                                circle{cx:"19",cy:"12",r:"1.8"}
                                                            }
                                                        }
                                                        if menu_open.read().as_ref().is_some_and(|v|v==&msg.id){
                                                            div{class:"ticket-message-menu",role:"menu","aria-label":"Actions sur le message",
                                                                if msg.deleted_at.is_none() && (current.ticket.status!="closed"||is_staff){
                                                                    button{r#type:"button",role:"menuitem",class:"ticket-msg-action",
                                                                        onclick:{
                                                                            let target=msg.clone();
                                                                            move |_|{
                                                                                reply_target.set(Some(target.clone()));
                                                                                menu_open.set(None);
                                                                            }
                                                                        },"Répondre"}
                                                                }
                                                                if msg.deleted_at.is_none() && !msg.message.is_empty(){
                                                                    button{r#type:"button",role:"menuitem",class:"ticket-msg-action",
                                                                        "data-ticket-copy-text":"{msg.message}",
                                                                        onclick:move |_|menu_open.set(None),"Copier le texte"}
                                                                }
                                                        if msg.author_id==current_user && msg.deleted_at.is_none() && !msg.message.is_empty(){
                                                            button{r#type:"button",role:"menuitem",class:"ticket-msg-action",
                                                                "data-ticket-edit-from-menu":"true",
                                                                title:"Modifier ce message",
                                                                onclick:{
                                                                    let mid=msg.id.clone();let content=msg.message.clone();
                                                                    move |_|{editing_id.set(Some(mid.clone()));editing_text.set(content.clone());menu_open.set(None);}
                                                                },"Modifier"}
                                                        }

                                                        if is_staff && (msg.edited_at.is_some() || msg.deleted_at.is_some()){
                                                            button{r#type:"button",role:"menuitem",class:"ticket-msg-action",
                                                                onclick:{
                                                                    let token=token.clone();let tid=id.clone();let mid=msg.id.clone();
                                                                    move |_|{
                                                                        history_for.set(Some(mid.clone()));revisions.set(None);menu_open.set(None);
                                                                        let token=token.clone();let tid=tid.clone();let mid=mid.clone();
                                                                        spawn(async move{
                                                                            match tickets::message_history(&token,&tid,&mid).await{
                                                                                Ok(rows)=>revisions.set(Some(rows)),
                                                                                Err(e)=>{history_for.set(None);error.set(Some(e));}
                                                                            }
                                                                        });
                                                                    }
                                                                },"Historique"}
                                                        }

                                                        if is_staff && msg.deleted_at.is_none(){
                                                            button{r#type:"button",role:"menuitem",class:"ticket-msg-action danger",
                                                                onclick:{
                                                                    let mid=msg.id.clone();
                                                                    move |_|{confirm_delete.set(Some(mid.clone()));menu_open.set(None);}
                                                                },"Supprimer"}
                                                        }
                                                    }

                                                            }
                                                        }
                                                    }
                                                    if msg.author_id==current_user && msg.deleted_at.is_none() && !msg.message.is_empty(){
                                                        button{r#type:"button",class:"ticket-edit-shortcut-target",
                                                            tabindex:"-1","aria-hidden":"true","data-ticket-edit":"true",
                                                            onclick:{
                                                                let mid=msg.id.clone();let content=msg.message.clone();
                                                                move |_|{
                                                                    editing_id.set(Some(mid.clone()));
                                                                    editing_text.set(content.clone());
                                                                    menu_open.set(None);
                                                                }
                                                            }}
                                                    }
                                                }
                                                if confirm_delete.read().as_ref().is_some_and(|v|v==&msg.id){
                                                    div{class:"ticket-delete-confirm",role:"group",
                                                        span{"Masquer ce message ? Une copie restera dans l’historique."}
                                                        button{r#type:"button",class:"button quiet",
                                                            onclick:move |_|confirm_delete.set(None),"Annuler"}
                                                        button{r#type:"button",class:"button solid",
                                                            disabled:*busy.read(),
                                                            onclick:{
                                                                let token=token.clone();let tid=id.clone();let mid=msg.id.clone();
                                                                move |_|{
                                                                    if *busy.read(){return;}
                                                                    busy.set(true);
                                                                    let token=token.clone();let tid=tid.clone();let mid=mid.clone();
                                                                    spawn(async move{
                                                                        match tickets::delete_message(&token,&tid,&mid).await{
                                                                            Ok(())=>{confirm_delete.set(None);detail.restart();error.set(None);},
                                                                            Err(e)=>error.set(Some(e))
                                                                        }
                                                                        busy.set(false);
                                                                    });
                                                                }
                                                            },"Confirmer la suppression"}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(name)=peer_typing.read().as_ref(){
                                    div{class:"ticket-typing-indicator",role:"status",
                                        span{class:"typing-dots",i{} i{} i{}}
                                        "{name} est en train d'écrire…"
                                    }
                                }
                            }
                            if let Some(mid)=history_for.read().as_ref(){
                                div{class:"dialog-backdrop",
                                    div{class:"dialog-panel ticket-history-dialog",role:"dialog",
                                        "aria-modal":"true","aria-label":"Historique du message",
                                        div{class:"dialog-header",
                                            h2{"Historique du message"}
                                            button{r#type:"button",class:"icon-button",
                                                onclick:move |_|history_for.set(None),"×"}
                                        }
                                        div{class:"dialog-body",
                                            p{class:"muted-small",
                                                "Chaque version antérieure est conservée, y compris après suppression."}
                                            if let Some(versions)=revisions.read().as_ref(){
                                                for version in versions {
                                                    div{class:"ticket-revision",key:"{version.id}",
                                                        div{class:"ticket-revision-meta",
                                                            strong{if version.action=="delete"{"Suppression"}
                                                                else if version.action=="restore"{"Restauration"}
                                                                else{"Modification"}}
                                                            time{"{format_time(version.changed_at)}"}
                                                        }
                                                        pre{"{version.previous_text}"}
                                                        if let Some(attachment)=version.previous_attachment_name.as_ref(){
                                                            p{"Image archivée : {attachment}"}
                                                        }
                                                        button{r#type:"button",class:"button outline",
                                                            disabled:*busy.read(),
                                                            onclick:{
                                                                let token=token.clone();let tid=id.clone();
                                                                let mid=mid.clone();let revision_id=version.id.clone();
                                                                move |_|{
                                                                    if *busy.read(){return;}
                                                                    busy.set(true);
                                                                    let token=token.clone();let tid=tid.clone();
                                                                    let mid=mid.clone();let revision_id=revision_id.clone();
                                                                    spawn(async move{
                                                                        match tickets::restore_message(&token,&tid,&mid,&revision_id).await{
                                                                            Ok(())=>{history_for.set(None);detail.restart();error.set(None);},
                                                                            Err(e)=>error.set(Some(e))
                                                                        }
                                                                        busy.set(false);
                                                                    });
                                                                }
                                                            },"Restaurer cette version"}
                                                    }
                                                }
                                            }else{p{"Chargement de l'historique…"}}
                                        }
                                    }
                                }
                            }
                            div{class:"ticket-chat-composer",
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
                                                let target_id=reply_target.read().as_ref().map(|p|p.id.clone());
                                                spawn(async move{
                                                    match tickets::reply(&t,&tid,&value,target_id.as_deref()).await {
                                                        Ok(())=>{
                                                            reply.set(String::new());
                                                            reply_target.set(None);
                                                            detail.restart();error.set(None);
                                                        },
                                                        Err(e)=>error.set(Some(e)),
                                                    }
                                                    busy.set(false);
                                                });
                                            }
                                        },
                                        if let Some(target)=reply_target.read().as_ref(){
                                            div{class:"ticket-reply-composer-preview",
                                                span{class:"ticket-reply-composer-content",
                                                    strong{"En réponse à "}
                                                    span{if target.deleted_at.is_some(){"Message supprimé"}
                                                        else if target.message.is_empty(){"Image jointe"}
                                                        else{"{target.message.chars().take(125).collect::<String>()}"}}
                                                }
                                                button{r#type:"button",title:"Annuler la réponse",
                                                    "aria-label":"Annuler la réponse",
                                                    onclick:move |_|reply_target.set(None),"×"}
                                            }
                                        }
                                        div{class:"ticket-composer-surface",
                                            label{class:"ticket-chat-tool ticket-add-attachment",
                                                title:"Joindre une image","aria-label":"Joindre une image",
                                                svg{width:"21",height:"21",view_box:"0 0 24 24",fill:"none",
                                                    stroke:"currentColor",stroke_width:"2",
                                                    stroke_linecap:"round",stroke_linejoin:"round",
                                                    path{d:"M12 5v14M5 12h14"}
                                                }
                                                input{r#type:"file","data-ticket-attach-input":"true",
                                                    accept:"image/png,image/jpeg,image/webp,image/gif"}
                                            }
                                            textarea{rows:"1",maxlength:"10000","aria-label":"Écrire un message",
                                                "data-ticket-composer":"true",
                                                placeholder:"Écrire un message…",
                                                value:"{reply}",
                                                oninput:move |e|reply.set(e.value())}
                                            button{r#type:"button",class:"ticket-chat-tool ticket-emoji-toggle",
                                                title:"Insérer un émoji","aria-label":"Insérer un émoji",
                                                onclick:move |_|{let next=!*emoji_open.read();emoji_open.set(next);},
                                                svg{width:"22",height:"22",view_box:"0 0 24 24",
                                                    fill:"none",stroke:"currentColor",stroke_width:"1.8",
                                                    stroke_linecap:"round",stroke_linejoin:"round",
                                                    circle{cx:"12",cy:"12",r:"9"}
                                                    path{d:"M8 14s1.5 2 4 2 4-2 4-2"}
                                                    path{d:"M9 9h.01M15 9h.01"}
                                                }
                                            }
                                            button{r#type:"submit",class:"ticket-send-icon",
                                                title:"Envoyer le message","aria-label":"Envoyer",
                                                disabled:*busy.read()||reply.read().trim().is_empty(),
                                                svg{width:"20",height:"20",view_box:"0 0 24 24",
                                                    fill:"none",stroke:"currentColor",stroke_width:"1.9",
                                                    stroke_linecap:"round",stroke_linejoin:"round",
                                                    path{d:"m22 2-7 20-4-9-9-4Z"}
                                                    path{d:"M22 2 11 13"}
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
                    },
                    Some(Err(e))=>rsx!{p{class:"form-error","{e}"}},
                    None=>rsx!{p{class:"ticket-detail-loading","Chargement du ticket…"}}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod group_tests{
    use super::*;
    fn message(author:&str,timestamp:i64)->tickets::Message{
        tickets::Message{
            id:format!("{author}-{timestamp}"),author_id:author.into(),
            message:"test".into(),created_at:timestamp,
            attachment_name:None,reply_to_id:None,edited_at:None,deleted_at:None
        }
    }
    #[test]
    fn ten_minute_groups_anchor_to_first_message_not_previous(){
        let groups=group_messages(vec![
            message("alice",0),message("alice",300),message("alice",600),
            message("alice",660),message("bob",670),message("bob",1200),
        ]);
        assert_eq!(groups.iter().map(|g|g.messages.len()).collect::<Vec<_>>(),
            vec![2,2,2]);
        assert_eq!(groups.iter().map(|g|g.started_at).collect::<Vec<_>>(),
            vec![0,600,670]);
    }
}
