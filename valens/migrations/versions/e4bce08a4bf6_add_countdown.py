"""
Add countdown to routine_activity and workout_set.

Revision ID: e4bce08a4bf6
Revises: c55e237099fd
Create Date: 2026-11-15

"""

import sqlalchemy as sa
from alembic import op

revision = "e4bce08a4bf6"
down_revision = "c55e237099fd"
branch_labels = None
depends_on = None


constraints = [
    ("countdown_type_integer", "typeof(countdown) = 'integer'"),
    ("countdown_ge_0", "countdown >= 0"),
    ("countdown_le_1", "countdown <= 1"),
]


def upgrade() -> None:
    for table in ("routine_activity", "workout_set"):
        with op.batch_alter_table(table) as batch_op:
            batch_op.add_column(
                sa.Column("countdown", sa.Boolean(), nullable=False, server_default="0")
            )
            for constraint_name, condition in constraints:
                batch_op.create_check_constraint(constraint_name, condition)

    # A hold and an automatic set with reps and a tempo used to be counted down.
    op.execute(
        """
        UPDATE routine_activity SET countdown = automatic OR reps = 0
        WHERE exercise_id IS NOT NULL AND json_array_length(tempo) > 0
        """
    )
    op.execute(
        """
        UPDATE workout_set SET countdown = target_reps IS NULL OR (
            SELECT workout_element.automatic FROM workout_element
            WHERE workout_element.workout_id = workout_set.workout_id
            AND workout_element.position = workout_set.position
        )
        WHERE target_tempo IS NOT NULL
        """
    )


def downgrade() -> None:
    # Browsers that cached the routines or training sessions of an affected user would otherwise
    # get a 304 and keep the chosen countdowns after a later upgrade.
    op.execute(
        """
        INSERT INTO data_version (user_id, collection, version)
        SELECT DISTINCT exercise.user_id, 'routines', 1
        FROM routine_activity
        JOIN exercise ON exercise.id = routine_activity.exercise_id
        WHERE routine_activity.countdown = 1
        ON CONFLICT (user_id, collection) DO UPDATE SET version = version + 1
        """
    )
    op.execute(
        """
        INSERT INTO data_version (user_id, collection, version)
        SELECT DISTINCT workout.user_id, 'workouts', 1
        FROM workout_set
        JOIN workout ON workout.id = workout_set.workout_id
        WHERE workout_set.countdown = 1
        ON CONFLICT (user_id, collection) DO UPDATE SET version = version + 1
        """
    )
    for table in ("routine_activity", "workout_set"):
        with op.batch_alter_table(table) as batch_op:
            for constraint_name, _ in constraints:
                batch_op.drop_constraint(f"ck_{table}_{constraint_name}")
            batch_op.drop_column("countdown")
