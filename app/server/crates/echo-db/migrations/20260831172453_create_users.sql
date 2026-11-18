-- Add migration script here
CREATE TABLE IF NOT EXISTS users (
	id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
	email TEXT NOT NULL UNIQUE,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TYPE failure_code AS ENUM ('invalid_state', 'missing_codec', 'unsupported_audio_type', 'missing_object', 'probe_rejected', 'pcm_rejected', 'empty_file', 'file_too_large', 'size_mismatch', 'no_audio_stream', 'multi_audio_streams', 'missing_duration', 'invalid_duration', 'recording_too_long');
CREATE TYPE recording_status AS ENUM ('initial', 'abort', 'uploading', 'uploaded', 'recovering', 'validating', 'validated', 'processing','processed', 'completed', 'invalid');

CREATE TABLE IF NOT EXISTS recordings (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  client_id UUID NOT NULL,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  location TEXT,
  upload_id TEXT,
  mime_type TEXT NOT NULL DEFAULT '',

  status recording_status NOT NULL DEFAULT 'initial',

  recovery_id UUID,
  recovery_expires TIMESTAMPTZ,

  validation_started TIMESTAMPTZ,
  processing_claimed TIMESTAMPTZ,
  processing_claim_id UUID,

  failure_code failure_code NULL,

  UNIQUE (user_id, client_id)
);


CREATE TABLE IF NOT EXISTS transcripts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  recording_id UUID NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

  content TEXT,

	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
