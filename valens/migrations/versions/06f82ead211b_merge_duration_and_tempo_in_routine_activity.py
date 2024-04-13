"""
Merge duration and tempo in routine_activity.

Revision ID: 06f82ead211b
Revises: 22f3ddb25741
Create Date: 2023-04-02

"""

import sqlalchemy as sa
from alembic import op

revision = "06f82ead211b"
down_revision = "22f3ddb25741"
branch_labels = None
depends_on = None


check_constraints_up = [
    ("time_type_integer", "typeof(time) = 'integer'"),
    ("time_ge_0", "time >= 0"),
]
check_constraints_down = [
    ("duration_type_integer", "typeof(duration) = 'integer'"),
    ("tempo_type_integer", "typeof(tempo) = 'integer'"),
    ("duration_ge_0", "duration >= 0"),
    ("tempo_ge_0", "tempo >= 0"),
]


def upgrade() -> None:
    with op.batch_alter_table("routine_activity", schema=None) as batch_op:
        for constraint_name, _ in check_constraints_down:
            batch_op.drop_constraint(constraint_name, type_="check")

        batch_op.alter_column("duration", new_column_name="time")
        batch_op.drop_column("tempo")

        for constraint_name, condition in check_constraints_up:
            batch_op.create_check_constraint(constraint_name, condition)


def downgrade() -> None:
    with op.batch_alter_table("routine_activity", schema=None) as batch_op:
        for constraint_name, _ in check_constraints_up:
            batch_op.drop_constraint(constraint_name, type_="check")

        batch_op.alter_column("time", new_column_name="duration")
        batch_op.add_column(sa.Column("tempo", sa.INTEGER(), nullable=False, default=0))

        for constraint_name, condition in check_constraints_down:
            batch_op.create_check_constraint(constraint_name, condition)
