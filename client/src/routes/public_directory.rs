use dioxus::prelude::*;
use crate::api::organization;
use super::Route;

#[component]
pub fn PublicDirectory()->Element{
    let clubs=use_resource(||async{organization::public_clubs().await});
    rsx!{
        document::Title { "Annuaire public des clubs | SEPPYL" }
        document::Link { rel:"canonical", href:"https://seppyl.donnarieix.fr/public" }
        document::Meta { name:"description",
            content:"Découvrez les clubs sportifs et les équipes qui partagent leurs informations sur SEPPYL." }
        document::Meta { name:"robots", content:"index,follow" }
        document::Meta { property:"og:title", content:"Annuaire public des clubs | SEPPYL" }
        document::Meta { property:"og:description",
            content:"Découvrez les clubs et leurs équipes sur SEPPYL." }
        document::Meta { property:"og:type", content:"website" }
        document::Meta { property:"og:url", content:"https://seppyl.donnarieix.fr/public" }
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
                                            span{class:"public-club-logo",img{src:"{src}",alt:"Logo de {club.name}",
                                            style:"{organization::logo_style(club.logo_x,club.logo_y,club.logo_zoom)}"}}
                                        }
                                        div{
                                            h2{"{club.name}"}
                                            if let Some(description)=club.description.as_deref(){
                                                p{"{description}"}
                                            }
                                        }
                                    }
                                    if club.show_public_teams==Some(true){
                                        PublicTeams{club_id:club.id.clone(),show_roster:club.show_public_roster==Some(true)}
                                    }else{
                                        p{class:"muted-small","Les équipes de ce club ne sont pas publiées."}
                                    }
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
fn PublicTeams(club_id:String,show_roster:bool)->Element{
    let choices=use_resource(move ||{let id=club_id.clone();async move{organization::public_teams(&id).await}});
    rsx!{
        match choices.read().clone(){
            Some(Ok(items))=>rsx!{
                if items.is_empty(){p{"Aucune équipe publiée."}}
                for team in items{
                    div{class:"public-team-row",key:"{team.id}",
                        span{class:"workspace-team-icon",
                            if let Some(src)=organization::club_logo_src(team.logo_url.as_deref()){
                                img{src:"{src}",alt:"",
                                    style:"{organization::logo_style(team.logo_x,team.logo_y,team.logo_zoom)}"}
                            }else{"⚽"}
                        }
                        div{strong{"{team.name}"}
                            if let Some(description)=team.description.as_deref(){
                                p{class:"muted-small","{description}"}
                            }
                            if show_roster{PublicRoster{team_id:team.id.clone()}}
                        }
                    }
                }
            },
            Some(Err(_))=>rsx!{p{"Équipes indisponibles."}},
            None=>rsx!{p{"Chargement…"}},
        }
    }
}

#[component]
fn PublicRoster(team_id:String)->Element{
    let people=use_resource(move ||{let id=team_id.clone();async move{organization::public_roster(&id).await}});
    rsx!{
        if let Some(Ok(players))=people.read().clone(){
            div{class:"public-roster",
                strong{"Effectif publié"}
                for player in players{
                    p{key:"{player.id}","{player.firstname} {player.lastname} · n°{player.number}"}
                }
            }
        }
    }
}
