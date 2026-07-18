"""
Add passkey and login link.

Revision ID: f3b9d17c5a2e
Revises: e2f1a8c40b7d
Create Date: 2026-07-18

"""

import sqlalchemy as sa
from alembic import op

revision = "f3b9d17c5a2e"
down_revision = "e2f1a8c40b7d"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.create_table(
        "passkey",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("user_id", sa.Integer(), nullable=False),
        sa.Column("credential_id", sa.LargeBinary(), nullable=False),
        sa.Column("public_key", sa.LargeBinary(), nullable=False),
        sa.Column("sign_count", sa.Integer(), nullable=False),
        sa.Column("transports", sa.String(), nullable=False),
        sa.Column("aaguid", sa.String(), nullable=False),
        sa.Column("label", sa.String(), nullable=False),
        sa.Column("created", sa.Date(), nullable=False),
        sa.Column("last_used", sa.Date(), nullable=True),
        sa.ForeignKeyConstraint(
            ["user_id"],
            ["user.id"],
            name=op.f("fk_passkey_user_id_user"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_passkey")),
        sa.UniqueConstraint("credential_id", name=op.f("uq_passkey_credential_id")),
    )
    op.create_table(
        "login_link",
        sa.Column("user_id", sa.Integer(), nullable=False),
        sa.Column("token_hash", sa.String(), nullable=False),
        sa.Column("expires_at", sa.DateTime(), nullable=False),
        sa.ForeignKeyConstraint(
            ["user_id"],
            ["user.id"],
            name=op.f("fk_login_link_user_id_user"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("user_id", name=op.f("pk_login_link")),
    )


def downgrade() -> None:
    op.drop_table("login_link")
    op.drop_table("passkey")
