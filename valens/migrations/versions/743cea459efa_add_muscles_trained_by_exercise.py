"""
Add muscles trained by exercise.

Revision ID: 743cea459efa
Revises: 06f82ead211b
Create Date: 2024-04-22

"""

import sqlalchemy as sa
from alembic import op

revision = "743cea459efa"
down_revision = "06f82ead211b"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.create_table(
        "exercise_muscle",
        sa.Column("user_id", sa.Integer(), nullable=False),
        sa.Column("exercise_id", sa.Integer(), nullable=False),
        sa.Column("muscle_id", sa.Integer(), nullable=False),
        sa.Column("stimulus", sa.Integer(), nullable=False),
        sa.CheckConstraint(
            "typeof(muscle_id) = 'integer'", name=op.f("ck_exercise_muscle_muscle_id_integer")
        ),
        sa.CheckConstraint(
            "typeof(stimulus) = 'integer'", name=op.f("ck_exercise_muscle_stimulus_integer")
        ),
        sa.CheckConstraint("stimulus <= 100", name=op.f("ck_exercise_muscle_stimulus_le_100")),
        sa.CheckConstraint("stimulus >= 1", name=op.f("ck_exercise_muscle_stimulus_ge_1")),
        sa.ForeignKeyConstraint(
            ["exercise_id"],
            ["exercise.id"],
            name=op.f("fk_exercise_muscle_exercise_id_exercise"),
            ondelete="CASCADE",
        ),
        sa.ForeignKeyConstraint(
            ["user_id"],
            ["user.id"],
            name=op.f("fk_exercise_muscle_user_id_user"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint(
            "user_id", "exercise_id", "muscle_id", name=op.f("pk_exercise_muscle")
        ),
        sa.UniqueConstraint(
            "user_id", "exercise_id", "muscle_id", name=op.f("uq_exercise_muscle_user_id")
        ),
    )


def downgrade() -> None:
    op.drop_table("exercise_muscle")
