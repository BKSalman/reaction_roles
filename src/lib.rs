use sqlx::{FromRow, PgPool};

pub mod commands;

pub struct Data {
    pub db_pool: PgPool,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "emoji_types", rename_all = "snake_case")]
pub enum EmojiType {
    Emote,
    Unicode,
}

#[derive(Debug, FromRow)]
pub struct ReactionUser {
    /// user id in discord
    pub id: String,
    pub username: String,
}

#[derive(Debug, FromRow)]
pub struct ReactionRole {
    pub id: i32,
    pub role_id: String,
    pub message_link: String,
    pub reaction_emoji_name: String,
    pub reaction_emoji_id: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct ReturnReactionId {
    pub id: i32,
}

#[derive(Debug, FromRow)]
pub struct ReturnRoleId {
    pub role_id: String,
}

#[derive(Debug, FromRow)]
pub struct ReturnUserId {
    pub id: String,
}
