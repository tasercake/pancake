"""Store review grades and comments by run, scene, and stacker."""

from contextlib import closing
import csv
from datetime import datetime, timedelta, timezone
from io import StringIO
import sqlite3

SCHEMA = """
    CREATE TABLE IF NOT EXISTS ratings (
        run_id TEXT NOT NULL, scene_id TEXT NOT NULL, stacker TEXT NOT NULL,
        grade INTEGER CHECK (grade BETWEEN 1 AND 5),
        comment TEXT NOT NULL, updated_at TEXT NOT NULL,
        PRIMARY KEY (run_id, scene_id, stacker)
    )
"""
UNCONDITIONAL = object()


class RatingConflict(Exception):
    def __init__(self, rating):
        super().__init__("This rating changed elsewhere.")
        self.rating = rating


class Ratings:
    def __init__(self, store):
        self.path = store / "ratings.sqlite3"
        with closing(sqlite3.connect(self.path)) as db, db:
            db.execute("BEGIN IMMEDIATE")
            db.execute(SCHEMA)
            required, = db.execute(
                """SELECT "notnull" FROM pragma_table_info('ratings') WHERE name = 'grade'"""
            ).fetchone()
            if required:
                db.execute("ALTER TABLE ratings RENAME TO ratings_with_required_grade")
                db.execute(SCHEMA)
                db.execute("INSERT INTO ratings SELECT * FROM ratings_with_required_grade")
                db.execute("DROP TABLE ratings_with_required_grade")

    def get(self, run_id, scene_id):
        with closing(sqlite3.connect(self.path)) as db:
            db.row_factory = sqlite3.Row
            return {row["stacker"]: dict(row) for row in db.execute(
                "SELECT stacker, grade, comment, updated_at FROM ratings WHERE run_id = ? AND scene_id = ?",
                (run_id, scene_id))}

    def save(self, run_id, scene_id, stacker, grade, comment, *, expected=UNCONDITIONAL):
        if grade is not None and (type(grade) is not int or not 1 <= grade <= 5):
            raise ValueError("Choose a grade from 1 to 5")
        if not isinstance(comment, str) or len(comment) > 4000:
            raise ValueError("Comments must be at most 4,000 characters")
        with closing(sqlite3.connect(self.path)) as db, db:
            db.row_factory = sqlite3.Row
            db.execute("BEGIN IMMEDIATE")
            current = db.execute("""
                SELECT stacker, grade, comment, updated_at FROM ratings
                WHERE run_id = ? AND scene_id = ? AND stacker = ?
            """, (run_id, scene_id, stacker)).fetchone()
            current_at = current["updated_at"] if current else None
            if current and (current["grade"], current["comment"]) == (grade, comment):
                # A reconnect may replay a write whose acknowledgement was lost.
                return dict(current)
            if expected is not UNCONDITIONAL and expected != current_at:
                raise RatingConflict(dict(current) if current else None)
            now = datetime.now(timezone.utc)
            if current_at:
                now = max(now, datetime.fromisoformat(current_at) + timedelta(microseconds=1))
            updated_at = now.isoformat(timespec="microseconds")
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
