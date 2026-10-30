use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name="teams")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub club_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub logo_x: Option<i32>,
    pub logo_y: Option<i32>,
    pub logo_zoom: Option<i32>,
}
impl ActiveModelBehavior for ActiveModel {}
