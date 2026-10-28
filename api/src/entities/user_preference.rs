use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "user_preferences")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub user_id: Uuid,
    pub attendance_view: String,
    // Nullable so schema-sync can add these fields to older SQLite databases.
    pub theme: Option<String>,
    pub density: Option<String>,
}

impl ActiveModelBehavior for ActiveModel {}
