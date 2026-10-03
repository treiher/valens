"""
Delete notes of exercises without a set.

Revision ID: c55e237099fd
Revises: 9e3c6a1f8b27
Create Date: 2026-10-03

"""

from alembic import op

revision = "c55e237099fd"
down_revision = "9e3c6a1f8b27"
branch_labels = None
depends_on = None

WITHOUT_SET = """
    NOT EXISTS (
        SELECT 1 FROM workout_set
        WHERE workout_set.workout_id = workout_exercise_note.workout_id
        AND workout_set.exercise_id = workout_exercise_note.exercise_id
    )
"""


def upgrade() -> None:
    # Browsers that cached the training sessions of an affected user would otherwise get a 304 and
    # send the deleted notes again, which the API rejects.
    op.execute(
        f"""
        INSERT INTO data_version (user_id, collection, version)
        SELECT DISTINCT workout.user_id, 'workouts', 1
        FROM workout_exercise_note
        JOIN workout ON workout.id = workout_exercise_note.workout_id
        WHERE {WITHOUT_SET}
        ON CONFLICT (user_id, collection) DO UPDATE SET version = version + 1
        """
    )
    op.execute(f"DELETE FROM workout_exercise_note WHERE {WITHOUT_SET}")


def downgrade() -> None:
    pass
