use sea_orm::{Database, DatabaseConnection};

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}
impl AppState {
    pub async fn default() -> Self {
        let db = Database::connect("sqlite://data/sappyl.db?mode=rwc")
            .await
            .expect("Cannot open database");
        db.get_schema_registry("api::entities::*")
            .sync(&db)
            .await
            .expect("Cannot synchronize database schema");
        // Never provision a publicly known administrator account.
        Self { db }
    }
}
