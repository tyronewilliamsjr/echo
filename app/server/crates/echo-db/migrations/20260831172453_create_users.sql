-- Add migration script here
CREATE TABLE IF NOT EXISTS users (
	id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
	email TEXT NOT NULL UNIQUE,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS recordings (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  client_id UUID NOT NULL,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  location TEXT,
  upload_id TEXT,
  mime_type TEXT NOT NULL,

  status TEXT NOT NULL DEFAULT 'initial',

  recovery_id UUID,
  recovery_expires TIMESTAMPTZ,

  UNIQUE (user_id, client_id)
);
