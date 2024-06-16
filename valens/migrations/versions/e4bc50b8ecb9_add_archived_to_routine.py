"""
Add archived to routine.

Revision ID: e4bc50b8ecb9
Revises: a6220efbbda7
Create Date: 2024-06-16

"""

import sqlalchemy as sa
from alembic import op

revision = "e4bc50b8ecb9"
down_revision = "a6220efbbda7"
branch_labels = None
depends_on = None


def upgrade() -> None:
    with op.batch_alter_table("routine", schema=None) as batch_op:
        batch_op.add_column(sa.Column("archived", sa.Boolean(), nullable=False, server_default="0"))


def downgrade() -> None:
    with op.batch_alter_table("routine", schema=None) as batch_op:
        batch_op.drop_column("archived")
