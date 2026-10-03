"""
Add workout_exercise_measurement.

Revision ID: 9e3c6a1f8b27
Revises: 5b2e9d7c41fa
Create Date: 2026-10-03

"""

import sqlalchemy as sa
from alembic import op

revision = "9e3c6a1f8b27"
down_revision = "5b2e9d7c41fa"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.create_table(
        "workout_exercise_measurement",
        sa.Column("workout_id", sa.Integer(), nullable=False),
        sa.Column("exercise_id", sa.Integer(), nullable=False),
        sa.Column("side", sa.Integer(), nullable=False),
        sa.Column("value", sa.Float(), nullable=False),
        sa.CheckConstraint(
            "typeof(side) = 'integer'",
            name=op.f("ck_workout_exercise_measurement_side_type_integer"),
        ),
        sa.CheckConstraint(
            "typeof(value) = 'real'",
            name=op.f("ck_workout_exercise_measurement_value_type_real"),
        ),
        sa.CheckConstraint("side >= 0", name=op.f("ck_workout_exercise_measurement_side_ge_0")),
        sa.CheckConstraint("side <= 2", name=op.f("ck_workout_exercise_measurement_side_le_2")),
        sa.CheckConstraint(
            "value >= -9999.99", name=op.f("ck_workout_exercise_measurement_value_ge_min")
        ),
        sa.CheckConstraint(
            "value <= 9999.99", name=op.f("ck_workout_exercise_measurement_value_le_max")
        ),
        sa.ForeignKeyConstraint(
            ["exercise_id"],
            ["exercise.id"],
            name=op.f("fk_workout_exercise_measurement_exercise_id_exercise"),
            ondelete="CASCADE",
        ),
        sa.ForeignKeyConstraint(
            ["workout_id"],
            ["workout.id"],
            name=op.f("fk_workout_exercise_measurement_workout_id_workout"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint(
            "workout_id", "exercise_id", "side", name=op.f("pk_workout_exercise_measurement")
        ),
    )


def downgrade() -> None:
    # Browsers that cached the training sessions of an affected user would otherwise get a 304 and
    # show the dropped measurements again after a later upgrade.
    op.execute(
        """
        INSERT INTO data_version (user_id, collection, version)
        SELECT DISTINCT workout.user_id, 'workouts', 1
        FROM workout_exercise_measurement
        JOIN workout ON workout.id = workout_exercise_measurement.workout_id
        WHERE true
        ON CONFLICT (user_id, collection) DO UPDATE SET version = version + 1
        """
    )
    op.drop_table("workout_exercise_measurement")
