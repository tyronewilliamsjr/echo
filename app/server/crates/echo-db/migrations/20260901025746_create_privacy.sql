-- Add migration script here
CREATE TABLE IF NOT EXISTS user_privacy (
	id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

	user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
	version INTEGER NOT NULL,
	is_current BOOLEAN NOT NULL DEFAULT TRUE,

	retain_audio BOOLEAN NOT NULL DEFAULT TRUE,
	retain_transcript BOOLEAN NOT NULL DEFAULT TRUE,
	allow_embedding BOOLEAN NOT NULL DEFAULT TRUE,
	allow_reminder BOOLEAN NOT NULL DEFAULT TRUE,

	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

	UNIQUE(user_id, version)
);

CREATE UNIQUE INDEX one_current_privacy_per_user
ON user_privacy (user_id)
WHERE is_current = TRUE;
