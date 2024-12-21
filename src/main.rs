use reaction_roles::{ReturnRoleId, commands::list_reaction_role};
use anyhow::anyhow;
use poise::{serenity_prelude::GatewayIntents, PrefixFrameworkOptions};
use reaction_roles::{
    commands::{add_reaction_role, hello, ping},
    Data,
};
use reaction_roles::ReturnReactionId;
use serenity::all::{CreateMessage, FullEvent};
use shuttle_runtime::{CustomError, SecretStore};
use sqlx::PgPool;
use poise::serenity_prelude as serenity;

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_runtime::Secrets] secret_store: SecretStore,
    #[shuttle_shared_db::Postgres(
        local_uri = r#"postgresql://postgres:123@localhost:5445/postgres"#
    )]
    pool: PgPool,
) -> shuttle_serenity::ShuttleSerenity {
    // Get the discord token set in `Secrets.toml`
    let Some(token) = secret_store.get("DISCORD_TOKEN") else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found").into());
    };

    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(CustomError::new)?;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![hello(), ping(), add_reaction_role(), list_reaction_role()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(String::from("!")),
                ..Default::default()
            },
            event_handler: |ctx, event, _fctx, data| {
                Box::pin(async move {
                    let pool = data.db_pool.clone();
                    match event {
                        FullEvent::ReactionRemove { removed_reaction } => {
                            let message = removed_reaction.message(&ctx).await?;
                            let message_link = message.link();

                            if let Some(role_id) = sqlx::query_as::<sqlx::Postgres, ReturnRoleId>(
                                    r#"SELECT role_id FROM reaction_roles WHERE message_link = $1 AND reaction_emoji_name = $2"#)
                                    .bind(message_link)
                                    .bind(removed_reaction.emoji.to_string())
                                    .fetch_optional(&pool)
                                    .await? {
                                if let Some((guild_id, user_id)) = removed_reaction.guild_id.zip(removed_reaction.user_id) {
                                    let member = guild_id.member(&ctx, user_id).await?;
                                    member.remove_role(&ctx, role_id.role_id.parse::<u64>().expect("role id should be parsable to u64")).await?;
                                }
                            }

                            let user = removed_reaction.user(ctx.http.clone()).await?;
                            let dm = user.direct_message(ctx.http.clone(),
                                CreateMessage::default()
                                    .content("Removed role successfully :)")
                            ).await;

                            if let Err(e) = dm {
                                tracing::error!("Failed to send DM to user {} : {}", user.id, e);
                            }
                        }
                        FullEvent::ReactionAdd { add_reaction } => {
                            let message = add_reaction.message(ctx.http.clone()).await?;

                            tracing::info!("emoji name: {}", add_reaction.emoji.to_string());

                            let reaction_role_id = sqlx::query_as::<sqlx::Postgres, ReturnReactionId>(
                                        r#"SELECT id FROM reaction_roles rr WHERE rr.reaction_emoji_name = $1"#,
                                    )
                                .bind(add_reaction.emoji.to_string())
                                .fetch_one(&pool).await?;

                            tracing::info!("created reaction role with id: {}", reaction_role_id.id);

                            let message_link = message.link();
                            
                            if let Some(role_id) = sqlx::query_as::<sqlx::Postgres, ReturnRoleId>(
                                    r#"SELECT role_id FROM reaction_roles WHERE message_link = $1 AND reaction_emoji_name = $2"#)
                                .bind(message_link)
                                .bind(add_reaction.emoji.to_string())
                                    .fetch_optional(&pool)
                                    .await? {
                                if let Some((guild_id, user_id)) = add_reaction.guild_id.zip(add_reaction.user_id) {
                                    let member = guild_id.member(&ctx, user_id).await?;
                                    member.add_role(&ctx, role_id.role_id.parse::<u64>().expect("role id should be parsable to u64")).await?;
                                }
                            }

                            let user = add_reaction.user(ctx.http.clone()).await?;
                            let dm = user.direct_message(ctx.http.clone(),
                                CreateMessage::default()
                                    .content("Added role successfully :)")
                            ).await;

                            if let Err(e) = dm {
                                tracing::error!("Failed to send DM to user {} : {}", user.id, e);
                            }
                        }
                        _ => {}
                    };

                    Ok(())
                })
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

    let client = serenity::ClientBuilder::new(token, GatewayIntents::GUILD_MESSAGE_REACTIONS
                | GatewayIntents::GUILDS
                | GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT
                | GatewayIntents::GUILD_EMOJIS_AND_STICKERS,).framework(framework).await.map_err(|e| {
                    shuttle_runtime::Error::Custom(e.into())
                })?;

    Ok(client.into())
}
