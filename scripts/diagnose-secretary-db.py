#!/usr/bin/env python3
"""Read-only secretary workflow diagnostics; never prints content or credentials."""

import json
import os
import sqlite3
import time


def rows(connection: sqlite3.Connection, query: str, params: tuple = ()) -> list[dict]:
    return [dict(row) for row in connection.execute(query, params).fetchall()]


def main() -> None:
    database_path = os.environ["MSL_DIAG_DB"].replace("\\", "/")
    connection = sqlite3.connect(f"file:{database_path}?mode=ro", uri=True)
    connection.row_factory = sqlite3.Row
    cutoff = int(time.time()) - 7 * 86_400
    result = {
        "runs": rows(
            connection,
            """
            SELECT r.id, r.trigger, r.status, r.error_code, r.error_message, r.created_at,
                   (SELECT COUNT(*) FROM ai_proposals p WHERE p.analysis_run_id=r.id) AS proposals
            FROM analysis_runs r ORDER BY r.id DESC LIMIT 15
            """,
        ),
        "proposal_counts": rows(
            connection,
            """
            SELECT COUNT(*) AS total,
                   SUM(CASE WHEN status='pending' THEN 1 ELSE 0 END) AS pending,
                   SUM(CASE WHEN status='pending' AND deferred_at IS NOT NULL THEN 1 ELSE 0 END) AS deferred,
                   SUM(CASE WHEN created_at>=? THEN 1 ELSE 0 END) AS recent7
            FROM ai_proposals
            """,
            (cutoff,),
        ),
        "routes": rows(
            connection,
            """
            SELECT r.task_kind, p.template_kind, p.enabled AS provider_enabled,
                   m.enabled AS model_enabled, m.available
            FROM ai_task_routes r
            JOIN provider_models m ON m.id=r.provider_model_id
            JOIN provider_settings p ON p.id=m.provider_id
            WHERE r.task_kind IN ('global_analysis','daily_brief')
            """,
        ),
        "latest_completed": rows(
            connection,
            """
            SELECT r.id, r.status,
                   (SELECT COUNT(*) FROM ai_proposals p
                    WHERE p.analysis_run_id=r.id AND p.status='pending' AND p.deferred_at IS NULL) AS visible
            FROM analysis_runs r
            WHERE r.status='completed'
              AND EXISTS (SELECT 1 FROM ai_proposals p WHERE p.analysis_run_id=r.id)
            ORDER BY COALESCE(r.finished_at,r.started_at) DESC,r.id DESC LIMIT 1
            """,
        ),
    }
    connection.close()
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
