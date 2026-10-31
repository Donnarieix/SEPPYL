use dioxus::prelude::*;
use crate::api::players::{self,Player};
use crate::api::organization as org;
use super::{Route,layout::Header};

#[component]
pub fn Players() -> Element {
    let session=use_context::<Signal<Option<String>>>();
    let nav=use_navigator();
    let token=session.read().clone();
    let fetch=token.clone();
    let mut data=use_resource(move||{let token=fetch.clone();async move {
        match token {Some(t)=>players::list(&t).await,None=>Err("Connexion requise.".into())}
    }});
    let scope_token=token.clone();
    let allowed_teams=use_resource(move ||{
        let token=scope_token.clone();
        async move{
            let Some(token)=token else{return Ok::<Vec<(String,String)>,String>(Vec::new());};
            let mut choices=Vec::new();
            for club in org::clubs(&token).await?{
                let caps=org::capabilities(&token,&club.id).await?;
                for team in org::teams(&token,&club.id).await?{
                    if caps.can_write_teams.contains(&team.id){
                        choices.push((team.id,format!("{} · {}",club.name,team.name)));
                    }
                }
            }
            Ok::<Vec<(String,String)>,String>(choices)
        }
    });
    let mut adding=use_signal(||false);
    let mut error=use_signal(||None::<String>);
    if token.is_none(){return rsx!{div{class:"centered",p{"Connectez-vous."}
        button{class:"button solid",onclick:move |_|{nav.push(Route::Login{});},"Connexion"}}};}
    let token=token.unwrap_or_default();
    rsx!{
        div{class:"app",
            Header{}
            main{class:"page",
                div{class:"page-heading",
                    div{h1{"Joueuses"} p{"Retrouvez les joueuses de vos équipes et les fiches de votre espace personnel."}}
                    div{class:"heading-actions",
                        Link{to:Route::Organization{},class:"button outline","Effectifs par équipe"}
                        button{class:"button solid",onclick:move |_|adding.set(true),"+ Ajouter une joueuse"}
                    }
                }
                if let Some(message)=error.read().as_ref(){p{class:"form-error",role:"alert","{message}"}}
                div{class:"sheet players-sheet",
                    div{class:"sheet-toolbar",strong{"Joueuses visibles"},span{class:"muted-small","Les équipes contrôlent le partage entre entraîneurs."}}
                    match data.read().clone(){
                        Some(Ok(list))=>rsx!{
                            if list.is_empty(){
                                div{class:"empty-state",p{"Aucune joueuse pour le moment."}}
                            }else{
                                table{class:"roster-table",
                                    thead{tr{th{"Prénom"} th{"Nom"} th{"Numéro"} th{"Actions"}}}
                                    tbody{
                                        for player in list{
                                            PlayerRow{key:"{player.id}",player,token:token.clone(),
                                                on_saved:move |_|data.restart(),
                                                on_error:move |message|error.set(Some(message))}
                                        }
                                    }
                                }
                            }
                        },
                        Some(Err(e))=>rsx!{div{class:"load-state",p{"{e}"} button{class:"button outline",onclick:move |_|data.restart(),"Réessayer"}}},
                        None=>rsx!{div{class:"load-state","Chargement…"}},
                    }
                }
            }
            if *adding.read(){
                PlayerEditor{token:token.clone(),player:None,
                    teams:allowed_teams.read().as_ref().and_then(|v|v.as_ref().ok()).cloned().unwrap_or_default(),
                    on_cancel:move |_|adding.set(false),
                    on_saved:move |_|{adding.set(false);data.restart();},
                    on_error:move |message|error.set(Some(message))}
            }
        }
    }
}

#[component]
fn PlayerRow(player:Player,token:String,on_saved:EventHandler<()>,on_error:EventHandler<String>)->Element{
    let mut editing=use_signal(||false);
    let mut confirming=use_signal(||false);
    let mut showing_times=use_signal(||false);
    let mut busy=use_signal(||false);
    rsx!{
        tr{
            td{"{player.firstname}"}
            td{"{player.lastname}"}
            td{"{player.number}"}
            td{class:"roster-actions",
                button{class:"button outline small",onclick:move |_|editing.set(true),"Modifier"}
                button{class:"button quiet small",r#type:"button",
                    onclick:move |_|{let visible=*showing_times.read();showing_times.set(!visible);},
                    "Temps de jeu"}
                if *showing_times.read(){
                    PlayerTimes{token:token.clone(),player_id:player.id.clone()}
                }
                if *confirming.read(){
                    button{class:"button danger small",disabled:*busy.read(),
                        onclick:{
                            let token=token.clone();let id=player.id.clone();
                            move |_|{
                                let token=token.clone();let id=id.clone();
                                busy.set(true);
                                spawn(async move{
                                    match players::delete(&token,&id).await{
                                        Ok(())=>on_saved.call(()),
                                        Err(e)=>on_error.call(e),
                                    }
                                    busy.set(false);confirming.set(false);
                                });
                            }
                        },
                        "Confirmer"
                    }
                    button{class:"button quiet small",onclick:move |_|confirming.set(false),"Annuler"}
                }else{
                    button{class:"button quiet small",onclick:move |_|confirming.set(true),"Supprimer"}
                }
            }
        }
        if *editing.read(){
            PlayerEditor{token:token.clone(),player:Some(player.clone()),teams:Vec::new(),
                on_cancel:move |_|editing.set(false),
                on_saved:move |_|{editing.set(false);on_saved.call(());},
                on_error}
        }
    }
}

