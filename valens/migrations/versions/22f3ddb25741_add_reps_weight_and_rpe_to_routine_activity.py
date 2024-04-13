"""
Add reps, weight and rpe to routine_activity.

Revision ID: 22f3ddb25741
Revises: 8a0dc258bf2a
Create Date: 2023-04-01

"""

from typing import Union

import sqlalchemy as sa
from alembic import op

revision = "22f3ddb25741"
down_revision = "8a0dc258bf2a"
branch_labels = None
depends_on = None


check_constraints: list[tuple[str, Union[str, sa.ColumnElement[bool]]]] = [
    ("reps_type_integer", "typeof(reps) = 'integer'"),
    ("weight_type_real", "typeof(weight) = 'real'"),
    ("rpe_type_real", "typeof(rpe) = 'real'"),
    ("reps_ge_0", sa.column("reps") >= 0),
    ("weight_ge_0", sa.column("weight") >= 0),
    ("rpe_ge_0", sa.column("rpe") >= 0),
    ("rpe_le_10", sa.column("rpe") <= 10),
]


def upgrade() -> None:
    with op.batch_alter_table("routine_activity", schema=None) as batch_op:
        batch_op.add_column(sa.Column("reps", sa.Integer(), nullable=False, default=0))
        batch_op.add_column(sa.Column("weight", sa.Float(), nullable=False, default=0))
        batch_op.add_column(sa.Column("rpe", sa.Float(), nullable=False, default=0))
        for constraint_name, condition in check_constraints:
            batch_op.create_check_constraint(constraint_name, condition)


def downgrade() -> None:
    with op.batch_alter_table("routine_activity", schema=None) as batch_op:
        batch_op.drop_column("rpe")
        batch_op.drop_column("weight")
        batch_op.drop_column("reps")
        for constraint_name, _ in check_constraints:
            batch_op.drop_constraint(constraint_name, type_="check")
