use crate::EmojiType;
use crate::ReactionRole;
use crate::ReturnReactionId;
use poise::serenity_prelude::{self as serenity, ArgumentConvert, Emoji, ReactionType};
use poise::CreateReply;

use crate::{Context, Error};

/// Say hi to the bot :)
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn hello(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("🤨🤚").await?;
    Ok(())
}

/// Add reaction role to message
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn add_reaction_role(
    ctx: Context<'_>,
    role: serenity::Role,
    msg: serenity::Message,
    emoji: String,
) -> Result<(), Error> {
    let pool = ctx.data().db_pool.clone();

    let reaction_type = match emojis::get(&emoji) {
        Some(_unicode_emoji) => ReactionType::Unicode(emoji.clone()),
        None => {
            let emoji = Emoji::convert(
                ctx.serenity_context(),
                ctx.guild_id(),
                Some(ctx.channel_id()),
                &emoji,
            )
            .await?;

            ReactionType::Custom {
                animated: emoji.animated,
                id: emoji.id,
                name: Some(emoji.name.clone()),
            }
        }
    };

    let message_link = msg.link();

    let None =
        sqlx::query(r#"SELECT * FROM reaction_roles WHERE message_link = $1 AND role_id = $2"#)
            .bind(&message_link)
            .bind(role.id.to_string())
            .fetch_optional(&pool)
            .await?
    else {
        return Err(anyhow::anyhow!("Role already has an emoji on this message"));
    };

    match &reaction_type {
        ReactionType::Unicode(name) => {
            tracing::info!("adding reaction emoji: {}", name);
            let reaction_roles_id = sqlx::query_as::<sqlx::Postgres,ReturnReactionId>(
                r#"INSERT INTO reaction_roles ( message_link, emoji_type, reaction_emoji_name, role_id ) VALUES ( $1, $2, $3, $4 ) RETURNING id"#
            )
                .bind(&message_link)
                .bind(EmojiType::Unicode)
                .bind(name)
                .bind(role.id.to_string())
                .fetch_one(&pool).await?;

            tracing::info!(
                "created new reaction role with id: {}",
                reaction_roles_id.id
            );
        }
        ReactionType::Custom {
            animated: _,
            id,
            name,
        } => {
            let Some(name) = name else {
                return Err(anyhow::anyhow!("Invalid emoji"));
            };

            tracing::info!("adding reaction emoji: {}", name);

            let message_link = msg.link();
            let reaction_roles_id = sqlx::query_as::<sqlx::Postgres,ReturnReactionId>(
                r#"INSERT INTO reaction_roles ( message_link, emoji_type, reaction_emoji_name, reaction_emoji_id, role_id ) VALUES ( $1, $2, $3, $4, $5 ) RETURNING id"#
            )
                .bind(message_link)
                .bind(EmojiType::Emote)
                .bind(name)
                .bind(id.to_string())
                .bind(role.id.to_string())
                .fetch_one(&pool).await?;
            tracing::info!(
                "created new reaction role with id: {}",
                reaction_roles_id.id
            );
        }
        _ => return Err(anyhow::anyhow!("Invalid emoji")),
    }

    let _reaction = msg.react(ctx, reaction_type).await?;

    tracing::info!("message link: {}", message_link);

    ctx.send(CreateReply::default().reply(true).content(format!(
        "added role: {role} with emoji: {emoji} to message: {}",
        message_link
    )))
    .await?;

    Ok(())
}

/// Change reaction role emoji
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn change_reaction_role_emoji(
    ctx: Context<'_>,
    role: serenity::Role,
    msg: serenity::Message,
    emoji: String,
) -> Result<(), Error> {
    let pool = ctx.data().db_pool.clone();

    let reaction_type = match emojis::get(&emoji) {
        Some(_unicode_emoji) => ReactionType::Unicode(emoji.clone()),
        None => {
            let emoji = Emoji::convert(
                ctx.serenity_context(),
                ctx.guild_id(),
                Some(ctx.channel_id()),
                &emoji,
            )
            .await?;

            ReactionType::Custom {
                animated: emoji.animated,
                id: emoji.id,
                name: Some(emoji.name.clone()),
            }
        }
    };

    let message_link = msg.link();

    match &reaction_type {
        ReactionType::Custom {
            animated: _,
            id,
            name,
        } => {
            let Some(name) = name else {
                return Err(anyhow::anyhow!("Invalid emoji"));
            };

            if let None = sqlx::query_as::<sqlx::Postgres, ReturnReactionId>(
                r#"UPDATE reaction_roles SET emoji_type = $1, reaction_emoji_id = $2, reaction_emoji_name = $3 WHERE message_link = $4 AND role_id = $5 RETURNING id"#,
            )
            .bind(EmojiType::Emote)
            .bind(id.to_string())
            .bind(name)
            .bind(&message_link)
            .bind(role.id.to_string())
            .fetch_optional(&pool)
            .await? {
                return Err(anyhow::anyhow!("Role does not exist on message"));
            }
        }
        ReactionType::Unicode(name) => {
            if let None = sqlx::query_as::<sqlx::Postgres, ReturnReactionId>(
                r#"UPDATE reaction_roles SET emoji_type = $1, reaction_emoji_name = $2 WHERE message_link = $3 AND role_id = $4 RETURNING id"#,
            )
            .bind(EmojiType::Emote)
            .bind(name)
            .bind(&message_link)
            .bind(role.id.to_string())
            .fetch_optional(&pool)
            .await? {
                return Err(anyhow::anyhow!("Role does not exist on message"));
            }
        }
        _ => {
            return Err(anyhow::anyhow!("Invalid emoji"));
        }
    }

    let _reaction = msg.react(ctx, reaction_type).await?;

    tracing::info!("message link: {}", message_link);

    ctx.send(CreateReply::default().reply(true).content(format!(
        "changed role: {role} to emoji: {emoji} on message: {}",
        message_link
    )))
    .await?;

    Ok(())
}

/// Remove reaction role from message
/// NOTE: this removes all previous reactions on the emoji
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn remove_reaction_role(
    ctx: Context<'_>,
    role: serenity::Role,
    msg: serenity::Message,
) -> Result<(), Error> {
    let pool = ctx.data().db_pool.clone();

    let message_link = msg.link();

    tracing::info!("message link: {}", message_link);

    let Some(reaction_role) = sqlx::query_as::<sqlx::Postgres, ReactionRole>(
        r#"SELECT * FROM reaction_roles WHERE message_link = $1 AND role_id = $2"#,
    )
    .bind(message_link.clone())
    .bind(role.id.to_string())
    .fetch_optional(&pool)
    .await?
    else {
        return Err(anyhow::anyhow!("Role does not exist on message"));
    };

    let reaction_type = match emojis::get(&reaction_role.reaction_emoji_name) {
        Some(_unicode_emoji) => ReactionType::Unicode(reaction_role.reaction_emoji_name.clone()),
        None => {
            let emoji = Emoji::convert(
                ctx.serenity_context(),
                ctx.guild_id(),
                Some(ctx.channel_id()),
                &reaction_role.reaction_emoji_name,
            )
            .await?;
            ReactionType::Custom {
                animated: emoji.animated,
                id: emoji.id,
                name: Some(emoji.name.clone()),
            }
        }
    };

    tracing::info!(
        "removing reaction emoji: {}",
        reaction_role.reaction_emoji_name
    );

    let reaction_roles_id = sqlx::query_as::<sqlx::Postgres, ReturnReactionId>(
                r#"DELETE FROM reaction_roles WHERE message_link = $1 AND emoji_type = $2 AND reaction_emoji_name = $3 AND role_id = $4 RETURNING id"#
            )
            .bind(message_link.clone())
            .bind(reaction_role.emoji_type)
            .bind(&reaction_role.reaction_emoji_name)
            .bind(role.id.to_string())
            .fetch_one(&pool).await?;

    tracing::info!(
        "created new reaction role with id: {}",
        reaction_roles_id.id
    );

    msg.delete_reaction_emoji(ctx, reaction_type).await?;

    ctx.reply(format!(
        "removed role emoji: {} from message: {}",
        reaction_role.reaction_emoji_name, message_link
    ))
    .await?;

    Ok(())
}

/// List reaction role of a message
#[poise::command(slash_command)]
pub async fn list_reaction_role(ctx: Context<'_>, msg: serenity::Message) -> Result<(), Error> {
    let pool = ctx.data().db_pool.clone();

    let message_link = msg.link();

    tracing::info!("message link: {}", message_link);

    ctx.defer_ephemeral().await?;

    let reaction_roles = sqlx::query_as::<sqlx::Postgres, ReactionRole>(
        r#"SELECT * FROM reaction_roles rr WHERE rr.message_link = $1"#,
    )
    .bind(message_link)
    .fetch_all(&pool)
    .await?;

    let message = reaction_roles
        .into_iter()
        .map(|rr| {
            format!(
                "- reaction {emoji} for <@&{role_id}>",
                emoji = rr.reaction_emoji_name,
                role_id = rr.role_id,
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    if message.is_empty() {
        ctx.send(
            CreateReply::default()
                .reply(true)
                .content("Message has no reaction roles"),
        )
        .await?;
    } else {
        ctx.send(CreateReply::default().reply(true).content(message))
            .await?;
    }

    Ok(())
}

/// Ping the bot
#[poise::command(prefix_command, slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let time = chrono::Utc::now().timestamp_millis() - ctx.created_at().timestamp_millis();
    ctx.say(format!("Pong! {time}ms latency")).await?;
    Ok(())
}
