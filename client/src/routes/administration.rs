use dioxus::prelude::*;
use crate::api::{account,organization as org};
use super::{Route,layout::Header,membership::MembershipPanel,
    team_management::{TeamManagerRow,CreateTeamPlayer},club_information::ClubInformationEditor};

#[component]
pub fn Administration()->Element{
    let session=use_context::<Signal<Option<String>>>();
    let navigator=use_navigator();
    let token=session.read().clone();
    let role_token=token.clone();
    let identity=use_resource(move ||{
        let token=role_token.clone();
        async move{match token{Some(token)=>account::get(&token).await,None=>Err("Connexion requise.".into())}}
    });
    let directory_token=token.clone();
    let mut users=use_resource(move ||{
        let token=directory_token.clone();
        async move{match token{Some(token)=>org::users(&token).await,None=>Err("Connexion requise.".into())}}
    });
    let clubs_token=token.clone();
    let mut clubs=use_resource(move ||{
        let token=clubs_token.clone();
        async move{match token{Some(token)=>org::admin_clubs(&token).await,None=>Err("Connexion requise.".into())}}
    });
    let mut tab=use_signal(||"users".to_string());
    let mut selected_user=use_signal(String::new);
    let mut role=use_signal(||"user".to_string());
    let mut club=use_signal(String::new);
    let mut team=use_signal(String::new);
    let team_token=token.clone();
    let teams=use_resource(move ||{
        let token=team_token.clone();
        let id=club.read().clone();
        async move{
            if id.is_empty(){Ok(Vec::new())}else{
                match token{Some(token)=>org::teams(&token,&id).await,None=>Err("Connexion requise.".into())}
            }
        }
    });
    let mut name=use_signal(String::new);
    let mut new_team=use_signal(String::new);
    let mut club_section=use_signal(||"information".to_string());
    let mut roster_player=use_signal(String::new);
    let roster_token=token.clone();
    let mut roster=use_resource(move ||{
        let token=roster_token.clone();
        let team_id=team.read().clone();
        async move{
            if team_id.is_empty(){Ok(Vec::new())}
            else{match token{Some(token)=>org::team_players(&token,&team_id).await,
                None=>Err("Connexion requise.".into())}}
        }
    });
    let candidates_token=token.clone();
    let mut club_candidates=use_resource(move ||{
        let token=candidates_token.clone();
        let id=club.read().clone();
        async move{
            if id.is_empty(){Ok(Vec::new())}
            else{match token{Some(token)=>org::club_players(&token,&id).await,
                None=>Err("Connexion requise.".into())}}
        }
    });
    let mut busy=use_signal(||false);
    let mut notice=use_signal(||None::<String>);
    if token.is_none(){
        return rsx!{div{class:"centered",p{"Connectez-vous pour accéder à l'administration."}
            button{class:"button solid",onclick:move |_|{navigator.push(Route::Login{});},"Connexion"}}
        };
    }
    match identity.read().clone(){
        Some(Ok(value)) if value.role=="administrator"=>{},
        Some(Ok(_))=>return rsx!{
            div{class:"app",Header{}
                main{class:"page",div{class:"workspace-empty",
                    h2{"Accès réservé"}
                    p{"Seuls les administrateurs peuvent gérer tous les comptes et leurs rattachements."}
                    Link{to:Route::Organization{},class:"button outline","Retour à mon club"}
                }}
            }
        },
        Some(Err(error))=>return rsx!{
            div{class:"app",Header{}
                main{class:"page",div{class:"workspace-empty",
                    h2{"Informations indisponibles"}
                    p{role:"alert","{error}"}
                }}
            }
        },
        None=>return rsx!{
            div{class:"app",Header{}
                main{class:"page",div{class:"workspace-empty",
                    p{"Vérification des autorisations…"}
                }}
            }
        },
    }
    let token=token.unwrap_or_default();
    let directory=users.read().as_ref().and_then(|v|v.as_ref().ok()).cloned().unwrap_or_default();
    let clubs_list=clubs.read().as_ref().and_then(|v|v.as_ref().ok()).cloned().unwrap_or_default();
    let teams_list=teams.read().as_ref().and_then(|v|v.as_ref().ok()).cloned().unwrap_or_default();
    let matching=directory.clone();
    let selected_club=clubs_list.iter().find(|item|item.id==*club.read()).cloned();
    let selected=directory.iter().find(|u|u.id==*selected_user.read()).cloned();
    let group_id=if team.read().is_empty(){club.read().clone()}else{team.read().clone()};
    let scope=if team.read().is_empty(){"club"}else{"team"};
    rsx!{
        div{class:"app",
            Header{}
            main{class:"page administration-page workspace-page",
                div{class:"page-heading",
                    div{
                        h1{"Administration"}
                        p{"Gestion globale des utilisateurs, de leurs droits et des clubs."}
                    }
                    span{class:"role-pill role-admin","Administrateur"}
                }
                if let Some(message)=notice.read().as_ref(){
                    p{class:"inline-notice",role:"status","{message}"}
                }
                nav{class:"workspace-tabs",role:"tablist","aria-label":"Administration",
                    button{class:if *tab.read()=="users"{"selected"}else{""},
                        onclick:move |_|tab.set("users".into()),"Utilisateurs"}
                    button{class:if *tab.read()=="memberships"{"selected"}else{""},
                        onclick:move |_|tab.set("memberships".into()),"Rattachements"}
                    button{class:if *tab.read()=="clubs"{"selected"}else{""},
                        onclick:move |_|tab.set("clubs".into()),"Clubs"}
                }
                div{class:"workspace-tab-panel",
                    if *tab.read()=="users"{
                        section{class:"panel",
                            div{class:"panel-head",
                                div{
                                    h2{"Comptes"}
                                    p{"Recherchez un compte pour gérer son rôle et ses rattachements."}
                                }
                            }
                            div{class:"admin-users",
                                for user in matching{
                                    button{key:"{user.id}",
                                        class:if *selected_user.read()==user.id{"admin-user selected"}else{"admin-user"},
                                        onclick:{
                                            let id=user.id.clone();
                                            let user_role=user.role.clone();
                                            move |_|{
                                                selected_user.set(id.clone());
                                                role.set(user_role.clone());
                                            }
                                        },
                                        span{class:"admin-user-avatar","{user.username.chars().next().unwrap_or('?').to_uppercase()}"}
                                        span{class:"admin-user-info",strong{"{user.username}"}
                                            small{"{user.role}"}}
                                        span{class:"admin-user-arrow","›"}
                                    }
                                }
                            }
                            if let Some(Err(message))=users.read().clone(){
                                p{class:"panel-empty","{message}"}
                            }
                        }
                        if let Some(user)=selected{
                            section{class:"panel workspace-secondary",
                                div{class:"panel-head",
                                    div{h2{"Droits de {user.username}"}
                                        p{"Ce rôle détermine les permissions générales du compte."}}
                                }
                                form{class:"workspace-form",
                                    onsubmit:{
                                        let token=token.clone();
                                        move |event|{
                                            event.prevent_default();
                                            let id=selected_user.read().clone();
                                            let new_role=role.read().clone();
                                            let token=token.clone();
                                            if id.is_empty()||*busy.read(){return;}
                                            busy.set(true);
                                            spawn(async move{
                                                match org::assign_global_role(&token,&id,&new_role).await{
                                                    Ok(())=>{users.restart();notice.set(Some("Rôle modifié.".into()));}
                                                    Err(e)=>notice.set(Some(e)),
                                                }
                                                busy.set(false);
                                            });
                                        }
                                    },
                                    div{class:"field",label{"Rôle global"},
                                        select{value:"{role}",onchange:move |e|role.set(e.value()),
                                            option{value:"user","Utilisateur — lecture"}
                                            option{value:"trainer","Entraîneur"}
                                            option{value:"manager","Gestionnaire"}
                                            option{value:"administrator","Administrateur"}
                                        }
                                    }
                                    button{class:"button solid",r#type:"submit",disabled:*busy.read(),"Enregistrer"}
                                }
                                div{class:"account-quick-links",
                                    button{class:"button outline",
                                        onclick:move |_|tab.set("memberships".into()),
                                        "Gérer ses rattachements"
                                    }
                                }
                                p{class:"organization-note","Choisissez ensuite le club ou l'équipe à laquelle rattacher ce compte."}
                            }
                        }
                    }else if *tab.read()=="memberships"{
                        div{class:"workspace-context",
                            div{class:"field",label{"Club"},
                                select{value:"{club}",onchange:move |e|{
                                    club.set(e.value());
                                    team.set(String::new());
                                },
                                    option{value:"","Choisir un club"}
                                    for item in &clubs_list{option{key:"{item.id}",value:"{item.id}","{item.name}"}}
                                }
                            }
                            div{class:"field",label{"Gérer les accès de"},
                                select{value:"{team}",disabled:club.read().is_empty(),
                                    onchange:move |e|team.set(e.value()),
                                    option{value:"","Club entier"}
                                    for item in &teams_list{
                                        option{key:"{item.id}",value:"{item.id}","Équipe · {item.name}"}
                                    }
                                }
                            }
                        }
                        if club.read().is_empty(){
                            div{class:"workspace-empty",h2{"Choisissez un club"},p{"Sélectionnez un club pour administrer ses utilisateurs et ses équipes."}}
                        }else{
                            MembershipPanel{
                                key:"{group_id}",
                                token:token.clone(),club_id:club.read().clone(),
                                scope:scope.to_string(),id:group_id,
                                can_manage:true,default_user:Some(selected_user.read().clone()),
                            }
                        }
                    }else if *tab.read()=="clubs"{
                        div{class:"club-layout admin-club-layout",
                            aside{class:"club-sidebar",
                                div{class:"club-sidebar-heading",
                                    strong{"Clubs de la plateforme"}
                                    span{class:"club-sidebar-count","{clubs_list.len()}"}
                                }
                                nav{class:"club-sidebar-list","aria-label":"Sélectionner un club à administrer",
                                    for item in &clubs_list{
                                        button{key:"{item.id}",
                                            class:if *club.read()==item.id{"club-sidebar-item active"}else{"club-sidebar-item"},
                                            onclick:{
                                                let id=item.id.clone();
                                                move |_|{
                                                    club.set(id.clone());
                                                    team.set(String::new());
                                                    club_section.set("information".into());
                                                    notice.set(None);
                                                }
                                            },
                                            span{class:"club-sidebar-avatar",
                                                if let Some(url)=org::club_logo_src(item.logo_url.as_deref()){
                                                    img{src:"{url}",alt:""}
                                                }else{"{item.name.chars().next().unwrap_or('C').to_uppercase()}"}
                                            }
                                            span{class:"club-sidebar-name","{item.name}"}
                                        }
                                    }
                                }
                                div{class:"admin-club-create",
                                    strong{"Nouveau club"}
                                    form{class:"workspace-form",
                                        onsubmit:{
                                            let token=token.clone();
                                            move |event|{
                                                event.prevent_default();
                                                let label=name.read().trim().to_string();
                                                if label.is_empty()||*busy.read(){return;}
                                                let token=token.clone();
                                                busy.set(true);
                                                spawn(async move{
                                                    match org::create_club(&token,&label).await{
                                                        Ok(())=>{
                                                            name.set(String::new());
                                                            clubs.restart();
                                                            notice.set(Some("Club créé.".into()));
                                                        },
                                                        Err(error)=>notice.set(Some(error))
                                                    }
                                                    busy.set(false);
                                                });
                                            }
                                        },
                                        input{value:"{name}",placeholder:"Nom du club",required:true,
                                            maxlength:"120",oninput:move |e|name.set(e.value())}
                                        button{class:"button solid small",r#type:"submit",disabled:*busy.read(),
                                            "Créer le club"}
                                    }
                                }
                            }
                            div{class:"club-main",
                                if let Some(item)=selected_club.clone(){
                                    div{class:"admin-club-header",
                                        div{
                                            h2{"{item.name}"}
                                            p{"Gestion globale du club, indépendante de vos rattachements personnels."}
                                        }
                                    }
                                    nav{class:"workspace-subtabs","aria-label":"Administration du club",
                                        button{class:if *club_section.read()=="information"{"selected"}else{""},
                                            onclick:move |_|club_section.set("information".into()),"Informations"}
                                        button{class:if *club_section.read()=="teams"{"selected"}else{""},
                                            onclick:move |_|club_section.set("teams".into()),"Équipes et effectifs"}
                                        button{class:if *club_section.read()=="access"{"selected"}else{""},
                                            onclick:move |_|{team.set(String::new());club_section.set("access".into());},"Accès"}
                                    }
                                    if *club_section.read()=="information"{
                                        ClubInformationEditor{key:"{item.id}",token:token.clone(),club:item.clone(),
                                            on_saved:move |_|{clubs.restart();notice.set(Some("Informations enregistrées.".into()));},
                                            on_error:move |error|notice.set(Some(error))}
                                    }else if *club_section.read()=="teams"{
                                        section{class:"panel",
                                            div{class:"panel-head",
                                                div{h2{"Équipes"}p{"Choisissez une équipe pour gérer son effectif."}}
                                            }
                                            div{class:"workspace-roster-list",
                                                for entry in &teams_list{
                                                    div{class:"club-team-row",key:"{entry.id}",
                                                        strong{class:"club-team-name","{entry.name}"}
                                                        button{class:"button outline small",
                                                            onclick:{
                                                                let id=entry.id.clone();
                                                                move |_|{team.set(id.clone());club_section.set("roster".into());}
                                                            },
                                                            "Gérer l'effectif"}
                                                        TeamManagerRow{
                                                            team:entry.clone(),token:token.clone(),
                                                            on_saved:move |_|{teams.restart();notice.set(Some("Équipe mise à jour.".into()));},
                                                            on_error:move |error|notice.set(Some(error))
                                                        }
                                                    }
                                                }
                                                if teams_list.is_empty(){
                                                    p{class:"panel-empty","Aucune équipe pour ce club."}
                                                }
                                            }
                                            form{class:"workspace-form admin-create-team",
                                                onsubmit:{
                                                    let token=token.clone();
                                                    move |event|{
                                                        event.prevent_default();
                                                        let label=new_team.read().trim().to_string();
                                                        let id=club.read().clone();
                                                        if label.is_empty()||id.is_empty()||*busy.read(){return;}
                                                        let token=token.clone();
                                                        busy.set(true);
                                                        spawn(async move{
                                                            match org::create_team(&token,&id,&label).await{
                                                                Ok(())=>{
                                                                    new_team.set(String::new());
                                                                    teams.restart();
                                                                    notice.set(Some("Équipe créée.".into()));
                                                                },
                                                                Err(error)=>notice.set(Some(error))
                                                            }
                                                            busy.set(false);
                                                        });
                                                    }
                                                },
                                                div{class:"field",label{"Nouvelle équipe"},
                                                    input{value:"{new_team}",required:true,maxlength:"120",
                                                        oninput:move |e|new_team.set(e.value())}}
                                                button{class:"button solid",r#type:"submit",disabled:*busy.read(),"Ajouter"}
                                            }
                                        }
                                    }else if *club_section.read()=="roster"{
                                        div{class:"admin-roster-heading",
                                            button{class:"button outline small",onclick:move |_|{
                                                team.set(String::new());club_section.set("teams".into());
                                            },"Retour aux équipes"}
                                            h2{"Effectif de {teams_list.iter().find(|t|t.id==*team.read()).map(|t|t.name.clone()).unwrap_or_default()}"}
                                        }
                                        div{class:"club-roster-grid",
                                            section{class:"panel",
                                                div{class:"panel-head",h2{"Joueuses de l'équipe"}}
                                                match roster.read().clone(){
                                                    Some(Ok(people))=>rsx!{
                                                        if people.is_empty(){p{class:"panel-empty","Effectif vide."}}
                                                        for player in people{
                                                            div{class:"workspace-roster-item",key:"{player.id}",
                                                                span{class:"roster-number","{player.number}"}
                                                                strong{"{player.firstname} {player.lastname}"}
                                                                button{class:"remove-link",disabled:*busy.read(),
                                                                    onclick:{
                                                                        let pid=player.id.clone();
                                                                        let token=token.clone();
                                                                        move |_|{
                                                                            let id=team.read().clone();
                                                                            let token=token.clone();
                                                                            let pid=pid.clone();
                                                                            busy.set(true);
                                                                            spawn(async move{
                                                                                match org::remove_team_player(&token,&id,&pid).await{
                                                                                    Ok(())=>{roster.restart();notice.set(Some("Joueuse retirée.".into()));},
                                                                                    Err(error)=>notice.set(Some(error))
                                                                                }
                                                                                busy.set(false);
                                                                            });
                                                                        }
                                                                    },"Retirer"}
                                                            }
                                                        }
                                                    },
                                                    Some(Err(error))=>rsx!{p{class:"panel-empty","{error}"}},
                                                    None=>rsx!{p{class:"panel-empty","Chargement…"}}
                                                }
                                            }
                                            section{class:"panel workspace-secondary",
                                                div{class:"panel-head",h2{"Ajouter des joueuses"}}
                                                CreateTeamPlayer{
                                                    key:"{team}",
                                                    token:token.clone(),team_id:team.read().clone(),
                                                    on_saved:move |_|{roster.restart();club_candidates.restart();notice.set(Some("Joueuse créée.".into()));},
                                                    on_error:move |error|notice.set(Some(error))
                                                }
                                                form{class:"workspace-form",
                                                    onsubmit:{
                                                        let token=token.clone();
                                                        move |event|{
                                                            event.prevent_default();
                                                            let pid=roster_player.read().clone();
                                                            let tid=team.read().clone();
                                                            if pid.is_empty()||tid.is_empty()||*busy.read(){return;}
                                                            let token=token.clone();
                                                            busy.set(true);
                                                            spawn(async move{
                                                                match org::add_team_player(&token,&tid,&pid).await{
                                                                    Ok(())=>{
                                                                        roster.restart();
                                                                        roster_player.set(String::new());
                                                                        notice.set(Some("Joueuse ajoutée à l'effectif.".into()));
                                                                    },
                                                                    Err(error)=>notice.set(Some(error))
                                                                }
                                                                busy.set(false);
                                                            });
                                                        }
                                                    },
                                                    div{class:"field",label{"Joueuse déjà enregistrée"},
                                                        select{value:"{roster_player}",required:true,
                                                            onchange:move |e|roster_player.set(e.value()),
                                                            option{value:"","Choisir une joueuse"}
                                                            if let Some(Ok(people))=club_candidates.read().clone(){
                                                                for player in people{
                                                                    option{key:"{player.id}",value:"{player.id}",
                                                                        "{player.firstname} {player.lastname} · n°{player.number}"}
                                                                }
                                                            }
                                                        }
                                                    }
                                                    button{class:"button outline",r#type:"submit",
                                                        disabled:*busy.read(),"Ajouter à l'effectif"}
                                                }
                                            }
                                        }
                                    }else if *club_section.read()=="access"{
                                        nav{class:"workspace-subtabs",
                                            button{class:if team.read().is_empty(){"selected"}else{""},
                                                onclick:move |_|team.set(String::new()),"Club entier"}
                                            for entry in &teams_list{
                                                button{key:"{entry.id}",
                                                    class:if *team.read()==entry.id{"selected"}else{""},
                                                    onclick:{
                                                        let id=entry.id.clone();
                                                        move |_|team.set(id.clone())
                                                    },"{entry.name}"}
                                            }
                                        }
                                        MembershipPanel{key:"{group_id}",token:token.clone(),club_id:club.read().clone(),
                                            scope:scope.to_string(),id:group_id,
                                            can_manage:true,default_user:None}
                                    }
                                }else{
                                    div{class:"workspace-empty",
                                        h2{"Choisissez un club"}
                                        p{"Sélectionnez un club dans la liste pour gérer ses informations, équipes et accès."}}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
