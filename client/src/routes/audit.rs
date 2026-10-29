use dioxus::prelude::*;
use crate::api::audit;
use super::{Route,layout::Header};
#[component]
pub fn AuditLog()->Element{
    let session=use_context::<Signal<Option<String>>>();
    let navigator=use_navigator();
    let token=session.read().clone();
    let read_token=token.clone();
    let mut events=use_resource(move ||{
        let token=read_token.clone();
        async move {match token{
            Some(token)=>audit::list(&token).await,
            None=>Err("Connexion requise.".into())
        }}
    });
    if token.is_none(){return rsx!{
        div{class:"centered",
            p{"Connectez-vous pour consulter le journal."}
            button{class:"button solid",onclick:move |_|{navigator.push(Route::Login{});},"Connexion"}
        }
    };}
    rsx!{
        div{class:"app",
            Header{}
            main{class:"page",
                div{class:"page-heading",
                    div{h1{"Journal d'activité"}
                        p{"Historique des 100 dernières opérations de création, modification et suppression. Accessible aux administrateurs."}}
                    button{class:"button outline",onclick:move |_|events.restart(),"Actualiser"}
                }
                section{class:"panel",
                    match events.read().clone(){
                        Some(Ok(items))=>rsx!{
                            if items.is_empty(){p{class:"panel-empty","Aucune opération enregistrée."}}
                            div{class:"audit-table-wrap",
                                table{class:"audit-table",
                                    thead{tr{
                                        th{"Date UTC"} th{"Action"} th{"Ressource"} th{"Auteur"} th{"HTTP"}
                                    }}
                                    tbody{
                                        for item in items{
                                            tr{key:"{item.id}",
                                                td{title:"Timestamp Unix (secondes)","{item.timestamp}"}
                                                td{"{item.action}"}
                                                td{"{item.path}"}
                                                td{code{"{item.actor_id}"}}
                                                td{"{item.status}"}
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Some(Err(error))=>rsx!{div{class:"panel-empty",role:"alert","{error}"}},
                        None=>rsx!{div{class:"panel-empty","Chargement du journal…"}},
                    }
                }
            }
        }
    }
}
