use reaction_roles::ReturnRoleId;
use anyhow::anyhow;
use poise::{serenity_prelude::GatewayIntents, PrefixFrameworkOptions};
use reaction_roles::{
    commands::{add_reaction_role, hello, ping},
    Data,
};
use reaction_roles::{ReturnReactionId, ReturnUserId};
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

    sqlx::migrate!()
        .run(&pool)
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
                        poise::Event::ReactionRemove { removed_reaction } => {
                            sqlx::query(
                                r#"DELETE FROM reaction_roles_users rru WHERE rru.id = $1"#,
                            )
                            .bind(removed_reaction.user_id.expect("user should have an id").to_string())
                            .execute(&pool)
                            .await?;

                            let message = removed_reaction.message(&ctx).await?;
                            let message_link = message.link_ensured(&ctx).await;

                            if let Some(role_id) = sqlx::query_as::<sqlx::Postgres, ReturnRoleId>(
                                    r#"SELECT role_id FROM reaction_roles WHERE message_link = $1 AND reaction_emoji_name = $2"#)
                                    .bind(message_link)
                                    .bind(removed_reaction.emoji.to_string())
                                    .fetch_optional(&pool)
                                    .await? {
                                if let Some((guild_id, user_id)) = removed_reaction.guild_id.zip(removed_reaction.user_id) {
                                    let mut member = guild_id.member(&ctx, user_id).await?;
                                    member.remove_role(&ctx, role_id.role_id.parse::<u64>().expect("role id should be parsable to u64")).await?;
                                }
                            }
                        }
                        poise::Event::ReactionAdd { add_reaction } => {
                            let message = add_reaction.message(ctx.http.clone()).await?;

                            let reaction_roles_id = sqlx::query_as::<sqlx::Postgres, ReturnReactionId>(
                                        r#"SELECT id FROM reaction_roles rr WHERE rr.reaction_emoji_name = $1"#,
                                    ).bind(add_reaction.emoji.to_string()).fetch_one(&pool).await?;

                            let user = add_reaction.user(&ctx).await?;

                            let reaction_roles_user_id = match sqlx::query_as::<sqlx::Postgres, ReturnUserId>(
                                    r#"SELECT id FROM reaction_roles_users WHERE id = $1"#)
                                    .bind(user.id.to_string())
                                    .fetch_optional(&pool)
                                    .await? {
                                Some(id) => id,
                                None => {
                                    sqlx::query_as::<sqlx::Postgres, ReturnUserId>(
                                        r#"INSERT INTO reaction_roles_users ( id, username ) VALUES ( $1, $2 ) RETURNING id"#,
                                    )
                                        .bind(user.id.to_string())
                                        .bind(user.name.to_string())
                                    .fetch_one(&pool)
                                    .await?
                                }
                            };

                            sqlx::query(
                                r#"INSERT INTO reaction_roles_and_users ( reaction_role_id, reaction_role_user_discord_id ) VALUES ( $1, $2 ) ON CONFLICT DO NOTHING"#
                            )
                                .bind(reaction_roles_id.id)
                                .bind(reaction_roles_user_id.id)
                            .execute(&pool)
                            .await?;

                            let message_link = message.link_ensured(&ctx).await;
                            
                            if let Some(role_id) = sqlx::query_as::<sqlx::Postgres, ReturnRoleId>(
                                    r#"SELECT role_id FROM reaction_roles WHERE message_link = $1 AND reaction_emoji_name = $2"#)
                                .bind(message_link)
                                .bind(add_reaction.emoji.to_string())
                                    .fetch_optional(&pool)
                                    .await? {
                                if let Some((guild_id, user_id)) = add_reaction.guild_id.zip(add_reaction.user_id) {
                                    let mut member = guild_id.member(&ctx, user_id).await?;
                                    member.add_role(&ctx, role_id.role_id.parse::<u64>().expect("role id should be parsable to u64")).await?;
                                }
                            }

                            // let reactions: Vec<ReactionRole> = sqlx::query_as::<sqlx::Postgres, ReactionRole>(
                            //     "SELECT rr.message_link, rr.reaction_emoji_name, rr.reaction_emoji_id, rr.id,
                            //         rru.id as user_discord_id, rru.username
                            //         FROM reaction_roles_and_users rrandrru
                            //         INNER JOIN reaction_roles rr
                            //         ON rrandrru.reaction_role_id = rr.id
                            //         INNER JOIN reaction_roles_users rru
                            //         ON rrandrru.reaction_role_user_discord_id = rru.id
                            //         WHERE ")
                            //     .bind(message_link)
                            //     .bind(add_reaction.emoji.to_string())
                            //     .fetch_all(&pool)
                            //     .await
                            //     .map_err(CustomError::new)?;
                            // tracing::info!("{:#?}", reactions);
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
