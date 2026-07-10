"""
Add schedule.

Revision ID: d7a3c9e5f1b8
Revises: c5e1a9f7d3b2
Create Date: 2026-07-04

"""

import sqlalchemy as sa
from alembic import op

revision = "d7a3c9e5f1b8"
down_revision = "c5e1a9f7d3b2"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.create_table(
        "schedule_rotation",
        sa.Column("user_id", sa.Integer(), nullable=False),
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("name", sa.String(), nullable=False),
        sa.ForeignKeyConstraint(
            ["user_id"],
            ["user.id"],
            name=op.f("fk_schedule_rotation_user_id_user"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("user_id", "id", name=op.f("pk_schedule_rotation")),
        sa.UniqueConstraint("user_id", "name", name=op.f("uq_schedule_rotation_user_id")),
    )
    op.create_table(
        "schedule_rotation_routine",
        sa.Column("user_id", sa.Integer(), nullable=False),
        sa.Column("rotation_id", sa.Integer(), nullable=False),
        sa.Column("position", sa.Integer(), nullable=False),
        sa.Column("routine_id", sa.Integer(), nullable=False),
        sa.CheckConstraint(
            "typeof(position) = 'integer'",
            name=op.f("ck_schedule_rotation_routine_position_type_integer"),
        ),
        sa.CheckConstraint("position > 0", name=op.f("ck_schedule_rotation_routine_position_gt_0")),
        sa.ForeignKeyConstraint(
            ["user_id", "rotation_id"],
            ["schedule_rotation.user_id", "schedule_rotation.id"],
            name=op.f("fk_schedule_rotation_routine_user_id_schedule_rotation"),
            ondelete="CASCADE",
        ),
        sa.ForeignKeyConstraint(
            ["routine_id"],
            ["routine.id"],
            name=op.f("fk_schedule_rotation_routine_routine_id_routine"),
        ),
        sa.PrimaryKeyConstraint(
            "user_id", "rotation_id", "position", name=op.f("pk_schedule_rotation_routine")
        ),
    )
    op.create_table(
        "schedule_slot",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("user_id", sa.Integer(), nullable=False),
        sa.Column("weekday", sa.Integer(), nullable=False),
        sa.Column("position", sa.Integer(), nullable=False),
        sa.Column("routine_id", sa.Integer(), nullable=True),
        sa.Column("rotation_id", sa.Integer(), nullable=True),
        sa.CheckConstraint(
            "typeof(weekday) = 'integer'", name=op.f("ck_schedule_slot_weekday_type_integer")
        ),
        sa.CheckConstraint("weekday >= 1", name=op.f("ck_schedule_slot_weekday_ge_1")),
        sa.CheckConstraint("weekday <= 7", name=op.f("ck_schedule_slot_weekday_le_7")),
        sa.CheckConstraint(
            "typeof(position) = 'integer'", name=op.f("ck_schedule_slot_position_type_integer")
        ),
        sa.CheckConstraint("position > 0", name=op.f("ck_schedule_slot_position_gt_0")),
        sa.CheckConstraint(
            "(routine_id IS NULL) != (rotation_id IS NULL)",
            name=op.f("ck_schedule_slot_routine_xor_rotation"),
        ),
        sa.ForeignKeyConstraint(
            ["user_id", "rotation_id"],
            ["schedule_rotation.user_id", "schedule_rotation.id"],
            name=op.f("fk_schedule_slot_user_id_schedule_rotation"),
            ondelete="CASCADE",
        ),
        sa.ForeignKeyConstraint(
            ["routine_id"],
            ["routine.id"],
            name=op.f("fk_schedule_slot_routine_id_routine"),
        ),
        sa.ForeignKeyConstraint(
            ["user_id"],
            ["user.id"],
            name=op.f("fk_schedule_slot_user_id_user"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_schedule_slot")),
        sa.UniqueConstraint(
            "user_id", "weekday", "position", name=op.f("uq_schedule_slot_user_id")
        ),
    )


def downgrade() -> None:
    op.drop_table("schedule_slot")
    op.drop_table("schedule_rotation_routine")
    op.drop_table("schedule_rotation")
