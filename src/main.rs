use anyhow::anyhow;
use poise::{serenity_prelude::GatewayIntents, PrefixFrameworkOptions};
use reaction_roles::ReactionRole;
use reaction_roles::{
    commands::{add_reaction_role, hello, ping},
    Data,
};
use shuttle_runtime::CustomError;
use shuttle_secrets::SecretStore;
use sqlx::PgPool;

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
    #[shuttle_shared_db::Postgres(
        local_uri = r#"postgresql://postgres:123@localhost:5445/postgres"#
    )]
    pool: PgPool,
) -> shuttle_poise::ShuttlePoise<Data, reaction_roles::Error> {
    // Get the discord token set in `Secrets.toml`
    let Some(token) = secret_store.get("DISCORD_TOKEN") else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found").into());
    };

    sqlx::query!(
        r#"DO $$
    BEGIN
        IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'emoji_types') THEN
            CREATE TYPE emoji_types AS ENUM ( 'unicode', 'emote' );
        END IF;
    END$$;"#,
    )
    .execute(&pool)
    .await
    .map_err(CustomError::new)?;
    sqlx::query!(
        r#"CREATE TABLE IF NOT EXISTS reaction_roles (
    message_link TEXT NOT NULL,
    emoji_type emoji_types NOT NULL,
    reaction_emoji_id TEXT,
    reaction_emoji_name TEXT NOT NULL,
    role_id TEXT NOT NULL
);"#
    )
    .execute(&pool)
    .await
    .map_err(CustomError::new)?;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![hello(), ping(), add_reaction_role()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(String::from("!")),
                ..Default::default()
            },
            event_handler: |ctx, event, _fctx, data| {
                Box::pin(async move {
                    let pool = data.db_pool.clone();
                    match event {
                        poise::Event::ReactionAdd { add_reaction } => {
                            let message = add_reaction.message(ctx.http.clone()).await?;
                            let reactions: Vec<ReactionRole> = sqlx::query_as!(
                                ReactionRole,
                                "SELECT message_link, reaction_emoji_name, reaction_emoji_id FROM reaction_roles WHERE message_link = $1 AND reaction_emoji_name = $2",
                                    message.link(),
                                    add_reaction.emoji.to_string())
                                .fetch_all(&pool)
                                .await
                                .map_err(CustomError::new)?;
                            println!("{:#?}", reactions);
                        }
                        _ => {}
                    };
                    Ok(())
                })
            },
            ..Default::default()
        })
        .token(token)
        .intents(
            GatewayIntents::GUILD_MESSAGE_REACTIONS
                | GatewayIntents::GUILDS
                | GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT
                | GatewayIntents::GUILD_EMOJIS_AND_STICKERS,
        )
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { db_pool: pool })
            })
        })
        .build()
        .await
        .map_err(shuttle_runtime::CustomError::new)?;

    Ok(framework.into())
}
