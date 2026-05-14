"""
Add workout_exercise_note.

Revision ID: b4f3a8c1d2e5
Revises: e4bc50b8ecb9
Create Date: 2026-05-14

"""

import sqlalchemy as sa
from alembic import op

revision = "b4f3a8c1d2e5"
down_revision = "e4bc50b8ecb9"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.create_table(
        "workout_exercise_note",
        sa.Column("workout_id", sa.Integer(), nullable=False),
        sa.Column("exercise_id", sa.Integer(), nullable=False),
        sa.Column("notes", sa.String(), nullable=False),
        sa.ForeignKeyConstraint(
            ["exercise_id"],
            ["exercise.id"],
            name=op.f("fk_workout_exercise_note_exercise_id_exercise"),
            ondelete="CASCADE",
        ),
        sa.ForeignKeyConstraint(
            ["workout_id"],
            ["workout.id"],
            name=op.f("fk_workout_exercise_note_workout_id_workout"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("workout_id", "exercise_id", name=op.f("pk_workout_exercise_note")),
    )


def downgrade() -> None:
    op.drop_table("workout_exercise_note")
