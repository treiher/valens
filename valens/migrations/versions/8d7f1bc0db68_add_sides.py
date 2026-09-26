"""
Add sides to routine_activity and side to workout_set.

Revision ID: 8d7f1bc0db68
Revises: c7a4e93f6b15
Create Date: 2026-09-26

"""

import sqlalchemy as sa
from alembic import op

revision = "8d7f1bc0db68"
down_revision = "c7a4e93f6b15"
branch_labels = None
depends_on = None


activity_constraints = [
    ("sides_type_integer", "typeof(sides) = 'integer'"),
    ("sides_ge_1", "sides >= 1"),
    ("sides_le_2", "sides <= 2"),
]
set_constraints = [
    ("side_type_integer_or_null", "typeof(side) = 'integer' or typeof(side) = 'null'"),
    ("side_ge_1", "side >= 1"),
    ("side_le_2", "side <= 2"),
]


def upgrade() -> None:
    with op.batch_alter_table("routine_activity") as batch_op:
        batch_op.add_column(sa.Column("sides", sa.Integer(), nullable=False, server_default="1"))
        for constraint_name, condition in activity_constraints:
            batch_op.create_check_constraint(constraint_name, condition)

    with op.batch_alter_table("workout_set") as batch_op:
        batch_op.add_column(sa.Column("side", sa.Integer(), nullable=True))
        for constraint_name, condition in set_constraints:
            batch_op.create_check_constraint(constraint_name, condition)


def downgrade() -> None:
    with op.batch_alter_table("routine_activity") as batch_op:
        for constraint_name, _ in activity_constraints:
            batch_op.drop_constraint(f"ck_routine_activity_{constraint_name}")
        batch_op.drop_column("sides")

    with op.batch_alter_table("workout_set") as batch_op:
        for constraint_name, _ in set_constraints:
            batch_op.drop_constraint(f"ck_workout_set_{constraint_name}")
        batch_op.drop_column("side")
