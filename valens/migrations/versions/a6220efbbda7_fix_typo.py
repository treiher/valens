"""
Fix typo.

Revision ID: a6220efbbda7
Revises: 743cea459efa
Create Date: 2024-05-05

"""

from alembic import op

revision = "a6220efbbda7"
down_revision = "743cea459efa"
branch_labels = None
depends_on = None


def upgrade() -> None:
    with op.batch_alter_table("body_fat", schema=None) as batch_op:
        batch_op.drop_constraint("tigh_type_integer_or_null", type_="check")
        batch_op.drop_constraint("tigh_gt_0", type_="check")
        batch_op.alter_column("tigh", new_column_name="thigh")
        batch_op.create_check_constraint(
            "thigh_type_integer_or_null", "typeof(thigh) = 'integer' or typeof(thigh) = 'null'"
        )
        batch_op.create_check_constraint("thigh_gt_0", "thigh > 0")


def downgrade() -> None:
    with op.batch_alter_table("body_fat", schema=None) as batch_op:
        batch_op.drop_constraint("thigh_type_integer_or_null", type_="check")
        batch_op.drop_constraint("thigh_gt_0", type_="check")
        batch_op.alter_column("thigh", new_column_name="tigh")
        batch_op.create_check_constraint(
            "tigh_type_integer_or_null", "typeof(tigh) = 'integer' or typeof(tigh) = 'null'"
        )
        batch_op.create_check_constraint("tigh_gt_0", "tigh > 0")
