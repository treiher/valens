"""
Add stretch to exercise_muscle and the flexibility category to exercise.

Revision ID: 5b2e9d7c41fa
Revises: 8d7f1bc0db68
Create Date: 2026-10-03

"""

import sqlalchemy as sa
from alembic import op

revision = "5b2e9d7c41fa"
down_revision = "8d7f1bc0db68"
branch_labels = None
depends_on = None


added_muscle_constraints = [
    ("stretch_integer", "typeof(stretch) = 'integer'"),
    ("stretch_ge_0", "stretch >= 0"),
    ("stretch_le_100", "stretch <= 100"),
    ("stimulus_or_stretch", "stimulus + stretch >= 1"),
]


def upgrade() -> None:
    with op.batch_alter_table("exercise_muscle") as batch_op:
        batch_op.add_column(sa.Column("stretch", sa.Integer(), nullable=False, server_default="0"))
        batch_op.drop_constraint("ck_exercise_muscle_stimulus_ge_1")
        batch_op.create_check_constraint("stimulus_ge_0", "stimulus >= 0")
        for constraint_name, condition in added_muscle_constraints:
            batch_op.create_check_constraint(constraint_name, condition)

    with op.batch_alter_table("exercise") as batch_op:
        batch_op.drop_constraint("ck_exercise_category_le_2")
        batch_op.create_check_constraint("category_le_3", "category <= 3")


def downgrade() -> None:
    # Browsers that cached the exercises of an affected user would otherwise get a 304 and keep
    # entries the old frontend cannot decode.
    op.execute(
        """
        INSERT INTO data_version (user_id, collection, version)
        SELECT DISTINCT user_id, 'exercises', 1 FROM (
            SELECT user_id FROM exercise_muscle WHERE stretch != 0 OR muscle_id IN (73, 74)
            UNION
            SELECT user_id FROM exercise WHERE category = 3
        ) WHERE true
        ON CONFLICT (user_id, collection) DO UPDATE SET version = version + 1
        """
    )
    op.execute("DELETE FROM exercise_muscle WHERE stimulus = 0 OR muscle_id IN (73, 74)")
    op.execute("UPDATE exercise SET category = NULL WHERE category = 3")

    with op.batch_alter_table("exercise") as batch_op:
        batch_op.drop_constraint("ck_exercise_category_le_3")
        batch_op.create_check_constraint("category_le_2", "category <= 2")

    with op.batch_alter_table("exercise_muscle") as batch_op:
        for constraint_name, _ in added_muscle_constraints:
            batch_op.drop_constraint(f"ck_exercise_muscle_{constraint_name}")
        batch_op.drop_constraint("ck_exercise_muscle_stimulus_ge_0")
        batch_op.create_check_constraint("stimulus_ge_1", "stimulus >= 1")
        batch_op.drop_column("stretch")
