CREATE TABLE IF NOT EXISTS short_urls (
  id BIGINT PRIMARY KEY,
  short_code VARCHAR(16) NOT NULL UNIQUE,
  long_url TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_short_urls_long_url
  ON short_urls (long_url);
