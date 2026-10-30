use dioxus::prelude::*;
use crate::api::{organization as org,players};
use super::{Route,layout::Header,membership::MembershipPanel,team_management::{TeamManagerRow,CreateTeamPlayer},club_information::ClubInformationEditor,team_information::TeamInformationEditor};

#[component]
pub fn Organization()->Element{
    let session=use_context::<Signal<Option<String>>>();
    let token=session.read().clone();
    let navigator=use_navigator();
    let fetch_token=token.clone();
    let mut clubs=use_resource(move ||{
        let token=fetch_token.clone();
        async move{
            match token{Some(token)=>org::clubs(&token).await,None=>Err("Connexion requise.".into())}
        }
    });
    let mut club_id=use_signal(String::new);
    // Open the first authorized club automatically; users can switch later.
    use_effect(move ||{
        let initial=if club_id.read().is_empty(){
            clubs.read().clone().and_then(|v|v.ok())
                .and_then(|items|items.first().map(|item|item.id.clone()))
        }else{None};
        if let Some(id)=initial{club_id.set(id);}
    });
    let team_token=token.clone();
    let mut teams=use_resource(move ||{
        let token=team_token.clone();
        let id=club_id.read().clone();
        async move{
            if id.is_empty(){Ok(Vec::new())}else{
                match token{Some(token)=>org::teams(&token,&id).await,None=>Err("Connexion requise.".into())}
            }
        }
    });
    let caps_token=token.clone();
    let capabilities=use_resource(move ||{
        let token=caps_token.clone();
        let id=club_id.read().clone();
        async move{
            if id.is_empty(){Err("Choisissez un club.".into())}
            else{match token{Some(token)=>org::capabilities(&token,&id).await,None=>Err("Connexion requise.".into())}}
        }
    });
    let mut team_id=use_signal(String::new);
    let club_roster_token=token.clone();
    let club_roster=use_resource(move ||{
        let token=club_roster_token.clone();
        let id=club_id.read().clone();
        async move{
            if id.is_empty(){Ok(Vec::new())}else{
                match token{Some(token)=>org::club_players(&token,&id).await,None=>Err("Connexion requise.".into())}
            }
        }
    });
    let mut tab=use_signal(||"overview".to_string());
    let mut new_team=use_signal(String::new);
    let mut roster_action=use_signal(||"create".to_string());
    let mut player_id=use_signal(String::new);
    let mut busy=use_signal(||false);
    let mut notice=use_signal(||None::<String>);
    let roster_token=token.clone();
    let mut roster=use_resource(move ||{
        let token=roster_token.clone();
        let id=team_id.read().clone();
        async move{
            if id.is_empty(){Ok(Vec::new())}else{
                match token{Some(token)=>org::team_players(&token,&id).await,None=>Err("Connexion requise.".into())}
            }
        }
    });
    let player_token=token.clone();
    let mut own_players=use_resource(move ||{
        let token=player_token.clone();
        async move{
            match token{Some(token)=>players::list(&token).await,None=>Err("Connexion requise.".into())}
        }
    });
    if token.is_none(){
        return rsx!{div{class:"centered",
            p{"Connectez-vous pour consulter vos équipes."}
            button{class:"button solid",onclick:move |_|{navigator.push(Route::Login{});},"Connexion"}
        }};
    }
    let token=token.unwrap_or_default();
    let clubs_list=clubs.read().as_ref().and_then(|v|v.as_ref().ok()).cloned().unwrap_or_default();
    let teams_list=teams.read().as_ref().and_then(|v|v.as_ref().ok()).cloned().unwrap_or_default();
    let caps=capabilities.read().as_ref().and_then(|v|v.as_ref().ok()).cloned();
    let can_manage=caps.as_ref().is_some_and(|c|c.can_manage);
    let can_write_team=caps.as_ref().is_some_and(|c|c.can_write_teams.contains(&team_id.read()));
    let selected_team_label=teams_list.iter().find(|t|t.id==*team_id.read())
        .map(|t|t.name.clone()).unwrap_or_else(||"Effectif de l'équipe".to_string());
    let current_club=clubs_list.iter().find(|c|c.id==*club_id.read()).cloned();
    let is_public=current_club.as_ref().and_then(|c|c.is_public)==Some(true);
    let team_for_members=team_id.read().clone();
    let club_for_members=club_id.read().clone();
    rsx!{
        div{class:"app",
            Header{}
            main{class:"page organization-page workspace-page",
                div{class:"page-heading",
                    div{
                        h1{"Mon club"}
                        p{"Les équipes regroupent les joueuses, les entraînements et les entraîneurs qui y ont accès."}
                    }
                }
                if let Some(message)=notice.read().as_ref(){
                    div{class:"seppyl-toast",role:"status",
                        span{class:"seppyl-toast-message","{message}"}
                        button{r#type:"button",class:"seppyl-toast-close",
                            "data-toast-dismiss":"true","aria-label":"Fermer la notification","×"}
                    }
                }
                div{class:"club-layout",
                    aside{class:"club-sidebar",
                        div{class:"club-sidebar-heading",
                            strong{"Mes clubs"}
                            span{class:"club-sidebar-count","{clubs_list.len()}"}
                        }
                        nav{class:"club-sidebar-list","aria-label":"Choisir un club",
                            for club in &clubs_list{
                                button{
                                    key:"{club.id}",
                                    class:if *club_id.read()==club.id{"club-sidebar-item active"}else{"club-sidebar-item"},
                                    "aria-current":if *club_id.read()==club.id{"page"}else{"false"},
                                    onclick:{
                                        let id=club.id.clone();
                                        move |_|{
                                            club_id.set(id.clone());
                                            team_id.set(String::new());
                                            tab.set("overview".into());
                                            notice.set(None);
                                        }
                                    },
                                    span{class:"club-sidebar-avatar",
                                        if let Some(src)=org::club_logo_src(club.logo_url.as_deref()){
                                            img{src:"{src}",alt:"",
                                                style:"{org::logo_style(club.logo_x,club.logo_y,club.logo_zoom)}"}
                                        }else{"{club.name.chars().next().unwrap_or('C').to_uppercase()}"}
                                    }
                                    span{class:"club-sidebar-name","{club.name}"}
                                }
                            }
                        }
                        if clubs_list.is_empty(){
                            p{class:"club-sidebar-empty","Aucun club associé à ce compte."}
                        }
                    }
                    div{class:"club-main",
                if club_id.read().is_empty(){
                    div{class:"workspace-empty",
                        h2{"Sélectionnez un club"}
                        p{"Choisissez un club pour retrouver ses équipes et les effectifs partagés. Si la liste est vide, demandez à un responsable de vous rattacher."}
                        if let Some(Err(message))=clubs.read().clone(){p{role:"alert","{message}"}}
                    }
                }else{
                    nav{class:"workspace-tabs",role:"tablist","aria-label":"Sections du club",
                        button{class:if *tab.read()=="overview"{"selected"}else{""},
                            "aria-selected":if *tab.read()=="overview"{"true"}else{"false"},
                            onclick:move |_|tab.set("overview".into()),"Vue d’ensemble"}
                        button{class:if *tab.read()=="teams"{"selected"}else{""},
                            onclick:move |_|{team_id.set(String::new());tab.set("teams".into());},"Équipes"}
                        if can_manage{
                            button{class:if *tab.read()=="access"{"selected"}else{""},
                                onclick:move |_|tab.set("access".into()),"Accès"}
                            button{class:if *tab.read()=="settings"{"selected"}else{""},
                                onclick:move |_|tab.set("settings".into()),"Informations"}
                        }
                    }
                    div{class:"workspace-tab-panel",
                        if *tab.read()=="overview"{
                            section{class:"panel",
                                div{class:"panel-head",
                                    div{h2{"Équipes du club"}p{"Choisissez une équipe pour consulter son effectif."}}
                                }
                                div{class:"workspace-card-grid",
                                    for team in &teams_list{
                                        button{class:"workspace-team-card",key:"{team.id}",
                                            onclick:{
                                                let id=team.id.clone();
                                                move |_|{
                                                    team_id.set(id.clone());
                                                    tab.set("roster".into());
                                                }
                                            },
                                            span{class:"workspace-team-icon",
                                                if let Some(src)=org::club_logo_src(team.logo_url.as_deref()
                                                    .or_else(||current_club.as_ref().and_then(|c|c.logo_url.as_deref()))){
                                                    img{src:"{src}",alt:"",
                                                        style:"{org::logo_style(team.logo_x.or_else(||current_club.as_ref().and_then(|c|c.logo_x)),team.logo_y.or_else(||current_club.as_ref().and_then(|c|c.logo_y)),team.logo_zoom.or_else(||current_club.as_ref().and_then(|c|c.logo_zoom)))}"}
                                                }else{"⚽"}
                                            }
                                            strong{"{team.name}"}
                                            span{class:"muted-small","Consulter l'effectif"}
                                        }
                                    }
                                }
                                if teams_list.is_empty(){div{class:"panel-empty","Aucune équipe accessible."}}
                            }
                        }else if *tab.read()=="roster"{
                            if team_id.read().is_empty(){
                                section{class:"panel",
                                    div{class:"panel-head",
                                        div{h2{"Effectif du club"}
                                            p{"Toutes les joueuses rattachées à ce club. Choisissez une équipe pour consulter ou modifier son effectif."}}
                                    }
                                    match club_roster.read().clone(){
                                        Some(Ok(people))=>rsx!{
                                            if people.is_empty(){p{class:"panel-empty","Aucune joueuse rattachée à ce club."}}
                                            div{class:"workspace-roster-list",
                                                for player in people{
                                                    div{class:"workspace-roster-item",key:"{player.id}",
                                                        span{class:"roster-number","{player.number}"}
                                                        strong{"{player.firstname} {player.lastname}"}
                                                    }
                                                }
                                            }
                                        },
                                        Some(Err(error))=>rsx!{p{class:"panel-empty","{error}"}},
                                        None=>rsx!{p{class:"panel-empty","Chargement de l’effectif…"}},
                                    }
                                }
                            }else{
                                div{class:"club-roster-grid",
                                section{class:"panel",
                                    div{class:"panel-head",
                                        div{h2{"{selected_team_label}"}p{"Joueuses de cette équipe"}}
                                        button{class:"button outline small",r#type:"button",
                                            onclick:move |_|{team_id.set(String::new());tab.set("teams".into());},
                                            "Retour aux équipes"}
                                    }
                                    match roster.read().clone(){
                                        Some(Ok(items))=>rsx!{
                                            if items.is_empty(){div{class:"panel-empty","Aucune joueuse dans cette équipe."}}
                                            div{class:"workspace-roster-list",
                                                for player in items{
                                                    div{class:"workspace-roster-item",key:"{player.id}",
                                                        span{class:"roster-number","{player.number}"}
                                                        strong{"{player.firstname} {player.lastname}"}
                                                        if can_write_team{
                                                            button{class:"remove-link",disabled:*busy.read(),
                                                                onclick:{
                                                                    let uid=player.id.clone();
                                                                    let token=token.clone();
                                                                    move |_|{
                                                                        let token=token.clone();
                                                                        let id=team_id.read().clone();
                                                                        let uid=uid.clone();
                                                                        busy.set(true);
                                                                        spawn(async move{
                                                                            match org::remove_team_player(&token,&id,&uid).await{
                                                                                Ok(())=>{roster.restart();notice.set(Some("Joueuse retirée.".into()));}
                                                                                Err(e)=>notice.set(Some(e)),
                                                                            }
                                                                            busy.set(false);
                                                                        });
                                                                    }
                                                                },"Retirer"}
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                        Some(Err(error))=>rsx!{p{class:"panel-empty","{error}"}},
                                        None=>rsx!{p{class:"panel-empty","Chargement…"}},
                                    }
                                }
                                if can_write_team{
                                    section{class:"panel workspace-secondary",
                                        div{class:"panel-head",div{
                                            h2{"Compléter l’effectif"}
                                            p{"Les joueuses créées ici sont immédiatement partagées avec les membres autorisés de l’équipe."}
                                        }}
                                        nav{class:"workspace-subtabs",
                                            button{class:if *roster_action.read()=="create"{"selected"}else{""},
                                                onclick:move |_|roster_action.set("create".into()),"Créer une joueuse"}
                                            button{class:if *roster_action.read()=="existing"{"selected"}else{""},
                                                onclick:move |_|roster_action.set("existing".into()),"Ajouter une joueuse existante"}
                                        }
                                        if *roster_action.read()=="create"{
                                            CreateTeamPlayer{token:token.clone(),team_id:team_id.read().clone(),
                                                on_saved:move |_|{roster.restart();own_players.restart();notice.set(Some("Joueuse créée dans l’équipe.".into()));},
                                                on_error:move |error|notice.set(Some(error))}
                                        }else{
                                            p{class:"workspace-hint","Sélectionnez une joueuse existante pour l'ajouter à cette équipe sans créer de doublon."}
                                        form{class:"workspace-form",
                                            onsubmit:{
                                                let token=token.clone();
                                                move |event|{
                                                    event.prevent_default();
                                                    let uid=player_id.read().clone();
                                                    if uid.is_empty()||*busy.read(){return;}
                                                    let token=token.clone();
                                                    let id=team_id.read().clone();
                                                    busy.set(true);
                                                    spawn(async move{
                                                        match org::add_team_player(&token,&id,&uid).await{
                                                            Ok(())=>{roster.restart();player_id.set(String::new());notice.set(Some("Joueuse ajoutée.".into()));}
                                                            Err(e)=>notice.set(Some(e)),
                                                        }
                                                        busy.set(false);
                                                    });
                                                }
                                            },
                                            div{class:"field",label{"Joueuse"},
                                                select{value:"{player_id}",required:true,
                                                    onchange:move |e|player_id.set(e.value()),
                                                    option{value:"","Sélectionner une joueuse"}
                                                    if let Some(Ok(people))=own_players.read().clone(){
                                                        for player in people{
                                                            option{key:"{player.id}",value:"{player.id}",
                                                                "{player.firstname} {player.lastname} · N°{player.number}"}
                                                        }
                                                    }
                                                }
                                            }
                                            button{class:"button solid",r#type:"submit",disabled:*busy.read(),"Ajouter à l'effectif"}
                                        }
                                        }
                                    }
                                }
                                }
                            }
                        }else if *tab.read()=="team-info" && can_manage{
                            div{class:"admin-roster-heading",
                                button{class:"button outline small",r#type:"button",
                                    onclick:move |_|tab.set("teams".into()),"Retour aux équipes"}
                            }
                            if let Some(selected)=teams_list.iter().find(|t|t.id==*team_id.read()).cloned(){
                                TeamInformationEditor{key:"{selected.id}",
                                    token:token.clone(),team:selected,
                                    on_saved:move |_|{teams.restart();notice.set(Some("Informations de l'équipe enregistrées.".into()));},
                                    on_error:move |error|notice.set(Some(error))
                                }
                            }
                        }else if *tab.read()=="teams"{
                            section{class:"panel",
                                div{class:"panel-head",
                                    div{h2{"Équipes"}p{"Consultez les effectifs et gérez vos équipes depuis un seul endroit."}}
                                }
                                div{class:"workspace-roster-list",
                                    for team in &teams_list{
                                        div{class:"club-team-row",key:"{team.id}",
                                            div{class:"club-team-name",
                                                strong{"{team.name}"}
                                                if let Some(description)=team.description.as_deref(){
                                                    span{class:"muted-small","{description}"}
                                                }else{
                                                    span{class:"muted-small","Équipe du club"}
                                                }
                                            }
                                            button{class:"button outline small",r#type:"button",
                                                onclick:{
                                                    let id=team.id.clone();
                                                    move |_|{
                                                        team_id.set(id.clone());
                                                        roster_action.set("create".into());
                                                        tab.set("roster".into());
                                                    }
                                                },
                                                "Voir l'effectif"
                                            }
                                            if can_manage{
                                                button{class:"button outline small",r#type:"button",
                                                    onclick:{
                                                        let id=team.id.clone();
                                                        move |_|{team_id.set(id.clone());tab.set("team-info".into());}
                                                    },"Informations"}
                                                TeamManagerRow{team:team.clone(),token:token.clone(),
                                                    on_saved:move |_|{teams.restart();notice.set(Some("Équipe actualisée.".into()));},
                                                    on_error:move |error|notice.set(Some(error))}
                                            }
                                        }
                                    }
                                    if teams_list.is_empty(){div{class:"panel-empty","Aucune équipe dans ce club."}}
                                }
                                if can_manage{
                                    form{class:"workspace-form",
                                        onsubmit:{
                                            let token=token.clone();
                                            move |event|{
                                                event.prevent_default();
                                                let value=new_team.read().trim().to_string();
                                                let club=club_id.read().clone();
                                                if value.is_empty()||club.is_empty()||*busy.read(){return;}
                                                let token=token.clone();
                                                busy.set(true);
                                                spawn(async move{
                                                    match org::create_team(&token,&club,&value).await{
                                                        Ok(())=>{new_team.set(String::new());teams.restart();notice.set(Some("Équipe créée.".into()));}
                                                        Err(e)=>notice.set(Some(e)),
                                                    }
                                                    busy.set(false);
                                                });
                                            }
                                        },
                                        div{class:"field",label{"Nouvelle équipe"},
                                            input{value:"{new_team}",required:true,maxlength:"120",
                                                placeholder:"Ex. U17 féminines",oninput:move |e|new_team.set(e.value())}}
                                        button{class:"button solid",r#type:"submit",disabled:*busy.read(),"Créer l'équipe"}
                                    }
                                }
                            }
                        }else if *tab.read()=="access" && can_manage{
                            nav{class:"workspace-subtabs",
                                button{class:if team_id.read().is_empty(){"selected"}else{""},
                                    onclick:move |_|team_id.set(String::new()),"Accès au club"}
                                for team in &teams_list{
                                    button{key:"{team.id}",class:if *team_id.read()==team.id{"selected"}else{""},
                                        onclick:{
                                            let id=team.id.clone();
                                            move |_|team_id.set(id.clone())
                                        },"{team.name}"}
                                }
                            }
                            MembershipPanel{
                                key:"{club_for_members}:{team_for_members}",
                                token:token.clone(),club_id:club_for_members.clone(),
                                id:if team_for_members.is_empty(){club_for_members.clone()}else{team_for_members.clone()},
                                scope:if team_for_members.is_empty(){"club".to_string()}else{"team".to_string()},
                                can_manage:true,default_user:None,
                            }
                        }else if *tab.read()=="settings" && can_manage{
                            if let Some(selected_club)=current_club.clone(){
                                ClubInformationEditor{
                                    key:"{selected_club.id}",
                                    token:token.clone(),
                                    club:selected_club,
                                    on_saved:move |_|{clubs.restart();notice.set(Some("Informations du club enregistrées.".into()));},
                                    on_error:move |error|notice.set(Some(error)),
                                }
                            }
                        }
                    }
                }
                    }
                }
            }
        }
    }
}
