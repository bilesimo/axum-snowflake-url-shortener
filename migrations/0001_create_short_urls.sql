CREATE TABLE IF NOT EXISTS short_urls (
  id BIGINT PRIMARY KEY,
  short_code VARCHAR(16) NOT NULL UNIQUE,
  long_url TEXT NOT NULL,
  normalized_long_url TEXT NOT NULL UNIQUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_short_urls_created_at
  ON short_urls (created_at DESC);
