use crate::EmojiType;
use crate::ReturnReactionId;
use poise::serenity_prelude::{self as serenity, ArgumentConvert, Emoji, ReactionType};

use crate::{Context, Error};

#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn hello(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("🤨🤚").await?;
    Ok(())
}

/// add reaction role to message
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn add_reaction_role(
    ctx: Context<'_>,
    role: serenity::Role,
    msg: serenity::Message,
    emoji: String,
) -> Result<(), Error> {
    let pool = ctx.data().db_pool.clone();

    let (reaction_type, emoji_id) = match emojis::get(&emoji) {
        Some(_unicode_emoji) => (ReactionType::Unicode(emoji.clone()), None),
        None => {
            let emoji = Emoji::convert(
                &ctx.serenity_context(),
                ctx.guild_id(),
                Some(ctx.channel_id()),
                &emoji,
            )
            .await?;
            (
                ReactionType::Custom {
                    animated: emoji.animated,
                    id: emoji.id,
                    name: Some(emoji.name.clone()),
                },
                Some(emoji.id.to_string()),
            )
        }
    };

    let reaction = msg.react(ctx, reaction_type).await?;

    let message_link = msg.link_ensured(&ctx).await;

    tracing::info!("message link: {}", message_link);

    if matches!(reaction.emoji, ReactionType::Unicode(_)) {
        tracing::info!("adding reaction emoji: {}", reaction.emoji.to_string());

        let reaction_roles_id = sqlx::query_as::<sqlx::Postgres, ReturnReactionId>(
            r#"INSERT INTO reaction_roles ( message_link, emoji_type, reaction_emoji_name, role_id ) VALUES ( $1, $2, $3, $4 ) RETURNING id"#
        )
        .bind(message_link.clone())
        .bind(EmojiType::Unicode)
        .bind(reaction.emoji.to_string())
        .bind(role.id.to_string())
        .fetch_one(&pool).await?;

        tracing::info!(
            "created new reaction role with id: {}",
            reaction_roles_id.id
        );
    } else {
        tracing::info!("adding reaction emoji: {}", reaction.emoji.to_string());
        let message_link = msg.link_ensured(&ctx).await;
        let reaction_roles_id = sqlx::query_as::<sqlx::Postgres, ReturnReactionId>(
            r#"INSERT INTO reaction_roles ( message_link, emoji_type, reaction_emoji_name, reaction_emoji_id, role_id ) VALUES ( $1, $2, $3, $4, $5 ) RETURNING id"#,
        )
        .bind(message_link)
        .bind(EmojiType::Emote)
        .bind(reaction.emoji.to_string())
        .bind(emoji_id.expect("emotes should have an id"))
        .bind(role.id.to_string())
        .fetch_one(&pool).await?;

        tracing::info!(
            "created new reaction role with id: {}",
            reaction_roles_id.id
        );
    }

    ctx.send(move |r| {
        r.reply(true).content(format!(
            "added role: {role} with emoji: {emoji} to message: {}",
            message_link
        ))
    })
    .await?;

    Ok(())
}

/// ping bot
#[poise::command(prefix_command, slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong! 🤨🤚").await?;
    Ok(())
}
