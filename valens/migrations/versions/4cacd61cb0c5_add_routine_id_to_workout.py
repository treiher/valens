"""Add routine_id to workout

Revision ID: 4cacd61cb0c5
Revises:
Create Date: 2021-07-03 21:19:40.634728

"""
import sqlalchemy as sa
from alembic import op

revision = "4cacd61cb0c5"
down_revision = None
branch_labels = None
depends_on = None


def upgrade() -> None:
    with op.batch_alter_table("workout", schema=None) as batch_op:  # type: ignore[no-untyped-call]
        batch_op.add_column(sa.Column("routine_id", sa.Integer(), nullable=True))
        batch_op.create_foreign_key(
            batch_op.f("fk_workout_routine_id_routine"), "routine", ["routine_id"], ["id"]
        )


def downgrade() -> None:
    with op.batch_alter_table("workout", schema=None) as batch_op:  # type: ignore[no-untyped-call]
        batch_op.drop_constraint(batch_op.f("fk_workout_routine_id_routine"), type_="foreignkey")
        batch_op.drop_column("routine_id")
