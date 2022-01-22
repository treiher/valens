"""Add type checking

Revision ID: 4b6051594962
Revises: 3e7317eb4efa
Create Date: 2022-01-22

"""
from alembic import op
from sqlalchemy import CheckConstraint, column

revision = "4b6051594962"
down_revision = "3e7317eb4efa"
branch_labels = None
depends_on = None

CHECK_CONSTRAINTS = [
    (
        "weight_type_real",
        "body_weight",
        "typeof(weight) = 'real'",
    ),
    (
        "chest_type_integer_or_null",
        "body_fat",
        "typeof(chest) = 'integer' or typeof(chest) = 'null'",
    ),
    (
        "abdominal_type_integer_or_null",
        "body_fat",
        "typeof(abdominal) = 'integer' or typeof(abdominal) = 'null'",
    ),
    (
        "tigh_type_integer_or_null",
        "body_fat",
        "typeof(tigh) = 'integer' or typeof(tigh) = 'null'",
    ),
    (
        "tricep_type_integer_or_null",
        "body_fat",
        "typeof(tricep) = 'integer' or typeof(tricep) = 'null'",
    ),
    (
        "subscapular_type_integer_or_null",
        "body_fat",
        "typeof(subscapular) = 'integer' or typeof(subscapular) = 'null'",
    ),
    (
        "suprailiac_type_integer_or_null",
        "body_fat",
        "typeof(suprailiac) = 'integer' or typeof(suprailiac) = 'null'",
    ),
    (
        "midaxillary_type_integer_or_null",
        "body_fat",
        "typeof(midaxillary) = 'integer' or typeof(midaxillary) = 'null'",
    ),
    (
        "intensity_type_integer",
        "period",
        "typeof(intensity) = 'integer'",
    ),
    (
        "position_type_integer",
        "routine_exercise",
        "typeof(position) = 'integer'",
    ),
    (
        "sets_type_integer",
        "routine_exercise",
        "typeof(sets) = 'integer'",
    ),
    (
        "position_type_integer",
        "workout_set",
        "typeof(position) = 'integer'",
    ),
    (
        "reps_type_integer_or_null",
        "workout_set",
        "typeof(reps) = 'integer' or typeof(reps) = 'null'",
    ),
    (
        "time_type_integer_or_null",
        "workout_set",
        "typeof(time) = 'integer' or typeof(time) = 'null'",
    ),
    (
        "weight_type_real_or_null",
        "workout_set",
        "typeof(weight) = 'real' or typeof(weight) = 'null'",
    ),
    (
        "rpe_type_real_or_null",
        "workout_set",
        "typeof(rpe) = 'real' or typeof(rpe) = 'null'",
    ),
]


def upgrade() -> None:
    for constraint_name, table_name, condition in CHECK_CONSTRAINTS:
        with op.batch_alter_table(table_name) as batch_op:  # type: ignore[no-untyped-call]
            batch_op.create_check_constraint(constraint_name, condition)


def downgrade() -> None:
    for constraint_name, table_name, condition in CHECK_CONSTRAINTS:
        with op.batch_alter_table(table_name) as batch_op:  # type: ignore[no-untyped-call]
            batch_op.drop_constraint(f"ck_{table_name}_{constraint_name}")
