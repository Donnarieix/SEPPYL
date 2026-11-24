use dioxus::prelude::*;
use crate::api::{matches,organization};
use super::{Route,layout::Header};
#[component]
pub fn Matches()->Element{
    let token=use_context::<Signal<Option<String>>>().read().clone().unwrap_or_default();
    let t=token.clone();
    let mut list=use_resource(move ||{let t=t.clone();async move{matches::list(&t).await}});
    let teams_token=token.clone();
    let teams=use_resource(move ||{
        let t=teams_token.clone();
        async move{
            let mut all=Vec::new();
            for club in organization::clubs(&t).await?{
                for team in organization::teams(&t,&club.id).await?{
                    all.push((team.id,format!("{} · {}",club.name,team.name)));
                }
            }
            Ok::<Vec<(String,String)>,String>(all)
        }
    });
    let navigator=use_navigator();
    let mut adding=use_signal(||false);
    let mut opponent=use_signal(String::new);
    let mut date=use_signal(String::new);
    let mut team_id=use_signal(String::new);
    let mut half=use_signal(||"45".to_owned());
    let mut busy=use_signal(||false);
    let mut error=use_signal(||None::<String>);
    rsx!{
        div{class:"app",Header{},
            main{class:"page workspace-page",
                div{class:"page-heading",
                    div{h1{"Matchs"}p{"Compositions, événements et temps de jeu des joueuses."}}
                    button{class:"button solid",onclick:move |_|adding.set(true),"+ Nouveau match"}
                }
                if let Some(e)=error.read().as_ref(){p{class:"form-error",role:"alert","{e}"}}
                section{class:"panel",
                    match list.read().clone(){
                        Some(Ok(items))=>rsx!{
                            if items.is_empty(){p{class:"muted-small","Aucun match enregistré."}}
                            div{class:"catalog-grid",
                                for game in items{
                                    button{class:"catalog-card match-card",key:"{game.id}",
                                        onclick:{let id=game.id.clone();move |_|{navigator.push(Route::MatchDetail{id:id.clone()});}},
                                        strong{"{game.opponent}"}
                                        span{"{game.date.get(..10).unwrap_or(&game.date)}"}
                                        span{class:"muted-small","{game.half_minutes} min / mi-temps"}
                                    }
                                }
                            }
                        },
                        Some(Err(e))=>rsx!{p{class:"form-error","{e}"}},
                        None=>rsx!{p{"Chargement…"}}
                    }
                }
                if *adding.read(){
                    div{class:"dialog-backdrop",
                        div{class:"dialog-panel",role:"dialog","aria-modal":"true",
                            div{class:"dialog-header",h2{"Nouveau match"}
                                button{class:"icon-button",onclick:move |_|adding.set(false),"×"}}
                            form{onsubmit:{
                                let t=token.clone();
                                move |event|{
                                    event.prevent_default();if *busy.read(){return;}
                                    busy.set(true);
                                    let t=t.clone();
                                    let team=team_id.read().clone();
                                    let other=opponent.read().clone();
                                    let day=date.read().clone();
                                    let minutes=half.read().parse::<i32>().unwrap_or(0);
                                    spawn(async move{
                                        match matches::create(&t,&team,&other,&day,minutes).await{
                                            Ok(game)=>{adding.set(false);list.restart();navigator.push(Route::MatchDetail{id:game.id});},
                                            Err(e)=>error.set(Some(e)),
                                        }
                                        busy.set(false);
                                    });
                                }
                            },
                                div{class:"dialog-body",
                                    div{class:"field",label{"Équipe"}
                                        select{required:true,value:"{team_id}",onchange:move |e|team_id.set(e.value()),
                                            option{value:"","Choisir l'équipe"}
                                            if let Some(Ok(options))=teams.read().clone(){
                                                for (id,label) in options{option{value:"{id}","{label}"}}
                                            }
                                        }
                                    }
                                    div{class:"field",label{"Adversaire"}
                                        input{value:"{opponent}",required:true,maxlength:"160",oninput:move |e|opponent.set(e.value())}}
                                    div{class:"field",label{"Date"}
                                        input{r#type:"date",required:true,value:"{date}",oninput:move |e|date.set(e.value())}}
                                    div{class:"field",label{"Durée d'une mi-temps (minutes)"}
                                        input{r#type:"number",min:"1",max:"120",value:"{half}",
                                            oninput:move |e|half.set(e.value())}}
                                }
                                div{class:"dialog-footer",
                                    button{class:"button quiet",r#type:"button",onclick:move |_|adding.set(false),"Annuler"}
                                    button{class:"button solid",r#type:"submit",disabled:*busy.read(),"Créer le match"}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn MatchDetail(id:String)->Element{
    let token=use_context::<Signal<Option<String>>>().read().clone().unwrap_or_default();
    let t=token.clone();let game_id=id.clone();
    let mut data=use_resource(move ||{let t=t.clone();let id=game_id.clone();
        async move{matches::get(&t,&id).await}});
    let mut kind=use_signal(||"goal".to_string());
    let mut minute=use_signal(||"0".to_string());
    let mut actor=use_signal(String::new);
    let mut incoming=use_signal(String::new);
    let mut custom_type=use_signal(String::new);
    let mut new_type=use_signal(String::new);
    let mut notes=use_signal(String::new);
    let mut busy=use_signal(||false);
    let mut notice=use_signal(||None::<String>);
    rsx!{
        div{class:"app",Header{},
            main{class:"page workspace-page match-detail-page",
                Link{to:Route::Matches{},class:"back-link","← Tous les matchs"}
                if let Some(e)=notice.read().as_ref(){p{class:"inline-notice",role:"alert","{e}"}}
                match data.read().clone(){
                    Some(Ok(detail))=>rsx!{
                        div{class:"page-heading",
                            div{h1{"Match contre {detail.game.opponent}"}
                                p{"{detail.game.date.get(..10).unwrap_or(&detail.game.date)} · {detail.game.half_minutes} minutes par mi-temps"}
                            }
                        }
                        MatchLineup{token:token.clone(),game_id:id.clone(),team_id:detail.game.team_id.clone(),
                            initial:detail.participants.clone(),on_saved:move |_|data.restart()}
                        section{class:"panel",
                            div{class:"panel-head",h2{"Événements du match"}}
                            if detail.events.is_empty(){p{class:"muted-small","Aucun événement enregistré."}}
                            for evt in &detail.events{
                                div{class:"ticket-row",key:"{evt.id}",
                                    strong{"{evt.minute}′ — {evt.kind}"}
                                    span{"{evt.notes}"}
                                    button{class:"button quiet small",r#type:"button",
                                        onclick:{
                                            let t=token.clone();let id=id.clone();let ev=evt.id.clone();
                                            move |_|{
                                                let t=t.clone();let id=id.clone();let ev=ev.clone();
                                                spawn(async move{
                                                    match matches::remove_event(&t,&id,&ev).await{
                                                        Ok(())=>data.restart(),Err(e)=>notice.set(Some(e))
                                                    }
                                                });
                                            }
                                        },"Retirer"}
                                }
                            }
                            form{onsubmit:{
                                let t=token.clone();let id=id.clone();
                                move |event|{
                                    event.prevent_default();if *busy.read(){return;}
                                    let t=t.clone();let id=id.clone();
                                    let at=minute.read().parse::<i32>().unwrap_or(-1);
                                    let selected=kind.read().clone();
                                    let who=actor.read().clone();let on=incoming.read().clone();
                                    let type_id=custom_type.read().clone();let description=notes.read().clone();
                                    busy.set(true);
                                    spawn(async move{
                                        let result=matches::event(&t,&id,at,&selected,
                                            if type_id.is_empty(){None}else{Some(&type_id)},
                                            if who.is_empty(){None}else{Some(&who)},
                                            if on.is_empty(){None}else{Some(&on)},&description).await;
                                        match result{Ok(())=>{notes.set(String::new());data.restart()},
                                            Err(e)=>notice.set(Some(e))}
                                        busy.set(false);
                                    });
                                }
                            },
                                div{class:"form-grid",
                                    div{class:"field",label{"Minute"}
                                        input{r#type:"number",min:"0",max:"240",value:"{minute}",
                                            oninput:move |e|minute.set(e.value())}}
                                    div{class:"field",label{"Type d'événement"}
                                        select{value:"{kind}",onchange:move |e|kind.set(e.value()),
                                            option{value:"goal","But"}
                                            option{value:"substitution","Changement"}
                                            option{value:"yellow_card","Carton jaune"}
                                            option{value:"red_card","Carton rouge"}
                                            option{value:"injury","Blessure"}
                                            option{value:"other","Autre"}
                                        }
                                    }
                                    div{class:"field",label{"Joueuse concernée"}
                                        select{value:"{actor}",onchange:move |e|actor.set(e.value()),
                                            option{value:"","Non renseignée"}
                                            for p in &detail.participants{
                                                option{value:"{p.player_id}","{p.player_id}"}
                                            }
                                        }
                                    }
                                    if *kind.read()=="substitution"{
                                        div{class:"field",label{"Joueuse entrante"}
                                            select{value:"{incoming}",onchange:move |e|incoming.set(e.value()),
                                                option{value:"","Choisir"}
                                                for p in &detail.participants{
                                                    option{value:"{p.player_id}","{p.player_id}"}
                                                }
                                            }
                                        }
                                    }
                                    if *kind.read()=="other"{
                                        div{class:"field",label{"Type personnalisé"}
                                            select{value:"{custom_type}",onchange:move |e|custom_type.set(e.value()),
                                                option{value:"","Événement libre"}
                                                for t in &detail.event_types{option{value:"{t.id}","{t.label}"}}
                                            }
                                        }
                                    }
                                    div{class:"field",label{"Précisions"}
                                        input{value:"{notes}",maxlength:"2000",oninput:move |e|notes.set(e.value())}}
                                }
                                button{class:"button solid",r#type:"submit",disabled:*busy.read(),"Ajouter l'événement"}
                            }
                            form{onsubmit:{
                                let t=token.clone();let id=id.clone();
                                move |event|{
                                    event.prevent_default();
                                    let t=t.clone();let id=id.clone();let label=new_type.read().clone();
                                    spawn(async move{
                                        match matches::custom_type(&t,&id,&label).await{
                                            Ok(())=>{new_type.set(String::new());data.restart()},
                                            Err(e)=>notice.set(Some(e))
                                        }
                                    });
                                }
                            },
                                div{class:"field",label{"Créer un type d'événement"}
                                    input{value:"{new_type}",required:true,maxlength:"80",
                                        oninput:move |e|new_type.set(e.value())}}
                                button{class:"button outline",r#type:"submit","Créer le type"}
                            }
                        }
                        section{class:"panel",
                            div{class:"panel-head",h2{"Temps de jeu calculés"}}
                            for time in detail.playing_times{
                                div{class:"ticket-row",key:"{time.player_id}",
                                    strong{"{time.player_id}"}
                                    span{"{time.minutes} min"}
                                }
                            }
                        }
                    },
                    Some(Err(e))=>rsx!{p{class:"form-error","{e}"}},
                    None=>rsx!{p{"Chargement…"}}
                }
            }
        }
    }
}
#[component]
fn MatchLineup(
    token:String,game_id:String,team_id:String,
    initial:Vec<matches::Participant>,on_saved:EventHandler<()>,
)->Element{
    let t=token.clone();let team=team_id.clone();
    let roster=use_resource(move ||{let t=t.clone();let team=team.clone();
        async move{organization::team_players(&t,&team).await}});
    let mut lineup=use_signal(||initial.iter().map(|p|matches::LineupPlayer{
        player_id:p.player_id.clone(),position:p.position.clone(),starter:p.starter
    }).collect::<Vec<_>>());
    let mut busy=use_signal(||false);let mut error=use_signal(||None::<String>);
    rsx!{
        section{class:"panel",
            div{class:"panel-head",h2{"Composition et postes"}
                p{"Cochez les titulaires ; les remplacements détermineront les minutes jouées."}}
            match roster.read().clone(){
                Some(Ok(players))=>rsx!{
                    for player in players{
                        {
                            let id=player.id.clone();
                            let entry=lineup.read().iter().find(|p|p.player_id==id).cloned();
                            let starter=entry.as_ref().is_some_and(|p|p.starter);
                            let position=entry.map(|p|p.position).unwrap_or_default();
                            rsx!{
                                div{class:"ticket-row",key:"{id}",
                                    strong{"{player.firstname} {player.lastname}"}
                                    label{
                                        input{r#type:"checkbox",checked:starter,
                                            onchange:{
                                                let id=id.clone();
                                                move |e|{
                                                    let mut items=lineup.read().clone();
                                                    if let Some(p)=items.iter_mut().find(|p|p.player_id==id){p.starter=e.checked();}
                                                    else{items.push(matches::LineupPlayer{player_id:id.clone(),position:String::new(),starter:e.checked()});}
                                                    lineup.set(items);
                                                }
                                            }}
                                        "Titulaire"
                                    }
                                    input{placeholder:"Poste",maxlength:"60",value:"{position}",
                                        oninput:{
                                            let id=id.clone();
                                            move |e|{
                                                let mut items=lineup.read().clone();
                                                if let Some(p)=items.iter_mut().find(|p|p.player_id==id){p.position=e.value();}
                                                else{items.push(matches::LineupPlayer{player_id:id.clone(),position:e.value(),starter:false});}
                                                lineup.set(items);
                                            }
                                        }}
                                }
                            }
                        }
                    }
                    button{class:"button outline",disabled:*busy.read(),
                        onclick:{
                            let t=token.clone();let id=game_id.clone();
                            move |_|{
                                busy.set(true);let t=t.clone();let id=id.clone();
                                let players=lineup.read().clone();
                                spawn(async move{
                                    match matches::lineup(&t,&id,&players).await{
                                        Ok(())=>on_saved.call(()),Err(e)=>error.set(Some(e))
                                    }
                                    busy.set(false);
                                });
                            }
                        },"Enregistrer la composition"}
                },
                Some(Err(e))=>rsx!{p{class:"form-error","{e}"}},
                None=>rsx!{p{"Chargement de l'effectif…"}}
            }
            if let Some(e)=error.read().as_ref(){p{class:"form-error","{e}"}}
        }
    }
}
