#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        crate::routes::authentication::login::login,
        crate::routes::authentication::register::register,

        crate::routes::entities::activity::get_activities,
        crate::routes::entities::activity::get_activity,
        crate::routes::entities::activity::create_activity,
        crate::routes::entities::activity::update_activity,
        crate::routes::entities::activity::update_activity_player,
        crate::routes::entities::activity::delete_activity,

        crate::routes::entities::player::get_players,
        crate::routes::entities::player::get_player,
        crate::routes::entities::player::create_player,
        crate::routes::entities::player::update_player,
        crate::routes::entities::player::delete_player,
    )
)]
pub struct ApiDoc;