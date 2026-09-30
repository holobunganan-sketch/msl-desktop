-- Device-local bridge metadata. Bot credentials stay in the OS credential store.
-- These tables must never be included in cloud synchronization or portable exports.
CREATE TABLE weixin_binding (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    bot_id TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    base_url TEXT NOT NULL,
    credential_ref TEXT NOT NULL,
    bound_at INTEGER NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    cursor TEXT NOT NULL DEFAULT '',
    connection_state TEXT NOT NULL DEFAULT 'disabled',
    last_received_at INTEGER,
    last_global_at INTEGER
);
CREATE TABLE weixin_receipts (
    bot_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    received_at INTEGER NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('capture', 'global', 'rate_limited', 'queue_busy')),
    inbox_id INTEGER,
    job_id INTEGER,
    PRIMARY KEY (bot_id, message_id)
);
CREATE INDEX weixin_receipts_rate ON weixin_receipts(bot_id, received_at);
