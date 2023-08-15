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
pub struct ReactionRole {
    pub message_link: String,
    pub reaction_emoji_name: String,
    pub reaction_emoji_id: Option<String>,
}
