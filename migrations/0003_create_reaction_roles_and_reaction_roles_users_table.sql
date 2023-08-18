CREATE TABLE IF NOT EXISTS reaction_roles_and_users (
    reaction_role_id UUID NOT NULL,
    reaction_role_user_discord_id TEXT NOT NULL,

    FOREIGN KEY(reaction_role_id)
        REFERENCES reaction_roles(id)
        ON DELETE CASCADE
        ON UPDATE CASCADE,

    FOREIGN KEY(reaction_role_user_discord_id)
        REFERENCES reaction_roles_users(id)
        ON DELETE CASCADE
        ON UPDATE CASCADE,

    PRIMARY KEY(reaction_role_id, reaction_role_user_discord_id)
);
