-- users table
CREATE TABLE users (
  id           UUID          PRIMARY KEY,
  email        TEXT          UNIQUE NOT NULL,
  display_name TEXT,
  avatar_url   TEXT,
  created_at   TIMESTAMPTZ   NOT NULL DEFAULT now(),
  updated_at   TIMESTAMPTZ   NOT NULL DEFAULT now()
);

-- connections table
CREATE TYPE connection_provider AS ENUM (
  'postgres',
  'bigquery',
  'snowflake',
  'databricks',
  's3',
  'gcs',
  'azure_blob',
  'sentinel_hub',
  'planet',
  'wms',
  'wfs'
);

CREATE TABLE connections (
  id           UUID                PRIMARY KEY DEFAULT gen_random_uuid(),
  owner_id     UUID                NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  name         TEXT                NOT NULL,
  provider     connection_provider NOT NULL,
  credentials  BYTEA               NOT NULL,
  config       JSONB               NOT NULL DEFAULT '{}',
  last_tested_at   TIMESTAMPTZ,
  last_test_ok     BOOLEAN,
  created_at   TIMESTAMPTZ         NOT NULL DEFAULT now(),
  updated_at   TIMESTAMPTZ         NOT NULL DEFAULT now(),
  UNIQUE(owner_id, name)
);

CREATE INDEX idx_connections_owner ON connections(owner_id);

-- workflows table
CREATE TABLE workflows (
  id          UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
  owner_id    UUID          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  name        TEXT          NOT NULL,
  description TEXT,
  graph       JSONB         NOT NULL DEFAULT '{"nodes":[],"edges":[]}',
  tags        TEXT[]        DEFAULT '{}',
  is_public   BOOLEAN       NOT NULL DEFAULT false,
  created_at  TIMESTAMPTZ   NOT NULL DEFAULT now(),
  updated_at  TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE INDEX idx_workflows_owner ON workflows(owner_id);
CREATE INDEX idx_workflows_graph ON workflows USING gin(graph);
