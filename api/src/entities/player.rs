use sea_orm::DeriveEntityModel;
use sea_orm::entity::prelude::*;
use serde::Serialize;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "players")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub firstname: String,
    pub lastname: String,
    pub number: u32,

    #[sea_orm(has_many, via="player_user")]
    #[serde(skip)]
    pub users: HasMany<super::user::Entity>,
    
    #[sea_orm(has_many, via="activity_player")]
    #[serde(skip)]
    pub activities: HasMany<super::activity::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
