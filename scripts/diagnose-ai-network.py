"""Inspect only connection/error metadata in a disposable database copy."""
import json
import os
import sqlite3
from pathlib import Path
from urllib.parse import urlsplit

path = Path(os.environ['MSL_DIAG_DB']).resolve()
assert '.test-runtime' in path.parts, 'Only an isolated database copy is allowed'
db = sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)
db.row_factory = sqlite3.Row
runs = []
for row in db.execute('SELECT id, trigger, status, error_code, error_message, started_at, finished_at, provider_model_id FROM analysis_runs ORDER BY id DESC LIMIT 20'):
    r = dict(row)
    error = r.pop('error_message') or ''
    r['duration_seconds'] = (r['finished_at'] or r['started_at']) - r['started_at']
    r['failure_category'] = next((code for text, code in [
        ('error sending request', 'request_send'), ('HTTP 500', 'http_500'),
        ('JSON', 'invalid_json'), ('未绑定', 'missing_route'), ('无正文', 'empty_content'),
    ] if text in error), 'other' if error else None)
    runs.append(r)
routes = []
for row in db.execute('''SELECT r.task_kind, p.template_kind, p.base_url,
    p.enabled provider_enabled, m.model_id, m.id provider_model_id, m.protocol,
    m.endpoint_path, m.enabled model_enabled, m.available
    FROM ai_task_routes r LEFT JOIN provider_models m ON m.id=r.provider_model_id
    LEFT JOIN provider_settings p ON p.id=m.provider_id'''):
    r = dict(row)
    url = urlsplit(r.pop('base_url') or '')
    r['endpoint_host'] = url.hostname
    r['base_path'] = url.path
    routes.append(r)
print(json.dumps({'runs': runs, 'routes': routes}, ensure_ascii=False, indent=2))
db.close()
