use crate::EmojiType;
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

    if matches!(reaction.emoji, ReactionType::Unicode(_)) {
        sqlx::query!(
            r#"INSERT INTO reaction_roles ( message_link, emoji_type, reaction_emoji_name, role_id ) VALUES ( $1, $2, $3, $4 ) "#,
            msg.link(),
            EmojiType::Unicode as _,
            reaction.emoji.to_string(),
            role.id.to_string(),
        ).execute(&pool).await?;
    } else {
        sqlx::query!(
            r#"INSERT INTO reaction_roles ( message_link, emoji_type, reaction_emoji_id, reaction_emoji_name, role_id ) VALUES ( $1, $2, $3, $4, $5 ) "#,
            msg.link(),
            EmojiType::Emote as _,
            emoji_id
                .ok_or(anyhow::anyhow!("failed to get emoji id"))?,
            reaction.emoji.to_string(),
            role.id.to_string(),
        ).execute(&pool).await?;
    }

    ctx.send(move |r| {
        r.reply(true).content(format!(
            "added role: {role} with emoji: {emoji} to message: {}",
            msg.link()
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
