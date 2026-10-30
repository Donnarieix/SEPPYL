use dioxus::prelude::*;
use crate::api::organization as org;

/// Reusable member editor for one club or one team.
/// The API enforces permissions; the UI only shows friendly account names.
#[component]
pub fn MembershipPanel(
    token:String, club_id:String, scope:String, id:String,
    can_manage:bool,
    default_user:Option<String>,
) -> Element {
    let directory_token=token.clone();
    let directory_club=club_id.clone();
    let accounts=use_resource(move ||{
        let token=directory_token.clone();
        let club=directory_club.clone();
        async move{org::eligible_users(&token,&club).await}
    });
    let list_token=token.clone();
    let list_id=id.clone();
    let list_scope=scope.clone();
    let mut members=use_resource(move ||{
        let token=list_token.clone();
        let id=list_id.clone();
        let scope=list_scope.clone();
        async move{
            if scope=="club" {
                org::club_members(&token,&id).await.map(|rows|
                    rows.into_iter().map(|m|(m.user_id,m.role)).collect::<Vec<_>>())
            }else{
                org::team_members(&token,&id).await.map(|rows|
                    rows.into_iter().map(|m|(m.user_id,m.role)).collect::<Vec<_>>())
            }
        }
    });
    let mut selected=use_signal(||default_user.unwrap_or_default());
    let mut role=use_signal(||if scope=="club"{"user".to_string()}else{"trainer".to_string()});
    let mut notice=use_signal(||None::<String>);
    let mut busy=use_signal(||false);
    let is_club=scope=="club";
    let accounts_list=accounts.read().clone();
    let names=accounts_list.as_ref().and_then(|v|v.as_ref().ok()).cloned().unwrap_or_default();
    let rows=members.read().clone();
    rsx!{
        section{class:"panel permissions-panel",
            div{class:"panel-head",
                div{
                    h2{if is_club{"Accès au club"}else{"Accès à l'équipe"}}
                    p{"Gérez les membres et leurs autorisations."}
                }
            }
            if let Some(message)=notice.read().as_ref(){
                p{class:"inline-notice",role:"status","{message}"}
            }
            div{class:"permission-table-wrap",
                table{class:"permission-table",
                    thead{tr{th{"Compte"}th{"Autorisation"}if can_manage{th{"Actions"}}}}
                    tbody{
                        match rows{
                            Some(Ok(entries))=>rsx!{
                                if entries.is_empty(){
                                    tr{td{colspan:if can_manage{"3"}else{"2"},"Aucun membre rattaché."}}
                                }
                                for (user_id,user_role) in entries{
                                    tr{key:"{user_id}",
                                        td{
                                            strong{
                                                "{names.iter().find(|u|u.id==user_id).map(|u|u.username.as_str()).unwrap_or(\"Compte inconnu\")}"
                                            }
                                        }
                                        td{span{class:"role-pill","{user_role}"}}
                                        if can_manage{
                                            td{
                                                button{
                                                    r#type:"button",class:"remove-link",
                                                    disabled:*busy.read(),
                                                    onclick:{
                                                        let token=token.clone();
                                                        let scope=scope.clone();
                                                        let id=id.clone();
                                                        let uid=user_id.clone();
                                                        move |_|{
                                                            let token=token.clone();
                                                            let scope=scope.clone();
                                                            let id=id.clone();
                                                            let uid=uid.clone();
                                                            busy.set(true);
                                                            spawn(async move{
                                                                match org::remove_member(&token,&scope,&id,&uid).await{
                                                                    Ok(())=>{members.restart();notice.set(Some("Accès retiré.".into()));}
                                                                    Err(e)=>notice.set(Some(e)),
                                                                }
                                                                busy.set(false);
                                                            });
                                                        }
                                                    },"Retirer"
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Some(Err(error))=>rsx!{tr{td{colspan:"3","{error}"}}},
                            None=>rsx!{tr{td{colspan:"3","Chargement…"}}},
                        }
                    }
                }
            }
            if can_manage{
                div{class:"permission-editor",
                    div{class:"permission-editor-title",
                        h3{"Ajouter ou modifier un accès"}
                        p{"Sélectionnez un utilisateur et définissez ses droits."}
                    }
                    form{
                        onsubmit:{
                            let token=token.clone();
                            let scope=scope.clone();
                            let id=id.clone();
                            move |event|{
                                event.prevent_default();
                                let account=selected.read().clone();
                                let chosen_role=role.read().clone();
                                if account.is_empty()||*busy.read(){return;}
                                let token=token.clone();
                                let scope=scope.clone();
                                let id=id.clone();
                                busy.set(true);
                                spawn(async move{
                                    let result=if scope=="club"{
                                        org::grant_club(&token,&id,&account,&chosen_role).await
                                    }else{
                                        org::grant_team(&token,&id,&account,&chosen_role).await
                                    };
                                    match result{
                                        Ok(())=>{
                                            notice.set(Some("Autorisation enregistrée.".into()));
                                            members.restart();selected.set(String::new());
                                        }
                                        Err(error)=>notice.set(Some(error)),
                                    }
                                    busy.set(false);
                                });
                            }
                        },
                        div{class:"field",
                            label{"Compte"}
                            select{value:"{selected}",required:true,onchange:move |e|selected.set(e.value()),
                                option{value:"","Choisir un utilisateur"}
                                for item in &names{
                                    option{key:"{item.id}",value:"{item.id}","{item.username}"}
                                }
                            }
                        }
                        div{class:"field",
                            label{"Autorisation"}
                            select{value:"{role}",onchange:move |e|role.set(e.value()),
                                option{value:"user","Lecture seule"}
                                if is_club{
                                    option{value:"manager","Gestionnaire du club"}
                                }else{
                                    option{value:"trainer","Entraîneur"}
                                }
                            }
                        }
                        button{class:"button solid",r#type:"submit",disabled:*busy.read(),
                            if *busy.read(){"Enregistrement…"}else{"Enregistrer l'accès"}
                        }
                    }
                }
            }
        }
    }
}
