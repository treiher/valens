"""
Add properties of exercise.

Revision ID: c3f7a1d94b26
Revises: a1c8e5b2d947
Create Date: 2026-08-28

"""

import sqlalchemy as sa
from alembic import op

revision = "c3f7a1d94b26"
down_revision = "a1c8e5b2d947"
branch_labels = None
depends_on = None

# Upper bounds mirror the exercise properties in `crates/domain/src/exercise.rs`
COLUMNS = {"force": 3, "mechanic": 2, "laterality": 2, "assistance": 2, "category": 2}


def upgrade() -> None:
    with op.batch_alter_table("exercise") as batch_op:
        for column_name, upper_bound in COLUMNS.items():
            batch_op.add_column(sa.Column(column_name, sa.Integer(), nullable=True))
            batch_op.create_check_constraint(
                f"{column_name}_type_integer_or_null",
                f"typeof({column_name}) = 'integer' or typeof({column_name}) = 'null'",
            )
            batch_op.create_check_constraint(f"{column_name}_ge_1", f"{column_name} >= 1")
            batch_op.create_check_constraint(
                f"{column_name}_le_{upper_bound}", f"{column_name} <= {upper_bound}"
            )

    op.create_table(
        "exercise_equipment",
        sa.Column("user_id", sa.Integer(), nullable=False),
        sa.Column("exercise_id", sa.Integer(), nullable=False),
        sa.Column("equipment", sa.Integer(), nullable=False),
        sa.CheckConstraint(
            "typeof(equipment) = 'integer'", name=op.f("ck_exercise_equipment_equipment_integer")
        ),
        sa.CheckConstraint("equipment <= 13", name=op.f("ck_exercise_equipment_equipment_le_13")),
        sa.CheckConstraint("equipment >= 1", name=op.f("ck_exercise_equipment_equipment_ge_1")),
        sa.ForeignKeyConstraint(
            ["exercise_id"],
            ["exercise.id"],
            name=op.f("fk_exercise_equipment_exercise_id_exercise"),
            ondelete="CASCADE",
        ),
        sa.ForeignKeyConstraint(
            ["user_id"],
            ["user.id"],
            name=op.f("fk_exercise_equipment_user_id_user"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint(
            "user_id", "exercise_id", "equipment", name=op.f("pk_exercise_equipment")
        ),
        sa.UniqueConstraint(
            "user_id", "exercise_id", "equipment", name=op.f("uq_exercise_equipment_user_id")
        ),
    )


def downgrade() -> None:
    op.drop_table("exercise_equipment")

    with op.batch_alter_table("exercise") as batch_op:
        for column_name, upper_bound in COLUMNS.items():
            batch_op.drop_constraint(f"ck_exercise_{column_name}_le_{upper_bound}")
            batch_op.drop_constraint(f"ck_exercise_{column_name}_ge_1")
            batch_op.drop_constraint(f"ck_exercise_{column_name}_type_integer_or_null")
            batch_op.drop_column(column_name)
