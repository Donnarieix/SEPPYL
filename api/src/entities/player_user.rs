use sea_orm::DeriveEntityModel;
use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "players_users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub user_id: Uuid,
    #[sea_orm(primary_key)]
    pub player_id: Uuid,

    #[sea_orm(belongs_to, from = "user_id", to = "id")]
    pub user: BelongsTo<super::user::Entity>,
    #[sea_orm(belongs_to, from = "player_id", to = "id")]
    pub player: BelongsTo<super::player::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
