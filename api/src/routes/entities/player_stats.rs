//! Player minutes are derived, not editable counters.
use axum::{Extension,Json,extract::{Path,State},http::StatusCode};
use sea_orm::{ColumnTrait,EntityTrait,QueryFilter};
use serde::Serialize;
use uuid::Uuid;
use crate::{
 entities::{training,training_attendance,match_game,match_participant,match_event},
 middleware::CurrentRole,routes::{authentication::login::Claims,entities::{organization,matches}},
 services,state::AppState,
};
type R<T>=Result<T,StatusCode>;
fn err(e:sea_orm::DbErr)->StatusCode {
    tracing::error!(%e,"Player time statistics failed");
    StatusCode::INTERNAL_SERVER_ERROR
}
#[derive(Serialize)]
pub struct Totals {
    pub player_id:Uuid,
    pub training_minutes:i64,
    pub match_minutes:i64,
    pub training_attended:i32,
    pub trainings_without_duration:i32,
    pub matches_played:i32,
}
pub async fn get(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,
)->R<Json<Totals>>{
    let mut found=role.0.is_admin() ||
        services::player::get_players(&claims,&state).await?
            .iter().any(|p|p.id==id);
    let mut stats=Totals{player_id:id,training_minutes:0,match_minutes:0,
        training_attended:0,trainings_without_duration:0,matches_played:0};
    let attended=training_attendance::Entity::find()
        .filter(training_attendance::COLUMN.player_id.eq(id))
        .filter(training_attendance::COLUMN.present.eq(true))
        .all(&state.db).await.map_err(err)?;
    for attendance in attended {
        let Some(session)=training::Entity::find_by_id(attendance.training_id)
            .one(&state.db).await.map_err(err)? else{continue};
        let allowed=match session.team_id{
            Some(team)=>organization::can_read_team(&state,&claims,role.0,team).await?,
            None=>session.user_id==claims.sub || role.0.is_admin(),
        };
        if !allowed {continue;}
        found=true;stats.training_attended+=1;
        if let Some(duration)=session.duration_minutes{
            stats.training_minutes+=duration.max(0) as i64;
        }else{stats.trainings_without_duration+=1;}
    }
    let participants=match_participant::Entity::find()
        .filter(match_participant::COLUMN.player_id.eq(id))
        .all(&state.db).await.map_err(err)?;
    for participation in participants{
        let Some(game)=match_game::Entity::find_by_id(participation.match_id)
            .one(&state.db).await.map_err(err)? else{continue};
        if !organization::can_read_team(&state,&claims,role.0,game.team_id).await? {
            continue;
        }
        found=true;
        let lineup=match_participant::Entity::find()
            .filter(match_participant::COLUMN.match_id.eq(game.id))
            .all(&state.db).await.map_err(err)?;
        let events=match_event::Entity::find()
            .filter(match_event::COLUMN.match_id.eq(game.id))
            .all(&state.db).await.map_err(err)?;
        let totals=matches::compute_playtime(game.half_minutes,&lineup,&events)?;
        let minutes=totals.iter().find(|p|p.player_id==id).map(|p|p.minutes).unwrap_or(0);
        if minutes>0 {stats.matches_played+=1;}
        stats.match_minutes+=minutes as i64;
    }
    if !found{return Err(StatusCode::NOT_FOUND);}
    Ok(Json(stats))
}
