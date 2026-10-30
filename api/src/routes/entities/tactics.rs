//! Minimal built-in tactical board with validated, structured vector objects.
//! Data are owned by a user; there is no untrusted SVG or embedded JavaScript.
use axum::{Extension,Json,extract::{Path,State},http::StatusCode};
use sea_orm::{ActiveModelTrait,ActiveValue::Set,EntityTrait,QueryFilter};
use serde::{Deserialize,Serialize};
use uuid::Uuid;
use crate::{entities::tactical_diagram,routes::authentication::login::Claims,state::AppState};

type ApiResult<T> = Result<T,StatusCode>;
fn error(_:sea_orm::DbErr)->StatusCode{StatusCode::INTERNAL_SERVER_ERROR}

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct DrawingObject {
    pub id:String,
    pub kind:String,
    pub x:f64,
    pub y:f64,
    pub end_x:Option<f64>,
    pub end_y:Option<f64>,
    pub label:Option<String>,
    #[serde(default)]
    pub color:Option<String>,
    #[serde(default)]
    pub size:Option<f64>,
    #[serde(default)]
    pub control_x:Option<f64>,
    #[serde(default)]
    pub control_y:Option<f64>,
    #[serde(default)]
    pub dashed:bool,
}
#[derive(Clone,Serialize,Deserialize)]
pub struct DiagramPayload {
    pub name:String,
    pub layout:String,
    pub objects:Vec<DrawingObject>,
}
#[derive(Serialize)]
pub struct Diagram {
    pub id:Uuid,
    pub name:String,
    pub layout:String,
    pub objects:Vec<DrawingObject>,
}
fn valid_color(value:&str)->bool{
    value.len()==7 && value.starts_with('#') &&
        value.as_bytes()[1..].iter().all(|c|c.is_ascii_hexdigit())
}
fn validate(body:&DiagramPayload)->bool{
    let kind_ok=|k:&str|matches!(k,"player"|"opponent"|"cone"|"ball"|"arrow"|"curve"|"goal");
    !body.name.trim().is_empty() && body.name.len()<=150 &&
    matches!(body.layout.as_str(),"football"|"volleyball"|"blank") &&
    body.objects.len()<=250 &&
    body.objects.iter().all(|o| {
        kind_ok(&o.kind) && !o.id.is_empty() && o.id.len()<=64 &&
        o.label.as_deref().is_none_or(|s|s.len()<=35) &&
        o.color.as_deref().is_none_or(valid_color) &&
        o.size.is_none_or(|s|s.is_finite()&&(8.0..=65.0).contains(&s)) &&
        o.control_x.is_none_or(|x|x.is_finite()&&(0.0..=800.0).contains(&x)) &&
        o.control_y.is_none_or(|y|y.is_finite()&&(0.0..=500.0).contains(&y)) &&
        o.x.is_finite() && o.y.is_finite() &&
        (0.0..=800.0).contains(&o.x) && (0.0..=500.0).contains(&o.y) &&
        o.end_x.is_none_or(|x|x.is_finite()&&(0.0..=800.0).contains(&x)) &&
        o.end_y.is_none_or(|y|y.is_finite()&&(0.0..=500.0).contains(&y))
    })
}
fn view(row:tactical_diagram::Model)->ApiResult<Diagram>{
    let objects:Vec<DrawingObject>=serde_json::from_str(&row.objects_json)
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Diagram{id:row.id,name:row.name,layout:row.layout,objects})
}
async fn owned(state:&AppState,claims:&Claims,id:Uuid)->ApiResult<tactical_diagram::Model>{
    tactical_diagram::Entity::find_by_id(id)
        .filter(tactical_diagram::COLUMN.user_id.eq(claims.sub))
        .one(&state.db).await.map_err(error)?.ok_or(StatusCode::NOT_FOUND)
}
pub async fn list(
    Extension(claims):Extension<Claims>,State(state):State<AppState>
)->ApiResult<Json<Vec<Diagram>>>{
    let all=tactical_diagram::Entity::find()
        .filter(tactical_diagram::COLUMN.user_id.eq(claims.sub))
        .all(&state.db).await.map_err(error)?;
    let views=all.into_iter().map(view).collect::<ApiResult<Vec<_>>>()?;
    Ok(Json(views))
}
pub async fn create(
    Extension(claims):Extension<Claims>,State(state):State<AppState>,Json(body):Json<DiagramPayload>
)->ApiResult<(StatusCode,Json<Diagram>)>{
    if !validate(&body){return Err(StatusCode::BAD_REQUEST);}
    let json=serde_json::to_string(&body.objects).map_err(|_|StatusCode::BAD_REQUEST)?;
    let saved=tactical_diagram::ActiveModel{
        id:Set(Uuid::new_v4()),user_id:Set(claims.sub),
        name:Set(body.name.trim().into()),layout:Set(body.layout),objects_json:Set(json),
        ..Default::default()
    }.insert(&state.db).await.map_err(error)?;
    tracing::info!(actor=%claims.sub,diagram=%saved.id,action="tactics.create");
    Ok((StatusCode::CREATED,Json(view(saved)?)))
}
pub async fn update(
    Extension(claims):Extension<Claims>,State(state):State<AppState>,
    Path(id):Path<Uuid>,Json(body):Json<DiagramPayload>
)->ApiResult<Json<Diagram>>{
    if !validate(&body){return Err(StatusCode::BAD_REQUEST);}
    let row=owned(&state,&claims,id).await?;
    let mut active:tactical_diagram::ActiveModel=row.into();
    active.name=Set(body.name.trim().into());
    active.layout=Set(body.layout);
    active.objects_json=Set(serde_json::to_string(&body.objects).map_err(|_|StatusCode::BAD_REQUEST)?);
    let saved=active.update(&state.db).await.map_err(error)?;
    tracing::info!(actor=%claims.sub,diagram=%id,action="tactics.update");
    Ok(Json(view(saved)?))
}
pub async fn delete(
    Extension(claims):Extension<Claims>,State(state):State<AppState>,Path(id):Path<Uuid>
)->ApiResult<StatusCode>{
    let row=owned(&state,&claims,id).await?;
    let active:tactical_diagram::ActiveModel=row.into();
    active.delete(&state.db).await.map_err(error)?;
    tracing::info!(actor=%claims.sub,diagram=%id,action="tactics.delete");
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn dashed_arrow_styles_round_trip_and_old_schemes_remain_solid(){
        let older=r#"{"id":"a","kind":"arrow","x":10,"y":20,"end_x":80,"end_y":90,"label":null}"#;
        let old:DrawingObject=serde_json::from_str(older).unwrap();
        assert!(!old.dashed);
        let mut changed=old;
        changed.dashed=true;
        let value=serde_json::to_string(&changed).unwrap();
        let restored:DrawingObject=serde_json::from_str(&value).unwrap();
        assert!(restored.dashed);
        assert!(value.contains(r#""dashed":true"#));
    }
    #[test]
    fn rejects_untrusted_objects_and_invalid_positions(){
        let mut good=DiagramPayload{name:"Exercice".into(),layout:"football".into(),
            objects:vec![DrawingObject{id:"1".into(),kind:"player".into(),x:400.0,y:250.0,
                end_x:None,end_y:None,label:Some("A1".into()),color:None,size:None,control_x:None,control_y:None,dashed:false}]};
        assert!(validate(&good));
        good.objects[0].color=Some("#123ABC".into());
        assert!(validate(&good));
        good.objects[0].color=Some("red;script".into());
        assert!(!validate(&good));
        good.objects[0].color=None;
        good.objects[0].kind="script".into();
        assert!(!validate(&good));
        good.objects[0].kind="player".into();
        good.objects[0].x=f64::INFINITY;
        assert!(!validate(&good));
    }
}
