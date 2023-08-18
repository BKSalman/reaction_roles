DO $$
    BEGIN
        IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'emoji_types') THEN
            CREATE TYPE emoji_types AS ENUM ( 'unicode', 'emote' );
        END IF;
    END$$;

CREATE TABLE IF NOT EXISTS reaction_roles (
    id SERIAL PRIMARY KEY,
    message_link TEXT NOT NULL,
    emoji_type emoji_types NOT NULL,
    reaction_emoji_id TEXT,
    reaction_emoji_name TEXT NOT NULL,
    role_id TEXT NOT NULL
);