#[component]
fn PlayerTimes(token:String,player_id:String)->Element{
    let summary=use_resource(move ||{let t=token.clone();let id=player_id.clone();
        async move{players::time_totals(&t,&id).await}});
    rsx!{
        div{class:"player-time-totals",
            match summary.read().clone(){
                Some(Ok(t))=>rsx!{
                    p{"Entraînements : {t.training_minutes} min ({t.training_attended} présences)"}
                    p{"Matchs : {t.match_minutes} min ({t.matches_played} joués)"}
                    if t.trainings_without_duration>0{
                        small{"{t.trainings_without_duration} entraînement(s) sans durée renseignée"}
                    }
                },
                Some(Err(e))=>rsx!{small{"{e}"}},
                None=>rsx!{small{"Calcul…"}}
            }
        }
    }
}

#[component]
fn PlayerEditor(
    token:String,player:Option<Player>,teams:Vec<(String,String)>,
    on_cancel:EventHandler<()>,on_saved:EventHandler<()>,on_error:EventHandler<String>
)->Element{
    let mut firstname=use_signal(||player.as_ref().map(|p|p.firstname.clone()).unwrap_or_default());
    let mut lastname=use_signal(||player.as_ref().map(|p|p.lastname.clone()).unwrap_or_default());
    let mut number=use_signal(||player.as_ref().map(|p|p.number.to_string()).unwrap_or_default());
    let mut busy=use_signal(||false);
    let mut error=use_signal(||None::<String>);
    let mut team_target=use_signal(String::new);
    let editing=player.is_some();
    let id=player.as_ref().map(|p|p.id.clone());
    rsx!{
        div{class:"dialog-backdrop",
            div{class:"dialog-panel",role:"dialog","aria-modal":"true",
                div{class:"dialog-header",div{h2{if editing{"Modifier une joueuse"}else{"Ajouter une joueuse"}}}
                    button{class:"icon-button",onclick:move |_|on_cancel.call(()),"×"}
                }
                form{
                    onsubmit:move |e|{
                        e.prevent_default();
                        if *busy.read(){return;}
                        let first=firstname.read().trim().to_string();
                        let last=lastname.read().trim().to_string();
                        let parsed=number.read().parse::<u32>();
                        if first.is_empty()||last.is_empty()||parsed.is_err(){
                            error.set(Some("Renseignez un prénom, un nom et un numéro valide.".into()));return;
                        }
                        let selected=team_target.read().clone();
                        if id.is_none()&&!teams.is_empty()&&selected.is_empty(){
                            error.set(Some("Sélectionnez l'équipe de la joueuse.".into()));return;
                        }
                        let token=token.clone();let id=id.clone();let num=parsed.unwrap();
                        busy.set(true);
                        spawn(async move{
                            let result=if id.is_none()&&!selected.is_empty(){
                                org::create_team_player(&token,&selected,&first,&last,num).await
                            }else{
                                players::save(&token,id.as_deref(),&first,&last,num).await
                            };
                            match result{
                                Ok(())=>on_saved.call(()),
                                Err(e)=>{error.set(Some(e.clone()));on_error.call(e);}
                            }
                            busy.set(false);
                        });
                    },
                    div{class:"dialog-body",
                        div{class:"field-grid",
                            div{class:"field",label{"Prénom"}input{value:"{firstname}",required:true,oninput:move |e|firstname.set(e.value())}}
                            div{class:"field",label{"Nom"}input{value:"{lastname}",required:true,oninput:move |e|lastname.set(e.value())}}
                        }
                        div{class:"field",label{"Numéro"}
                            input{r#type:"number",min:"0",max:"999",value:"{number}",required:true,oninput:move |e|number.set(e.value())}
                        }
                        if !editing{
                            if teams.is_empty(){
                                p{class:"workspace-hint","Aucune équipe modifiable. La fiche restera personnelle jusqu’à son rattachement à une équipe."}
                            }else{
                                div{class:"field",
                                    label{"Équipe"}
                                    select{value:"{team_target}",required:true,onchange:move |e|team_target.set(e.value()),
                                        option{value:"","Choisir une équipe"}
                                        for (id,label) in &teams{
                                            option{key:"{id}",value:"{id}","{label}"}
                                        }
                                    }
                                    small{"La joueuse sera visible par les membres autorisés de cette équipe."}
                                }
                            }
                        }
                        if let Some(message)=error.read().as_ref(){p{class:"form-error",role:"alert","{message}"}}
                    }
                    div{class:"dialog-footer",
                        button{class:"button quiet",r#type:"button",onclick:move |_|on_cancel.call(()),"Annuler"}
                        button{class:"button solid",r#type:"submit",disabled:*busy.read(),
                            if *busy.read(){"Enregistrement…"}else{"Enregistrer"}}
                    }
                }
            }
        }
    }
}
