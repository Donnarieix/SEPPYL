use sea_orm::DeriveEntityModel;
use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "activities_players")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub activity_id: Uuid,
    #[sea_orm(primary_key)]
    pub player_id: Uuid,

    #[sea_orm(belongs_to, from = "activity_id", to = "id")]
    pub activity: BelongsTo<super::activity::Entity>,

    #[sea_orm(belongs_to, from = "player_id", to = "id")]
    pub player: BelongsTo<super::player::Entity>,

    #[sea_orm(column_type = "Text", nullable)]
    pub result: Option<String>,
}

impl ActiveModelBehavior for ActiveModel {}
