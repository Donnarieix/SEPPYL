use dioxus::prelude::*;
use crate::api::organization;
use super::Route;

#[component]
pub fn PublicDirectory()->Element{
    let clubs=use_resource(||async{organization::public_clubs().await});
    rsx!{
        main { class:"page public-directory",
            div{class:"page-heading",
                div {h1{"SEPPYL · Annuaire public"}p{"Les clubs ayant choisi d'être visibles publiquement."}}
                Link{to:Route::Login{},class:"button solid public-login-action","Connexion"}
            }
            match clubs.read().clone(){
                Some(Ok(items))=>rsx!{
                    if items.is_empty(){div{class:"empty-state","Aucun club public pour le moment."}}
                    div{class:"catalog-grid",
                        for club in items{
                            article{class:"catalog-card",key:"{club.id}",
                                div{class:"catalog-content",
                                    div{class:"public-club-heading",
                                        if let Some(src)=organization::club_logo_src(club.logo_url.as_deref()){
                                            span{class:"public-club-logo",img{src:"{src}",alt:"Logo de {club.name}"}}
                                        }
                                        div{
                                            h2{"{club.name}"}
                                            if let Some(description)=club.description.as_deref(){
                                                p{"{description}"}
                                            }
                                        }
                                    }
                                    PublicTeams{club_id:club.id.clone()}
                                }
                            }
                        }
                    }
                },
                Some(Err(message))=>rsx!{p{class:"load-state","{message}"}},
                None=>rsx!{p{class:"load-state","Chargement des clubs…"}},
            }
        }
    }
}
#[component]
fn PublicTeams(club_id:String)->Element{
    let choices=use_resource(move ||{let id=club_id.clone();async move{organization::public_teams(&id).await}});
    rsx!{
        match choices.read().clone(){
            Some(Ok(items))=>rsx!{
                if items.is_empty(){p{"Aucune équipe publiée."}}
                for team in items{p{class:"muted-small",key:"{team.id}","{team.name}"}}
            },
            Some(Err(_))=>rsx!{p{"Équipes indisponibles."}},
            None=>rsx!{p{"Chargement…"}},
        }
    }
}
