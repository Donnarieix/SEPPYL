use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name="clubs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub name: String,
    /// Public visibility is explicit, never enabled by default.
    pub is_public: Option<bool>,
    // Never expose member identities by default.
    pub show_public_teams: Option<bool>,
    pub show_public_roster: Option<bool>,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub logo_x: Option<i32>,
    pub logo_y: Option<i32>,
    pub logo_zoom: Option<i32>,
}
impl ActiveModelBehavior for ActiveModel {}
