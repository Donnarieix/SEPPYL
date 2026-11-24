use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;

/// Immutable pre-edit snapshot; a new revision is added for each edit or deletion.
#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name="support_ticket_message_revisions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub message_id: Uuid,
    pub edited_by: Uuid,
    pub previous_text: String,
    pub previous_attachment_name: Option<String>,
    pub action: String,
    pub changed_at: i64,
}
impl ActiveModelBehavior for ActiveModel {}
