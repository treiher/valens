"""
Add data version.

Revision ID: e2f1a8c40b7d
Revises: 9c4e2f8b6a1d
Create Date: 2026-07-15

"""

import sqlalchemy as sa
from alembic import op

revision = "e2f1a8c40b7d"
down_revision = "9c4e2f8b6a1d"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.create_table(
        "data_version",
        sa.Column("user_id", sa.Integer(), nullable=False),
        sa.Column("collection", sa.String(), nullable=False),
        sa.Column("version", sa.Integer(), nullable=False),
        sa.ForeignKeyConstraint(
            ["user_id"],
            ["user.id"],
            name=op.f("fk_data_version_user_id_user"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("user_id", "collection", name=op.f("pk_data_version")),
    )


def downgrade() -> None:
    op.drop_table("data_version")
