use anyhow::anyhow;
use poise::serenity_prelude as serenity;
use poise::{serenity_prelude::GatewayIntents, PrefixFrameworkOptions};
use reaction_roles::commands::{change_reaction_role_emoji, remove_reaction_role};
use reaction_roles::{commands::list_reaction_role, event_handler::handle_events};
use reaction_roles::{
    commands::{add_reaction_role, hello, ping},
    Data,
};
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), reaction_roles::Error> {
    env_logger::init();

    dotenvy::dotenv().ok();

    let Ok(token) = std::env::var("DISCORD_TOKEN") else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found"));
    };

    let Ok(db_url) = std::env::var("DATABASE_URL") else {
        return Err(anyhow!("'DATABASE_URL' was not found"));
    };

    let pool = PgPool::connect(&db_url).await?;

    sqlx::migrate!().run(&pool).await?;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                hello(),
                ping(),
                add_reaction_role(),
                remove_reaction_role(),
                list_reaction_role(),
                change_reaction_role_emoji(),
            ],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(String::from("!")),
                ..Default::default()
            },
            event_handler: |ctx, event, _fctx, data| {
                Box::pin(handle_events(ctx, event, _fctx, data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { db_pool: pool })
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(
        token,
        GatewayIntents::GUILD_MESSAGE_REACTIONS
            | GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_EMOJIS_AND_STICKERS,
    )
    .framework(framework)
    .await?;

    client.start().await?;

    Ok(())
}
