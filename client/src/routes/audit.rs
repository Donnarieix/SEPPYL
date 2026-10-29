use dioxus::prelude::*;
use crate::api::audit;
use super::{Route,layout::Header};
#[component]
pub fn AuditLog()->Element{
    let session=use_context::<Signal<Option<String>>>();
    let navigator=use_navigator();
    let token=session.read().clone();
    let mut page=use_signal(||1_u64);
    let mut page_size=use_signal(||50_u64);
    let read_token=token.clone();
    let mut events=use_resource(move ||{
        let token=read_token.clone();
        let p=*page.read();
        let size=*page_size.read();
        async move{match token{
            Some(token)=>audit::list(&token,p,size).await,
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
        div{class:"app",Header{}
            main{class:"page audit-page",
                div{class:"page-heading",
                    div{h1{"Journal d'activité"}
                        p{"Historique complet des opérations, accessible aux administrateurs."}}
                    button{class:"button outline",onclick:move |_|events.restart(),"Actualiser"}
                }
                section{class:"panel",
                    div{class:"audit-pager",
                        div{class:"field",
                            label{"Lignes par page"}
                            select{value:"{page_size}",onchange:move |e|{
                                if let Ok(size)=e.value().parse::<u64>(){
                                    page_size.set(size);
                                    page.set(1);
                                }
                            },
                                for size in [10_u64,50,100,250,500,1000]{
                                    option{key:"{size}",value:"{size}","{size}"}
                                }
                            }
                        }
                        if let Some(Ok(result))=events.read().clone(){
                            span{class:"audit-page-count",
                                "{result.total} événement(s) · Page {result.page} / {result.total_pages.max(1)}"}
                            div{class:"audit-page-buttons",
                                button{class:"button outline small",disabled:*page.read()<=1,
                                    onclick:move |_|{let value=*page.read();if value>1{page.set(value-1);}},"Précédent"}
                                button{class:"button outline small",
                                    disabled:*page.read()>=result.total_pages,
                                    onclick:move |_|{let value=*page.read();page.set(value+1);},"Suivant"}
                            }
                        }
                    }
                    match events.read().clone(){
                        Some(Ok(result))=>rsx!{
                            if result.items.is_empty(){p{class:"panel-empty","Aucun événement sur cette page."}}
                            div{class:"audit-table-wrap",
                                table{class:"audit-table",
                                    thead{tr{th{"Horodatage (UTC)"}th{"Action"}th{"Ressource"}th{"Compte"}th{"HTTP"}}}
                                    tbody{
                                        for item in result.items{
                                            tr{key:"{item.id}",
                                                td{title:"Horodatage Unix en secondes","{item.timestamp}"}
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
