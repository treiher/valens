"""
Extend workouts.

Revision ID: 8a0dc258bf2a
Revises: b9f4e42c7135
Create Date: 2023-02-16

"""

from typing import Union

import sqlalchemy as sa
from alembic import op

revision = "8a0dc258bf2a"
down_revision = "b9f4e42c7135"
branch_labels = None
depends_on = None


workout_set = sa.table(
    "workout_set",
    sa.column("workout_id", sa.Integer),
    sa.column("position", sa.Integer),
    sa.column("exercise_id", sa.Integer),
    sa.column("reps", sa.Integer),
    sa.column("time", sa.Integer),
    sa.column("weight", sa.Float),
    sa.column("rpe", sa.Float),
)
workout_element = sa.table(
    "workout_element",
    sa.column("workout_id", sa.Integer),
    sa.column("position", sa.Integer),
    sa.column("type", sa.Text),
    sa.column("automatic", sa.Boolean),
)

check_constraints: list[tuple[str, Union[str, sa.ColumnElement[bool]]]] = [
    (
        "target_reps_type_integer_or_null",
        "typeof(target_reps) = 'integer' or typeof(target_reps) = 'null'",
    ),
    (
        "target_time_type_integer_or_null",
        "typeof(target_time) = 'integer' or typeof(target_time) = 'null'",
    ),
    (
        "target_weight_type_real_or_null",
        "typeof(target_weight) = 'real' or typeof(target_weight) = 'null'",
    ),
    ("target_rpe_type_real_or_null", "typeof(target_rpe) = 'real' or typeof(target_rpe) = 'null'"),
    ("target_reps_gt_0", sa.column("target_reps") > 0),
    ("target_time_gt_0", sa.column("target_time") > 0),
    ("target_weight_gt_0", sa.column("target_weight") > 0),
    ("target_rpe_ge_0", sa.column("target_rpe") >= 0),
    ("target_rpe_le_10", sa.column("target_rpe") <= 10),
]


def upgrade() -> None:
    op.create_table(
        "workout_element",
        sa.Column("workout_id", sa.Integer(), nullable=False),
        sa.Column("position", sa.Integer(), nullable=False),
        sa.Column("type", sa.String(), nullable=False),
        sa.Column("automatic", sa.Boolean(), nullable=False),
        sa.CheckConstraint(
            "typeof(position) = 'integer'", name=op.f("ck_workout_element_position_type_integer")
        ),
        sa.CheckConstraint("typeof(type) = 'text'", name=op.f("ck_workout_element_type_type_text")),
        sa.CheckConstraint(
            "typeof(automatic) = 'integer'", name=op.f("ck_workout_element_automatic_type_integer")
        ),
        sa.CheckConstraint("automatic <= 1", name=op.f("ck_workout_element_rpe_le_1")),
        sa.CheckConstraint("automatic >= 0", name=op.f("ck_workout_element_rpe_ge_0")),
        sa.CheckConstraint("position > 0", name=op.f("ck_workout_element_position_gt_0")),
        sa.ForeignKeyConstraint(
            ["workout_id"],
            ["workout.id"],
            name=op.f("fk_workout_element_workout_id_workout"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("workout_id", "position", name=op.f("pk_workout_element")),
    )

    connection = op.get_bind()

    for ws in connection.execute(workout_set.select()):
        connection.execute(
            workout_element.insert().values(
                workout_id=ws.workout_id,
                position=ws.position,
                type="set",
                automatic=False,
            )
        )

    op.create_table(
        "workout_rest",
        sa.Column("workout_id", sa.Integer(), nullable=False),
        sa.Column("position", sa.Integer(), nullable=False),
        sa.Column("target_time", sa.Integer(), nullable=True),
        sa.CheckConstraint(
            "typeof(position) = 'integer'", name=op.f("ck_workout_rest_position_type_integer")
        ),
        sa.CheckConstraint(
            "typeof(target_time) = 'integer' or typeof(target_time) = 'null'",
            name=op.f("ck_workout_rest_target_time_type_integer_or_null"),
        ),
        sa.CheckConstraint("position > 0", name=op.f("ck_workout_rest_position_gt_0")),
        sa.CheckConstraint("target_time > 0", name=op.f("ck_workout_rest_target_time_gt_0")),
        sa.ForeignKeyConstraint(
            ["workout_id", "position"],
            ["workout_element.workout_id", "workout_element.position"],
            name=op.f("fk_workout_rest_workout_id_workout_element"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("workout_id", "position", name=op.f("pk_workout_rest")),
    )

    with op.batch_alter_table("workout_set", schema=None) as batch_op:
        batch_op.add_column(sa.Column("target_reps", sa.Integer(), nullable=True))
        batch_op.add_column(sa.Column("target_time", sa.Integer(), nullable=True))
        batch_op.add_column(sa.Column("target_weight", sa.Float(), nullable=True))
        batch_op.add_column(sa.Column("target_rpe", sa.Float(), nullable=True))
        batch_op.drop_constraint("fk_workout_set_workout_id_workout", type_="foreignkey")
        batch_op.create_foreign_key(
            batch_op.f("fk_workout_set_workout_id_workout_element"),
            "workout_element",
            ["workout_id", "position"],
            ["workout_id", "position"],
            ondelete="CASCADE",
        )
        for constraint_name, condition in check_constraints:
            batch_op.create_check_constraint(constraint_name, condition)


def downgrade() -> None:
    with op.batch_alter_table("workout_set", schema=None) as batch_op:
        batch_op.drop_constraint(
            batch_op.f("fk_workout_set_workout_id_workout_element"), type_="foreignkey"
        )
        batch_op.create_foreign_key(
            "fk_workout_set_workout_id_workout",
            "workout",
            ["workout_id"],
            ["id"],
            ondelete="CASCADE",
        )
        for constraint_name, _ in check_constraints:
            batch_op.drop_constraint(constraint_name, type_="check")
        batch_op.drop_column("target_rpe")
        batch_op.drop_column("target_weight")
        batch_op.drop_column("target_time")
        batch_op.drop_column("target_reps")

    op.drop_table("workout_rest")
    op.drop_table("workout_element")
