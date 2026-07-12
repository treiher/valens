"""
Add role to user.

Revision ID: 9c4e2f8b6a1d
Revises: f8d2a6c4b7e1
Create Date: 2026-07-12

"""

import sqlalchemy as sa
from alembic import op

revision = "9c4e2f8b6a1d"
down_revision = "f8d2a6c4b7e1"
branch_labels = None
depends_on = None


def upgrade() -> None:
    with op.batch_alter_table("user", schema=None) as batch_op:
        batch_op.add_column(
            sa.Column(
                "role",
                sa.Enum("USER", "ADMIN", name="role"),
                nullable=False,
                server_default="USER",
            )
        )
    # Users existing before the introduction of roles keep full access to user management
    op.execute("UPDATE user SET role = 'ADMIN'")


def downgrade() -> None:
    with op.batch_alter_table("user", schema=None) as batch_op:
        batch_op.drop_column("role")
