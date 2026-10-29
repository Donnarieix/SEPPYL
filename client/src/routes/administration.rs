use dioxus::prelude::*;
use crate::api::{account,organization as org};
use super::{Route,layout::Header,membership::MembershipPanel};

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
        async move{match token{Some(token)=>org::clubs(&token).await,None=>Err("Connexion requise.".into())}}
    });
    let mut tab=use_signal(||"users".to_string());
    let mut query=use_signal(String::new);
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
    let matching:Vec<_>=directory.iter().filter(|u|u.username.to_lowercase().contains(&query.read().to_lowercase())).cloned().collect();
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
                            div{class:"workspace-search",
                                input{value:"{query}",placeholder:"Rechercher un utilisateur…",
                                    "aria-label":"Rechercher un utilisateur",
                                    oninput:move |e|query.set(e.value())}
                                span{class:"muted-small","{matching.len()} compte(s)"}
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
                        section{class:"panel",
                            div{class:"panel-head",
                                div{h2{"Clubs de la plateforme"}
                                    p{"Création des clubs. Les équipes et leurs effectifs se gèrent dans l'espace Club."}}
                            }
                            div{class:"workspace-card-grid",
                                for item in &clubs_list{
                                    button{class:"workspace-team-card",key:"{item.id}",
                                        onclick:{
                                            let id=item.id.clone();
                                            move |_|{
                                                club.set(id.clone());team.set(String::new());tab.set("memberships".into());
                                            }
                                        },
                                        span{class:"workspace-team-icon","⚽"}
                                        strong{"{item.name}"}
                                        span{class:"muted-small",if item.is_public==Some(true){"Public"}else{"Privé"}}
                                    }
                                }
                            }
                        }
                        section{class:"panel workspace-secondary",
                            div{class:"panel-head",div{h2{"Créer un club"}p{"Chaque nouveau club est privé par défaut."}}}
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
                                                Ok(())=>{name.set(String::new());clubs.restart();notice.set(Some("Club créé.".into()));}
                                                Err(e)=>notice.set(Some(e)),
                                            }
                                            busy.set(false);
                                        });
                                    }
                                },
                                div{class:"field",label{"Nom du club"},
                                    input{value:"{name}",required:true,maxlength:"120",
                                        placeholder:"Ex. Club sportif",oninput:move |e|name.set(e.value())}}
                                button{class:"button solid",r#type:"submit",disabled:*busy.read(),"Créer"}
                            }
                        }
                    }
                }
            }
        }
    }
}
