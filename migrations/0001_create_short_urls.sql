CREATE SEQUENCE IF NOT EXISTS short_urls_id_seq AS BIGINT;

CREATE TABLE IF NOT EXISTS short_urls (
  id BIGINT PRIMARY KEY DEFAULT nextval('short_urls_id_seq'),
  short_code VARCHAR(16) NOT NULL UNIQUE,
  long_url TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- `short_code` already has an index via the UNIQUE constraint above.
-- This index exists for chapter-8 deduplication lookups by exact `long_url`.
CREATE INDEX IF NOT EXISTS idx_short_urls_long_url
  ON short_urls (long_url);
