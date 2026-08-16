"""
Require positive RPE in workout set.

Revision ID: a1c8e5b2d947
Revises: f3b9d17c5a2e
Create Date: 2026-08-16

"""

from alembic import op
from sqlalchemy import column

revision = "a1c8e5b2d947"
down_revision = "f3b9d17c5a2e"
branch_labels = None
depends_on = None

COLUMN_NAMES = ["rpe", "target_rpe"]


def upgrade() -> None:
    for column_name in COLUMN_NAMES:
        op.execute(f"UPDATE workout_set SET {column_name} = NULL WHERE {column_name} = 0")
    with op.batch_alter_table("workout_set") as batch_op:
        for column_name in COLUMN_NAMES:
            batch_op.drop_constraint(f"ck_workout_set_{column_name}_ge_0")
            batch_op.create_check_constraint(f"{column_name}_gt_0", column(column_name) > 0)


def downgrade() -> None:
    with op.batch_alter_table("workout_set") as batch_op:
        for column_name in COLUMN_NAMES:
            batch_op.drop_constraint(f"ck_workout_set_{column_name}_gt_0")
            batch_op.create_check_constraint(f"{column_name}_ge_0", column(column_name) >= 0)
