"""
Add height to user.

Revision ID: c5e1a9f7d3b2
Revises: b4f3a8c1d2e5
Create Date: 2026-06-07

"""

from typing import Union

import sqlalchemy as sa
from alembic import op

revision = "c5e1a9f7d3b2"
down_revision = "b4f3a8c1d2e5"
branch_labels = None
depends_on = None


check_constraints: list[tuple[str, Union[str, sa.ColumnElement[bool]]]] = [
    ("height_type_integer_or_null", "typeof(height) = 'integer' or typeof(height) = 'null'"),
    ("height_gt_0", sa.column("height") > 0),
]


def upgrade() -> None:
    with op.batch_alter_table("user", schema=None) as batch_op:
        batch_op.add_column(sa.Column("height", sa.Integer(), nullable=True))
        for constraint_name, condition in check_constraints:
            batch_op.create_check_constraint(constraint_name, condition)


def downgrade() -> None:
    with op.batch_alter_table("user", schema=None) as batch_op:
        batch_op.drop_column("height")
        for constraint_name, _ in check_constraints:
            batch_op.drop_constraint(constraint_name, type_="check")
