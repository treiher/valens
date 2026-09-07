"""
Replace time by tempo in routine_activity and workout_set.

Revision ID: c7a4e93f6b15
Revises: d5e2b71c4a83
Create Date: 2026-09-08

"""

import sqlalchemy as sa
from alembic import op

revision = "c7a4e93f6b15"
down_revision = "d5e2b71c4a83"
branch_labels = None
depends_on = None


activity_constraints_up = [
    ("tempo_valid_json", "json_valid(tempo)"),
    ("tempo_type_array", "json_type(tempo) = 'array'"),
]
activity_constraints_down = [
    ("time_type_integer", "typeof(time) = 'integer'"),
    ("time_ge_0", "time >= 0"),
]
set_constraints_up = [
    ("target_tempo_valid_json_or_null", "json_valid(target_tempo) or target_tempo is null"),
    (
        "target_tempo_type_array_or_null",
        "json_type(target_tempo) = 'array' or target_tempo is null",
    ),
    ("target_tempo_length_gt_0", "json_array_length(target_tempo) > 0"),
]
set_constraints_down = [
    ("target_time_type_integer_or_null", "typeof(target_time) = 'integer' or target_time is null"),
    ("target_time_gt_0", "target_time > 0"),
]


def upgrade() -> None:
    with op.batch_alter_table("routine_activity") as batch_op:
        for constraint_name, _ in activity_constraints_down:
            batch_op.drop_constraint(f"ck_routine_activity_{constraint_name}")
        batch_op.add_column(sa.Column("tempo", sa.JSON(), nullable=True))
    op.execute("UPDATE routine_activity SET tempo = iif(time = 0, json_array(), json_array(time))")
    with op.batch_alter_table("routine_activity") as batch_op:
        batch_op.alter_column("tempo", existing_type=sa.JSON(), nullable=False)
        batch_op.drop_column("time")
        for constraint_name, condition in activity_constraints_up:
            batch_op.create_check_constraint(constraint_name, condition)

    with op.batch_alter_table("workout_set") as batch_op:
        for constraint_name, _ in set_constraints_down:
            batch_op.drop_constraint(f"ck_workout_set_{constraint_name}")
        batch_op.add_column(sa.Column("target_tempo", sa.JSON(), nullable=True))
    op.execute(
        "UPDATE workout_set SET target_tempo = json_array(target_time)"
        " WHERE target_time IS NOT NULL"
    )
    with op.batch_alter_table("workout_set") as batch_op:
        batch_op.drop_column("target_time")
        for constraint_name, condition in set_constraints_up:
            batch_op.create_check_constraint(constraint_name, condition)


def downgrade() -> None:
    with op.batch_alter_table("routine_activity") as batch_op:
        for constraint_name, _ in activity_constraints_up:
            batch_op.drop_constraint(f"ck_routine_activity_{constraint_name}")
        batch_op.add_column(sa.Column("time", sa.Integer(), nullable=True))
    op.execute(
        "UPDATE routine_activity SET time ="
        " (SELECT coalesce(sum(value), 0) FROM json_each(routine_activity.tempo))"
    )
    with op.batch_alter_table("routine_activity") as batch_op:
        batch_op.alter_column("time", existing_type=sa.Integer(), nullable=False)
        batch_op.drop_column("tempo")
        for constraint_name, condition in activity_constraints_down:
            batch_op.create_check_constraint(constraint_name, condition)

    with op.batch_alter_table("workout_set") as batch_op:
        for constraint_name, _ in set_constraints_up:
            batch_op.drop_constraint(f"ck_workout_set_{constraint_name}")
        batch_op.add_column(sa.Column("target_time", sa.Integer(), nullable=True))
    op.execute(
        "UPDATE workout_set SET target_time ="
        " (SELECT sum(value) FROM json_each(workout_set.target_tempo))"
        " WHERE target_tempo IS NOT NULL"
    )
    with op.batch_alter_table("workout_set") as batch_op:
        batch_op.drop_column("target_tempo")
        for constraint_name, condition in set_constraints_down:
            batch_op.create_check_constraint(constraint_name, condition)
