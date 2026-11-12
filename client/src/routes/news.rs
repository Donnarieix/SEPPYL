use dioxus::prelude::*;
use crate::api::announcements;
use super::Route;
#[component]
pub fn News()->Element{
    let entries=use_resource(||async{announcements::active().await});
    rsx!{
        main{class:"page news-page",
            div{class:"page-heading",
                div{h1{"Nouveautés"}p{"Annonces et informations du service."}}
                Link{to:Route::Login{},class:"button outline","Connexion"}
            }
            match entries.read().clone(){
                Some(Ok(items))=>rsx!{
                    if items.is_empty(){div{class:"empty-state","Aucune annonce en cours."}}
                    for item in items{
                        article{class:"panel",key:"{item.id}",
                            h2{"{item.title}"}
                            p{"{item.body}"}
                            if item.urgent{strong{class:"ticket-escalation","Information importante"}}
                        }
                    }
                },
                Some(Err(e))=>rsx!{p{class:"form-error","{e}"}},
                None=>rsx!{p{"Chargement…"}}
            }
        }
    }
}
