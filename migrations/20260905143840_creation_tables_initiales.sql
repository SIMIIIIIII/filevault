-- Add migration script here
--CREATE TABLE utilisateurs (
--    id SERIAL PRIMARY KEY,
--    username TEXT NOT NULL UNIQUE,
--    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
--);

CREATE TABLE files (
    id SERIAL PRIMARY KEY,
    --user_id INTEGER NOT NULL REFERENCES utilisateurs(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    size BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

--CREATE INDEX idx_fichiers_user_id ON fichiers(user_id);
