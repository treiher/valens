"""Add foreign key ON DELETE cascade

Revision ID: 3e7317eb4efa
Revises: 4cacd61cb0c5
Create Date: 2021-07-18

"""
from alembic import op
from sqlalchemy import CheckConstraint, column

revision = "3e7317eb4efa"
down_revision = "4cacd61cb0c5"
branch_labels = None
depends_on = None


FOREIGN_KEY_CONSTRAINTS = [
    (
        "fk_body_weight_user_id_user",
        "body_weight",
        "user",
        "user_id",
        [
            CheckConstraint(column("weight") > 0, name="ck_body_weight_weight_gt_0"),
        ],
    ),
    (
        "fk_body_fat_user_id_user",
        "body_fat",
        "user",
        "user_id",
        [
            CheckConstraint(column("chest") > 0, name="ck_body_fat_chest_gt_0"),
            CheckConstraint(column("abdominal") > 0, name="ck_body_fat_abdominal_gt_0"),
            CheckConstraint(column("tigh") > 0, name="ck_body_fat_tigh_gt_0"),
            CheckConstraint(column("tricep") > 0, name="ck_body_fat_tricep_gt_0"),
            CheckConstraint(column("subscapular") > 0, name="ck_body_fat_subscapular_gt_0"),
            CheckConstraint(column("suprailiac") > 0, name="ck_body_fat_suprailiac_gt_0"),
            CheckConstraint(column("midaxillary") > 0, name="ck_body_fat_midaxillary_gt_0"),
        ],
    ),
    (
        "fk_period_user_id_user",
        "period",
        "user",
        "user_id",
        [
            CheckConstraint(column("intensity") >= 1, name="ck_period_intensity_ge_1"),
            CheckConstraint(column("intensity") <= 4, name="ck_period_intensity_le_4"),
        ],
    ),
    (
        "fk_routine_exercise_routine_id_routine",
        "routine_exercise",
        "routine",
        "routine_id",
        [
            CheckConstraint(column("position") > 0, name="ck_routine_exercise_position_gt_0"),
            CheckConstraint(column("sets") > 0, name="ck_routine_exercise_sets_gt_0"),
        ],
    ),
    (
        "fk_routine_exercise_exercise_id_exercise",
        "routine_exercise",
        "exercise",
        "exercise_id",
        [
            CheckConstraint(column("position") > 0, name="ck_routine_exercise_position_gt_0"),
            CheckConstraint(column("sets") > 0, name="ck_routine_exercise_sets_gt_0"),
        ],
    ),
    (
        "fk_workout_set_workout_id_workout",
        "workout_set",
        "workout",
        "workout_id",
        [
            CheckConstraint(column("position") > 0, name="ck_workout_set_position_gt_0"),
            CheckConstraint(column("reps") > 0, name="ck_workout_set_reps_gt_0"),
            CheckConstraint(column("time") > 0, name="ck_workout_set_time_gt_0"),
            CheckConstraint(column("weight") > 0, name="ck_workout_set_weight_gt_0"),
            CheckConstraint(column("rpe") >= 0, name="ck_workout_set_rpe_ge_0"),
            CheckConstraint(column("rpe") <= 10, name="ck_workout_set_rpe_le_10"),
        ],
    ),
    (
        "fk_workout_set_exercise_id_exercise",
        "workout_set",
        "exercise",
        "exercise_id",
        [
            CheckConstraint(column("position") > 0, name="ck_workout_set_position_gt_0"),
            CheckConstraint(column("reps") > 0, name="ck_workout_set_reps_gt_0"),
            CheckConstraint(column("time") > 0, name="ck_workout_set_time_gt_0"),
            CheckConstraint(column("weight") > 0, name="ck_workout_set_weight_gt_0"),
            CheckConstraint(column("rpe") >= 0, name="ck_workout_set_rpe_ge_0"),
            CheckConstraint(column("rpe") <= 10, name="ck_workout_set_rpe_le_10"),
        ],
    ),
    ("fk_exercise_user_id_user", "exercise", "user", "user_id", []),
    ("fk_workout_user_id_user", "workout", "user", "user_id", []),
    ("fk_workout_routine_id_routine", "workout", "routine", "routine_id", []),
    ("fk_routine_user_id_user", "routine", "user", "user_id", []),
]


def upgrade() -> None:
    for (
        constraint_name,
        source_table,
        referent_table,
        local_col,
        table_args,
    ) in FOREIGN_KEY_CONSTRAINTS:
        with op.batch_alter_table(source_table, schema=None, table_args=table_args) as batch_op:  # type: ignore[no-untyped-call]
            batch_op.drop_constraint(constraint_name, type_="foreignkey")
            batch_op.create_foreign_key(
                batch_op.f(constraint_name), referent_table, [local_col], ["id"], ondelete="CASCADE"
            )


def downgrade() -> None:
    for (
        constraint_name,
        source_table,
        referent_table,
        local_col,
        table_args,
    ) in FOREIGN_KEY_CONSTRAINTS:
        with op.batch_alter_table(source_table, schema=None, table_args=table_args) as batch_op:  # type: ignore[no-untyped-call]
            batch_op.drop_constraint(constraint_name, type_="foreignkey")
            batch_op.create_foreign_key(
                batch_op.f(constraint_name), referent_table, [local_col], ["id"]
            )
