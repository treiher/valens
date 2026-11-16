"""
Allow alternating sides in routine_activity and workout_set.

Revision ID: e062925203fd
Revises: e4bce08a4bf6
Create Date: 2026-11-16

"""

from alembic import op

revision = "e062925203fd"
down_revision = "e4bce08a4bf6"
branch_labels = None
depends_on = None


def upgrade() -> None:
    with op.batch_alter_table("routine_activity") as batch_op:
        batch_op.drop_constraint("ck_routine_activity_sides_le_2")
        batch_op.create_check_constraint("sides_le_3", "sides <= 3")

    with op.batch_alter_table("workout_set") as batch_op:
        batch_op.drop_constraint("ck_workout_set_side_le_2")
        batch_op.create_check_constraint("side_le_3", "side <= 3")


def downgrade() -> None:
    # Browsers that cached the routines or training sessions of an affected user would otherwise
    # get a 304 and keep showing the alternating sides.
    op.execute(
        """
        INSERT INTO data_version (user_id, collection, version)
        SELECT DISTINCT exercise.user_id, 'routines', 1
        FROM routine_activity
        JOIN exercise ON exercise.id = routine_activity.exercise_id
        WHERE routine_activity.sides = 3
        ON CONFLICT (user_id, collection) DO UPDATE SET version = version + 1
        """
    )
    op.execute(
        """
        INSERT INTO data_version (user_id, collection, version)
        SELECT DISTINCT workout.user_id, 'workouts', 1
        FROM workout_set
        JOIN workout ON workout.id = workout_set.workout_id
        WHERE workout_set.side = 3
        ON CONFLICT (user_id, collection) DO UPDATE SET version = version + 1
        """
    )
    op.execute("UPDATE routine_activity SET sides = 1 WHERE sides = 3")
    op.execute("UPDATE workout_set SET side = NULL WHERE side = 3")

    with op.batch_alter_table("routine_activity") as batch_op:
        batch_op.drop_constraint("ck_routine_activity_sides_le_3")
        batch_op.create_check_constraint("sides_le_2", "sides <= 2")

    with op.batch_alter_table("workout_set") as batch_op:
        batch_op.drop_constraint("ck_workout_set_side_le_3")
        batch_op.create_check_constraint("side_le_2", "side <= 2")
