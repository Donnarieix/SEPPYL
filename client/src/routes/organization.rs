use dioxus::prelude::*;
use crate::api::{account,organization as org,players};
use super::{Route,layout::Header};

#[component]
pub fn Organization() -> Element {
    let session=use_context::<Signal<Option<String>>>();
    let navigator=use_navigator();
    let token=session.read().clone();
    let fetch_token=token.clone();
    let mut clubs=use_resource(move ||{
        let token=fetch_token.clone();
        async move {
            match token{Some(token)=>org::clubs(&token).await,None=>Err("Connexion requise.".into())}
        }
    });
    let admin_token=token.clone();
    let mut users=use_resource(move ||{
        let token=admin_token.clone();
        async move {
            match token{Some(token)=>org::users(&token).await,None=>Err("Connexion requise.".into())}
        }
    });
    let self_token=token.clone();
    let account_info=use_resource(move ||{
        let token=self_token.clone();
        async move{
            match token{Some(token)=>account::get(&token).await,None=>Err("Connexion requise.".into())}
        }
    });
    let mut selected_club=use_signal(String::new);
    let teams_token=token.clone();
    let mut teams=use_resource(move ||{
        let token=teams_token.clone();
        let id=selected_club.read().clone();
        async move{
            match(token,id.as_str()){
                (Some(token),id) if !id.is_empty()=>org::teams(&token,id).await,
                _=>Ok(Vec::new()),
            }
        }
    });
    let mut new_club=use_signal(String::new);
    let mut new_team=use_signal(String::new);
    let mut member_user_id=use_signal(String::new);
    let mut member_team_id=use_signal(String::new);
    let mut member_scope=use_signal(||"team".to_string());
    let mut member_role=use_signal(||"trainer".to_string());
    let mut global_user_id=use_signal(String::new);
    let mut global_role=use_signal(||"user".to_string());
    let roster_token=token.clone();
    let mut roster=use_resource(move ||{
        let token=roster_token.clone();
        let id=member_team_id.read().clone();
        async move{
            if id.is_empty(){Ok(Vec::new())}else{
                match token {Some(token)=>org::team_players(&token,&id).await,None=>Err("Connexion requise.".into())}
            }
        }
    });
    let owned_token=token.clone();
    let own_players=use_resource(move ||{
        let token=owned_token.clone();
        async move{
            match token{Some(token)=>players::list(&token).await,None=>Err("Connexion requise.".into())}
        }
    });
    let mut selected_player=use_signal(String::new);
    let mut notice=use_signal(||None::<String>);
    let mut busy=use_signal(||false);
    if token.is_none(){
        return rsx!{div{class:"centered",
            p{"Connectez-vous pour accéder aux clubs."}
            button{class:"button solid",onclick:move |_|{navigator.push(Route::Login{});},"Connexion"}
        }};
    }
    let token=token.unwrap_or_default();
    let create_token=token.clone();
    let team_token=token.clone();
    let member_token=token.clone();
    let role_token=token.clone();
    let is_admin=matches!(account_info.read().as_ref(),Some(Ok(account)) if account.role=="administrator");

    rsx!{
        div{class:"app",
            Header{}
            main{class:"page organization-page",
                div{class:"page-heading",
                    div{
                        h1{"Clubs et équipes"}
                        p{"Organisation des accès et affectation des entraîneurs."}
                    }
                }
                if let Some(message)=notice.read().as_ref(){
                    div{class:"inline-notice",role:"status","{message}"}
                }
                div{class:"organization-layout",
                    section{class:"panel",
                        div{class:"panel-head",
                            div{h2{"Mes clubs"}p{"Les clubs visibles selon vos rattachements."}}
                        }
                        match clubs.read().clone(){
                            Some(Ok(list))=>rsx!{
                                if list.is_empty(){div{class:"panel-empty","Aucun club accessible."}}
                                for club in list {
                                    button{
                                        key:"{club.id}",
                                        class:if *selected_club.read()==club.id{"organization-item active"}else{"organization-item"},
                                        onclick:move |_|{
                                            selected_club.set(club.id.clone());
                                            member_team_id.set(String::new());
                                        },
                                        "{club.name}"
                                    }
                                }
                            },
                            Some(Err(error))=>rsx!{p{class:"panel-empty","{error}"}},
                            None=>rsx!{p{class:"panel-empty","Chargement…"}},
                        }
                        if !selected_club.read().is_empty() {
                            div { class:"organization-form",
                                span { "Annuaire public : désactivé par défaut." }
                                button { class:"button outline small", disabled:*busy.read(),
                                    onclick:{
                                        let token=token.clone();
                                        move |_|{
                                            let id=selected_club.read().clone();
                                            if id.is_empty()||*busy.read(){return;}
                                            let is_public=matches!(clubs.read().as_ref(),Some(Ok(items)) if items.iter().find(|c|c.id==id).and_then(|c|c.is_public)==Some(true));
                                            let token=token.clone();
                                            busy.set(true);
                                            spawn(async move {
                                                match org::set_public(&token,&id,!is_public).await {
                                                    Ok(())=>{
                                                        clubs.restart();
                                                        notice.set(Some(if is_public{"Club rendu privé.".into()}else{"Club rendu public.".into()}));
                                                    }
                                                    Err(message)=>notice.set(Some(message)),
                                                }
                                                busy.set(false);
                                            });
                                        }
                                    },
                                    if matches!(clubs.read().as_ref(),Some(Ok(items)) if items.iter().any(|c|c.id==*selected_club.read() && c.is_public==Some(true))) {
                                        "Rendre privé"
                                    } else { "Rendre public" }
                                }
                            }
                        }
                        if is_admin{
                            form{class:"organization-form",
                                onsubmit:move |event|{
                                    event.prevent_default();
                                    let label=new_club.read().trim().to_string();
                                    if label.is_empty() || *busy.read(){return;}
                                    let token=create_token.clone();
                                    busy.set(true);
                                    spawn(async move{
                                        match org::create_club(&token,&label).await{
                                            Ok(())=>{new_club.set(String::new());clubs.restart();notice.set(Some("Club créé.".into()));}
                                            Err(error)=>notice.set(Some(error)),
                                        }
                                        busy.set(false);
                                    });
                                },
                                label{"Créer un club"}
                                input{value:"{new_club}",required:true,maxlength:"120",
                                    placeholder:"Nom du club",oninput:move |e|new_club.set(e.value())}
                                button{class:"button solid small",r#type:"submit",disabled:*busy.read(),"Ajouter"}
                            }
                        }
                    }
                    section{class:"panel",
                        div{class:"panel-head",
                            div{h2{"Équipes"}p{"Sélectionnez un club pour voir ses équipes."}}
                        }
                        if selected_club.read().is_empty(){
                            div{class:"panel-empty","Choisissez un club."}
                        }else{
                            match teams.read().clone(){
                                Some(Ok(items))=>rsx!{
                                    if items.is_empty(){div{class:"panel-empty","Aucune équipe disponible."}}
                                    for team in items{
                                        button{
                                            key:"{team.id}",
                                            class:if *member_team_id.read()==team.id{"organization-item active"}else{"organization-item"},
                                            onclick:move |_|member_team_id.set(team.id.clone()),
                                            "{team.name}"
                                        }
                                    }
                                },
                                Some(Err(error))=>rsx!{p{class:"panel-empty","{error}"}},
                                None=>rsx!{p{class:"panel-empty","Chargement…"}},
                            }
                            form{class:"organization-form",
                                onsubmit:move |event|{
                                    event.prevent_default();
                                    let label=new_team.read().trim().to_string();
                                    let club_id=selected_club.read().clone();
                                    if label.is_empty()||club_id.is_empty()||*busy.read(){return;}
                                    let token=team_token.clone();
                                    busy.set(true);
                                    spawn(async move{
                                        match org::create_team(&token,&club_id,&label).await{
                                            Ok(())=>{new_team.set(String::new());teams.restart();notice.set(Some("Équipe créée.".into()));}
                                            Err(error)=>notice.set(Some(error)),
                                        }
                                        busy.set(false);
                                    });
                                },
                                label{"Créer une équipe (gestionnaire)"}
                                input{value:"{new_team}",required:true,maxlength:"120",
                                    placeholder:"Nom de l'équipe",oninput:move |e|new_team.set(e.value())}
                                button{class:"button solid small",r#type:"submit",disabled:*busy.read(),"Ajouter"}
                            }
                        }
                    }
                }
                if !member_team_id.read().is_empty() {
                    section { class:"panel organization-members",
                        div { class:"panel-head",
                            div { h2 { "Joueuses de l'équipe" }
                                p { "Effectif partagé entre les entraîneurs rattachés à cette équipe." } }
                        }
                        match roster.read().clone() {
                            Some(Ok(people)) => rsx! {
                                if people.is_empty(){div{class:"panel-empty","Aucune joueuse rattachée."}}
                                for player in people {
                                    div { class:"organization-user", key:"{player.id}",
                                        span { "{player.firstname} {player.lastname}" }
                                        strong { "#{player.number}" }
                                        button { class:"button quiet small", disabled:*busy.read(),
                                            onclick:{
                                                let player_id=player.id.clone();
                                                let token=token.clone();
                                                move |_|{
                                                    let team=member_team_id.read().clone();
                                                    let token=token.clone();
                                                    let id=player_id.clone();
                                                    busy.set(true);
                                                    spawn(async move {
                                                        match org::remove_team_player(&token,&team,&id).await {
                                                            Ok(())=>{roster.restart();notice.set(Some("Joueuse retirée de l'équipe.".into()));},
                                                            Err(message)=>notice.set(Some(message)),
                                                        }
                                                        busy.set(false);
                                                    });
                                                }
                                            },
                                            "Retirer"
                                        }
                                    }
                                }
                            },
                            Some(Err(message)) => rsx! { p { class:"panel-empty", "{message}" } },
                            None => rsx! { p { class:"panel-empty", "Chargement…" } },
                        }
                        form { class:"organization-form",
                            onsubmit:{
                                let token=token.clone();
                                move |event|{
                                    event.prevent_default();
                                    let player_id=selected_player.read().clone();
                                    let team=member_team_id.read().clone();
                                    if player_id.is_empty()||team.is_empty()||*busy.read(){return;}
                                    let token=token.clone();
                                    busy.set(true);
                                    spawn(async move {
                                        match org::add_team_player(&token,&team,&player_id).await {
                                            Ok(())=>{
                                                roster.restart();
                                                selected_player.set(String::new());
                                                notice.set(Some("Joueuse rattachée.".into()));
                                            }
                                            Err(message)=>notice.set(Some(message)),
                                        }
                                        busy.set(false);
                                    });
                                }
                            },
                            label { "Ajouter une de mes joueuses à l'équipe" }
                            select { value:"{selected_player}",onchange:move |e|selected_player.set(e.value()),
                                option { value:"", "Sélectionner…" }
                                if let Some(Ok(people))=own_players.read().clone(){
                                    for player in people {
                                        option { value:"{player.id}", "{player.firstname} {player.lastname} · #{player.number}" }
                                    }
                                }
                            }
                            button { r#type:"submit", class:"button outline small", disabled:*busy.read(), "Rattacher" }
                        }
                    }
                }
                section{class:"panel organization-members",
                    div{class:"panel-head",
                        div{
                            h2{"Rattachements et autorisations"}
                            p{"La gestion des droits est réservée aux gestionnaires de club et aux administrateurs."}
                        }
                    }
                    form{class:"organization-form organization-grant",
                        onsubmit:move |event|{
                            event.prevent_default();
                            let token=member_token.clone();
                            let scope=member_scope.read().clone();
                            let id=if scope=="club"{selected_club.read().clone()}else{member_team_id.read().clone()};
                            let user=member_user_id.read().trim().to_string();
                            let role=member_role.read().clone();
                            if id.is_empty()||user.is_empty()||*busy.read(){
                                notice.set(Some("Sélectionnez un club ou une équipe et un utilisateur.".into()));
                                return;
                            }
                            busy.set(true);
                            spawn(async move{
                                let result=if scope=="club"{org::grant_club(&token,&id,&user,&role).await}
                                    else{org::grant_team(&token,&id,&user,&role).await};
                                match result{
                                    Ok(())=>notice.set(Some("Rattachement enregistré.".into())),
                                    Err(error)=>notice.set(Some(error)),
                                }
                                busy.set(false);
                            });
                        },
                        div{class:"field",
                            label{"Niveau"}
                            select{value:"{member_scope}",onchange:move |e|{
                                let scope=e.value();
                                member_role.set(if scope=="club"{"user".into()}else{"trainer".into()});
                                member_scope.set(scope);
                            },
                                option{value:"team","Équipe"} option{value:"club","Club"}
                            }
                        }
                        div{class:"field",
                            label{"Identifiant de l'utilisateur (UUID)"}
                            input{value:"{member_user_id}",required:true,
                                placeholder:"UUID du compte",oninput:move |e|member_user_id.set(e.value())}
                        }
                        div{class:"field",
                            label{"Droit"}
                            if *member_scope.read()=="club"{
                                select{value:"{member_role}",onchange:move |e|member_role.set(e.value()),
                                    option{value:"user","Utilisateur · lecture"}
                                    option{value:"manager","Gestionnaire · écriture club"}
                                }
                            }else{
                                select{value:"{member_role}",onchange:move |e|member_role.set(e.value()),
                                    option{value:"user","Utilisateur · lecture"}
                                    option{value:"trainer","Entraîneur · écriture équipe"}
                                }
                            }
                        }
                        button{class:"button solid small",r#type:"submit",disabled:*busy.read(),"Enregistrer les droits"}
                    }
                    p{class:"organization-note","Le rattachement seul ne rend pas publiques les données ; l'autorisation est vérifiée côté API."}
                }
                if is_admin{
                    section{class:"panel organization-members",
                        div{class:"panel-head",
                            div{h2{"Administration des comptes"}p{"Rôles globaux et identifiants des comptes inscrits."}}
                        }
                        match users.read().clone(){
                            Some(Ok(list))=>rsx!{
                                div{class:"organization-users",
                                    for item in list{
                                        div{class:"organization-user",key:"{item.id}",
                                            span{"{item.username}"}
                                            code{"{item.id}"}
                                            strong{"{item.role}"}
                                            button{class:"button quiet small",
                                                onclick:move |_|global_user_id.set(item.id.clone()),
                                                "Sélectionner"}
                                        }
                                    }
                                }
                            },
                            Some(Err(error))=>rsx!{p{class:"panel-empty","{error}"}},
                            None=>rsx!{p{class:"panel-empty","Chargement…"}},
                        }
                        form{class:"organization-form organization-grant",
                            onsubmit:move |event|{
                                event.prevent_default();
                                let target=global_user_id.read().clone();
                                let role=global_role.read().clone();
                                if target.is_empty()||*busy.read(){return;}
                                let token=role_token.clone();
                                busy.set(true);
                                spawn(async move{
                                    match org::assign_global_role(&token,&target,&role).await{
                                        Ok(())=>{users.restart();notice.set(Some("Rôle global modifié.".into()));}
                                        Err(error)=>notice.set(Some(error)),
                                    }
                                    busy.set(false);
                                });
                            },
                            div{class:"field",
                                label{"Compte sélectionné"}
                                input{value:"{global_user_id}",readonly:true}
                            }
                            div{class:"field",
                                label{"Rôle global"}
                                select{value:"{global_role}",onchange:move |e|global_role.set(e.value()),
                                    option{value:"user","Utilisateur"}
                                    option{value:"trainer","Entraîneur"}
                                    option{value:"manager","Gestionnaire"}
                                    option{value:"administrator","Administrateur"}
                                }
                            }
                            button{class:"button solid small",r#type:"submit",disabled:*busy.read(),"Modifier le rôle"}
                        }
                    }
                }
            }
        }
    }
}
