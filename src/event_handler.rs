use poise::serenity_prelude as serenity;

use ::serenity::all::CreateMessage;
use serenity::all::FullEvent;

use crate::{Data, Error, ReturnReactionId, ReturnRoleId};

pub async fn handle_events(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _fctx: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    let pool = data.db_pool.clone();
    match event {
        FullEvent::ReactionRemove { removed_reaction } => {
            let message = removed_reaction.message(&ctx).await?;
            let message_link = message.link();

            let Some(role_id) = sqlx::query_as::<sqlx::Postgres, ReturnRoleId>(
                                    r#"SELECT role_id FROM reaction_roles WHERE message_link = $1 AND reaction_emoji_name = $2"#)
                                    .bind(message_link)
                                    .bind(removed_reaction.emoji.to_string())
                                    .fetch_optional(&pool)
                                    .await? else {
                                        return Ok(());
                                    };

            if let Some((guild_id, user_id)) =
                removed_reaction.guild_id.zip(removed_reaction.user_id)
            {
                let member = guild_id.member(&ctx, user_id).await?;
                member
                    .remove_role(
                        &ctx,
                        role_id
                            .role_id
                            .parse::<u64>()
                            .expect("role id should be parsable to u64"),
                    )
                    .await?;
            }

            let user = removed_reaction.user(ctx.http.clone()).await?;
            let dm = user
                .direct_message(
                    ctx.http.clone(),
                    CreateMessage::default().content("Removed role successfully :)"),
                )
                .await;

            if let Err(e) = dm {
                tracing::error!("Failed to send DM to user {} : {}", user.id, e);
            }
        }
        FullEvent::ReactionAdd { add_reaction } => {
            let message = add_reaction.message(ctx.http.clone()).await?;

            tracing::info!("emoji name: {}", add_reaction.emoji.to_string());

            let Some(reaction_role_id) = sqlx::query_as::<sqlx::Postgres, ReturnReactionId>(
                r#"SELECT id FROM reaction_roles rr WHERE rr.reaction_emoji_name = $1"#,
            )
            .bind(add_reaction.emoji.to_string())
            .fetch_optional(&pool)
            .await?
            else {
                return Ok(());
            };

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
            let dm = user
                .direct_message(
                    ctx.http.clone(),
                    CreateMessage::default().content("Added role successfully :)"),
                )
                .await;

            if let Err(e) = dm {
                tracing::error!("Failed to send DM to user {} : {}", user.id, e);
            }
        }
        _ => {}
    };

    Ok(())
}
