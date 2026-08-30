"""
Add notes to exercise.

Revision ID: d5e2b71c4a83
Revises: c3f7a1d94b26
Create Date: 2026-08-30

"""

import sqlalchemy as sa
from alembic import op

revision = "d5e2b71c4a83"
down_revision = "c3f7a1d94b26"
branch_labels = None
depends_on = None


def upgrade() -> None:
    with op.batch_alter_table("exercise") as batch_op:
        batch_op.add_column(sa.Column("notes", sa.String(), nullable=True))


def downgrade() -> None:
    with op.batch_alter_table("exercise") as batch_op:
        batch_op.drop_column("notes")
