"""
Add routine sections.

Revision ID: b9f4e42c7135
Revises: 4b6051594962
Create Date: 2022-11-13

"""

import sqlalchemy as sa
from alembic import op
from sqlalchemy import Boolean, Integer, String
from sqlalchemy.sql import column, table

revision = "b9f4e42c7135"
down_revision = "4b6051594962"
branch_labels = None
depends_on = None


routine_exercise = table(
    "routine_exercise",
    column("routine_id", Integer),
    column("position", Integer),
    column("exercise_id", Integer),
    column("sets", Integer),
)
routine_part = table(
    "routine_part",
    column("id", Integer),
    column("type", String),
    column("routine_section_id", Integer),
    column("position", Integer),
)
routine_section = table(
    "routine_section",
    column("id", Integer),
    column("routine_id", Integer),
    column("rounds", Integer),
)
routine_activity = table(
    "routine_activity",
    column("id", Integer),
    column("exercise_id", Integer),
    column("duration", Integer),
    column("tempo", Integer),
    column("automatic", Boolean),
)


def upgrade() -> None:
    op.create_table(
        "routine_part",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("type", sa.String(), nullable=False),
        sa.Column("routine_section_id", sa.Integer(), nullable=True),
        sa.Column("position", sa.Integer(), nullable=False),
        sa.CheckConstraint(
            "typeof(position) = 'integer'", name=op.f("ck_routine_part_position_type_integer")
        ),
        sa.CheckConstraint("typeof(type) = 'text'", name=op.f("ck_routine_part_type_type_text")),
        sa.CheckConstraint("position > 0", name=op.f("ck_routine_part_position_gt_0")),
        sa.ForeignKeyConstraint(
            ["routine_section_id"],
            ["routine_section.id"],
            name=op.f("fk_routine_part_routine_section_id_routine_section"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_routine_part")),
    )
    op.create_table(
        "routine_section",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("routine_id", sa.Integer(), nullable=True),
        sa.Column("rounds", sa.Integer(), nullable=False),
        sa.CheckConstraint(
            "typeof(rounds) = 'integer'", name=op.f("ck_routine_section_rounds_type_integer")
        ),
        sa.CheckConstraint("rounds > 0", name=op.f("ck_routine_section_rounds_gt_0")),
        sa.ForeignKeyConstraint(
            ["id"], ["routine_part.id"], name=op.f("fk_routine_section_id_routine_part")
        ),
        sa.ForeignKeyConstraint(
            ["routine_id"],
            ["routine.id"],
            name=op.f("fk_routine_section_routine_id_routine"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_routine_section")),
    )
    op.create_table(
        "routine_activity",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("exercise_id", sa.Integer(), nullable=True),
        sa.Column("duration", sa.Integer(), nullable=False),
        sa.Column("tempo", sa.Integer(), nullable=False),
        sa.Column("automatic", sa.Boolean(), nullable=False),
        sa.CheckConstraint(
            "typeof(duration) = 'integer'", name=op.f("ck_routine_activity_duration_type_integer")
        ),
        sa.CheckConstraint("duration >= 0", name=op.f("ck_routine_activity_duration_ge_0")),
        sa.CheckConstraint(
            "typeof(tempo) = 'integer'", name=op.f("ck_routine_activity_tempo_type_integer")
        ),
        sa.CheckConstraint("tempo >= 0", name=op.f("ck_routine_activity_tempo_ge_0")),
        sa.ForeignKeyConstraint(
            ["id"], ["routine_part.id"], name=op.f("fk_routine_activity_id_routine_part")
        ),
        sa.ForeignKeyConstraint(
            ["exercise_id"],
            ["exercise.id"],
            name=op.f("fk_routine_activity_exercise_id_exercise"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_routine_activity")),
    )

    connection = op.get_bind()

    part_id = 1

    for exercise in connection.execute(routine_exercise.select()):
        connection.execute(
            routine_part.insert().values(
                id=part_id, type="routine_section", position=exercise.position
            )
        )
        connection.execute(
            routine_section.insert().values(
                id=part_id, routine_id=exercise.routine_id, rounds=exercise.sets
            )
        )

        connection.execute(
            routine_part.insert().values(
                id=part_id + 1, type="routine_activity", routine_section_id=part_id, position=1
            )
        )
        connection.execute(
            routine_activity.insert().values(
                id=part_id + 1,
                exercise_id=exercise.exercise_id,
                duration=0,
                tempo=0,
                automatic=False,
            )
        )
        connection.execute(
            routine_part.insert().values(
                id=part_id + 2, type="routine_activity", routine_section_id=part_id, position=1
            )
        )
        connection.execute(
            routine_activity.insert().values(
                id=part_id + 2, exercise_id=None, duration=60, tempo=0, automatic=True
            )
        )

        part_id += 3

    op.drop_table("routine_exercise")


def downgrade() -> None:
    op.create_table(
        "routine_exercise",
        sa.Column("routine_id", sa.INTEGER(), nullable=False),
        sa.Column("position", sa.INTEGER(), nullable=False),
        sa.Column("exercise_id", sa.INTEGER(), nullable=False),
        sa.Column("sets", sa.INTEGER(), nullable=False),
        sa.CheckConstraint("typeof(position) = 'integer'", name="position_type_integer"),
        sa.CheckConstraint("typeof(sets) = 'integer'", name="sets_type_integer"),
        sa.CheckConstraint("position > 0", name="position_gt_0"),
        sa.CheckConstraint("sets > 0", name="sets_gt_0"),
        sa.ForeignKeyConstraint(
            ["routine_id"],
            ["routine.id"],
            name="fk_routine_exercise_routine_id_routine",
            ondelete="CASCADE",
        ),
        sa.ForeignKeyConstraint(
            ["exercise_id"],
            ["exercise.id"],
            name="fk_routine_exercise_exercise_id_exercise",
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("routine_id", "position", name="pk_routine_exercise"),
    )

    connection = op.get_bind()

    for section in connection.execute(
        routine_part.join(routine_section, routine_part.c.id == routine_section.c.id).select()
    ):
        for activity in connection.execute(
            routine_part.join(routine_activity, routine_part.c.id == routine_activity.c.id)
            .select()
            .where(routine_part.c.routine_section_id == section.id)
            .where(routine_activity.c.exercise_id != None)  # noqa: E711
        ):
            if section.routine_id is None:
                continue
            connection.execute(
                routine_exercise.insert().values(
                    routine_id=section.routine_id,
                    position=section.position,
                    exercise_id=activity.exercise_id,
                    sets=section.rounds,
                )
            )

    op.drop_table("routine_activity")
    op.drop_table("routine_section")
    op.drop_table("routine_part")
