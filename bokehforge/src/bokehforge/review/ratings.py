"""Store one numeric grade and comment per run, scene, and stacker."""

from contextlib import closing
import csv
from datetime import datetime, timezone
from io import StringIO
import sqlite3


class Ratings:
    def __init__(self, store):
        self.path = store / "ratings.sqlite3"
        with closing(sqlite3.connect(self.path)) as db, db:
            db.execute("""
                CREATE TABLE IF NOT EXISTS ratings (
                    run_id TEXT NOT NULL, scene_id TEXT NOT NULL, stacker TEXT NOT NULL,
                    grade INTEGER NOT NULL CHECK (grade BETWEEN 1 AND 5),
                    comment TEXT NOT NULL, updated_at TEXT NOT NULL,
                    PRIMARY KEY (run_id, scene_id, stacker)
                )
            """)

    def get(self, run_id, scene_id):
        with closing(sqlite3.connect(self.path)) as db:
            db.row_factory = sqlite3.Row
            return {row["stacker"]: dict(row) for row in db.execute(
                "SELECT stacker, grade, comment, updated_at FROM ratings WHERE run_id = ? AND scene_id = ?",
                (run_id, scene_id))}

    def save(self, run_id, scene_id, stacker, grade, comment):
        if type(grade) is not int or not 1 <= grade <= 5:
            raise ValueError("Choose a grade from 1 to 5")
        if not isinstance(comment, str) or len(comment) > 4000:
            raise ValueError("Comments must be at most 4,000 characters")
        updated_at = datetime.now(timezone.utc).isoformat()
        with closing(sqlite3.connect(self.path)) as db, db:
            db.execute("""
                INSERT INTO ratings VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT (run_id, scene_id, stacker) DO UPDATE SET
                    grade = excluded.grade, comment = excluded.comment, updated_at = excluded.updated_at
            """, (run_id, scene_id, stacker, grade, comment, updated_at))
        return {"stacker": stacker, "grade": grade, "comment": comment, "updated_at": updated_at}

    def export_csv(self):
        export = StringIO(newline="")
        writer = csv.writer(export)
        with closing(sqlite3.connect(self.path)) as db:
            rows = db.execute("SELECT * FROM ratings ORDER BY run_id, scene_id, stacker")
            writer.writerow([column[0] for column in rows.description])
            writer.writerows(rows)
        return export.getvalue()
