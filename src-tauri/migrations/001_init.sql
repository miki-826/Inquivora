PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;

CREATE TABLE schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE workspaces (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  root_path TEXT NOT NULL UNIQUE,
  last_opened_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE files (
  id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL,
  relative_path TEXT NOT NULL,
  absolute_path TEXT NOT NULL,
  name TEXT NOT NULL,
  extension TEXT,
  size_bytes INTEGER NOT NULL DEFAULT 0,
  encoding TEXT,
  line_ending TEXT,
  content_hash TEXT,
  modified_at TEXT,
  indexed_at TEXT,
  is_binary INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
  UNIQUE(workspace_id, relative_path)
);

CREATE INDEX idx_files_workspace ON files(workspace_id);
CREATE INDEX idx_files_name ON files(name);

CREATE TABLE meetings (
  id TEXT PRIMARY KEY,
  workspace_id TEXT,
  title TEXT NOT NULL,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  timezone TEXT NOT NULL DEFAULT 'Asia/Tokyo',
  target_file_path TEXT NOT NULL,
  start_marker TEXT NOT NULL,
  end_marker TEXT NOT NULL,
  mic_audio_path TEXT,
  system_audio_path TEXT,
  summary TEXT,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);

CREATE INDEX idx_meetings_started ON meetings(started_at DESC);

CREATE TABLE transcript_segments (
  id TEXT PRIMARY KEY,
  meeting_id TEXT NOT NULL,
  source TEXT NOT NULL,
  speaker_label TEXT NOT NULL,
  start_ms INTEGER NOT NULL,
  end_ms INTEGER NOT NULL,
  text TEXT NOT NULL,
  confidence REAL,
  status TEXT NOT NULL,
  audio_chunk_path TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX idx_transcript_meeting_time
  ON transcript_segments(meeting_id, start_ms);

CREATE TABLE meeting_decisions (
  id TEXT PRIMARY KEY,
  meeting_id TEXT NOT NULL,
  text TEXT NOT NULL,
  source_start_ms INTEGER,
  created_at TEXT NOT NULL,
  FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT,
  due_at TEXT,
  timezone TEXT NOT NULL DEFAULT 'Asia/Tokyo',
  priority TEXT NOT NULL DEFAULT 'medium',
  status TEXT NOT NULL DEFAULT 'todo',
  assignee TEXT,
  project_name TEXT,
  meeting_id TEXT,
  linked_file_path TEXT,
  source_start_ms INTEGER,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  completed_at TEXT,
  FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE SET NULL
);

CREATE INDEX idx_tasks_due ON tasks(due_at);
CREATE INDEX idx_tasks_status_due ON tasks(status, due_at);
CREATE INDEX idx_tasks_meeting ON tasks(meeting_id);

CREATE TABLE task_candidates (
  id TEXT PRIMARY KEY,
  meeting_id TEXT NOT NULL,
  title TEXT NOT NULL,
  description TEXT,
  due_at TEXT,
  priority TEXT NOT NULL DEFAULT 'medium',
  assignee TEXT,
  source_start_ms INTEGER,
  status TEXT NOT NULL DEFAULT 'pending',
  created_at TEXT NOT NULL,
  FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE TABLE events (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT,
  start_at TEXT NOT NULL,
  end_at TEXT,
  timezone TEXT NOT NULL DEFAULT 'Asia/Tokyo',
  all_day INTEGER NOT NULL DEFAULT 0,
  event_type TEXT NOT NULL DEFAULT 'event',
  recurrence_rule TEXT,
  meeting_id TEXT,
  task_id TEXT,
  location TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE SET NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE INDEX idx_events_start ON events(start_at);

CREATE TABLE reminders (
  id TEXT PRIMARY KEY,
  task_id TEXT,
  event_id TEXT,
  notify_at TEXT NOT NULL,
  timezone TEXT NOT NULL DEFAULT 'Asia/Tokyo',
  status TEXT NOT NULL DEFAULT 'scheduled',
  sent_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
  FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE,
  CHECK (task_id IS NOT NULL OR event_id IS NOT NULL)
);

CREATE UNIQUE INDEX idx_reminder_unique
  ON reminders(COALESCE(task_id, ''), COALESCE(event_id, ''), notify_at);
CREATE INDEX idx_reminders_notify ON reminders(status, notify_at);

CREATE TABLE api_provider_profiles (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  provider_type TEXT NOT NULL,
  base_url TEXT NOT NULL,
  auth_type TEXT NOT NULL DEFAULT 'bearer',
  credential_target TEXT,
  organization_id TEXT,
  project_id TEXT,
  default_headers_json TEXT NOT NULL DEFAULT '{}',
  timeout_ms INTEGER NOT NULL DEFAULT 60000,
  capabilities_json TEXT NOT NULL DEFAULT '[]',
  enabled INTEGER NOT NULL DEFAULT 1,
  last_test_status TEXT,
  last_tested_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_api_provider_name
  ON api_provider_profiles(display_name);

CREATE TABLE ai_feature_bindings (
  feature_key TEXT PRIMARY KEY,
  provider_profile_id TEXT,
  model_id TEXT,
  fallback_provider_profile_id TEXT,
  fallback_model_id TEXT,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (provider_profile_id)
    REFERENCES api_provider_profiles(id) ON DELETE SET NULL,
  FOREIGN KEY (fallback_provider_profile_id)
    REFERENCES api_provider_profiles(id) ON DELETE SET NULL
);

-- feature_key:
-- transcription.batch
-- transcription.realtime
-- meeting.summary
-- editor.ai

CREATE TABLE api_usage_logs (
  id TEXT PRIMARY KEY,
  provider_profile_id TEXT NOT NULL,
  feature_key TEXT NOT NULL,
  model_id TEXT NOT NULL,
  entity_id TEXT,
  input_units INTEGER,
  output_units INTEGER,
  audio_duration_ms INTEGER,
  latency_ms INTEGER,
  status TEXT NOT NULL,
  error_code TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (provider_profile_id)
    REFERENCES api_provider_profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_api_usage_provider_created
  ON api_usage_logs(provider_profile_id, created_at);

CREATE TABLE api_jobs (
  id TEXT PRIMARY KEY,
  job_type TEXT NOT NULL,
  provider_profile_id TEXT,
  model_id TEXT,
  capability TEXT,
  entity_id TEXT,
  request_path TEXT,
  status TEXT NOT NULL,
  retry_count INTEGER NOT NULL DEFAULT 0,
  next_retry_at TEXT,
  last_error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (provider_profile_id)
    REFERENCES api_provider_profiles(id) ON DELETE SET NULL
);

CREATE INDEX idx_api_jobs_status_retry
  ON api_jobs(status, next_retry_at);

CREATE TABLE app_settings (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE recent_tabs (
  id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL,
  path TEXT NOT NULL,
  tab_order INTEGER NOT NULL,
  is_pinned INTEGER NOT NULL DEFAULT 0,
  cursor_line INTEGER NOT NULL DEFAULT 1,
  cursor_column INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE TABLE search_documents (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  path TEXT,
  updated_at TEXT NOT NULL,
  UNIQUE(entity_type, entity_id)
);

CREATE VIRTUAL TABLE search_documents_fts USING fts5(
  title,
  body,
  path UNINDEXED,
  entity_type UNINDEXED,
  entity_id UNINDEXED,
  tokenize='trigram'
);
