//! Private ticket attachments: allowlisted raster formats, magic-byte checks,
//! scoped filenames, bounded uploads and an authenticated read route.
use axum::{Extension,body::Bytes,extract::{Path,State},http::{StatusCode,header,HeaderValue},response::{IntoResponse,Response},Json};
use sea_orm::{ActiveModelTrait,ActiveValue::Set,EntityTrait};
use uuid::Uuid;
use crate::{entities::{ticket,ticket_message},middleware::CurrentRole,
    routes::{authentication::login::Claims,miscellaneous::{tickets,ticket_live}},state::AppState};
const LIMIT:usize=4*1024*1024;
fn kind(bytes:&[u8])->Option<(&'static str,&'static str)>{
    if bytes.starts_with(&[137,80,78,71,13,10,26,10]){Some(("png","image/png"))}
    else if bytes.starts_with(&[255,216,255]){Some(("jpg","image/jpeg"))}
    else if bytes.starts_with(b"RIFF")&&bytes.get(8..12)==Some(b"WEBP"){Some(("webp","image/webp"))}
    else if bytes.starts_with(b"GIF87a")||bytes.starts_with(b"GIF89a"){Some(("gif","image/gif"))}
    else{None}
}
fn allowed_file(name:&str)->Option<&'static str>{
    let (id,ext)=name.rsplit_once('.')?;
    Uuid::parse_str(id).ok()?;
    match ext{"png"=>Some("image/png"),"jpg"=>Some("image/jpeg"),
        "webp"=>Some("image/webp"),"gif"=>Some("image/gif"),_=>None}
}
pub async fn upload(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(ticket_id):Path<Uuid>,bytes:Bytes,
)->Result<(StatusCode,Json<ticket_message::Model>),StatusCode>{
    let row=tickets::access(&state,&claims,role.0,ticket_id).await?;
    if row.status=="closed"&&!role.0.is_admin(){return Err(StatusCode::CONFLICT);}
    if bytes.is_empty()||bytes.len()>LIMIT{return Err(StatusCode::PAYLOAD_TOO_LARGE);}
    let (extension,_)=kind(&bytes).ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;
    let filename=format!("{}.{}",Uuid::new_v4(),extension);
    let directory=format!("data/ticket-uploads/{ticket_id}");
    tokio::fs::create_dir_all(&directory).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let path=format!("{directory}/{filename}");
    tokio::fs::write(&path,&bytes).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|time|time.as_secs() as i64).unwrap_or_default();
    let record=ticket_message::ActiveModel{
        id:Set(Uuid::new_v4()),ticket_id:Set(ticket_id),author_id:Set(claims.sub),
        message:Set(String::new()),attachment_name:Set(Some(filename)),
        created_at:Set(stamp),..Default::default()
    }.insert(&state.db).await.map_err(|e|{
        tracing::error!(%e,"Ticket upload database failure");
        let _=std::fs::remove_file(&path);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let mut active:ticket::ActiveModel=row.into();
    active.updated_at=Set(stamp);
    active.update(&state.db).await.map_err(|e|{
        tracing::error!(%e,"Ticket attachment update failed");StatusCode::INTERNAL_SERVER_ERROR
    })?;
    ticket_live::changed(ticket_id,claims.sub);
    Ok((StatusCode::CREATED,Json(record)))
}
pub async fn download(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path((ticket_id,name)):Path<(Uuid,String)>,
)->Result<Response,StatusCode>{
    tickets::access(&state,&claims,role.0,ticket_id).await?;
    let mime=allowed_file(&name).ok_or(StatusCode::NOT_FOUND)?;
    let path=format!("data/ticket-uploads/{ticket_id}/{name}");
    let data=tokio::fs::read(path).await.map_err(|_|StatusCode::NOT_FOUND)?;
    if data.len()>LIMIT{return Err(StatusCode::NOT_FOUND);}
    let mut response=([(header::CONTENT_TYPE,mime),
         (header::CACHE_CONTROL,"private, no-store")],data).into_response();
    response.headers_mut().insert(header::X_CONTENT_TYPE_OPTIONS,HeaderValue::from_static("nosniff"));
    Ok(response)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn only_raster_images_and_safe_filenames(){
        assert_eq!(kind(b"GIF89a\x00\x00"),Some(("gif","image/gif")));
        assert_eq!(kind(b"<svg onload=alert(1)>"),None);
        assert_eq!(kind(b"%PDF-1.7"),None);
        assert_eq!(allowed_file("../payload.svg"),None);
        assert_eq!(allowed_file("foo.png"),None);
        assert_eq!(allowed_file(&format!("{}.png",Uuid::new_v4())),Some("image/png"));
    }
}
