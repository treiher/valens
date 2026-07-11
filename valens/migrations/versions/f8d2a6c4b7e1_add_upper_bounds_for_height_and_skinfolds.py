"""
Add upper bounds for height and skinfolds.

Revision ID: f8d2a6c4b7e1
Revises: d7a3c9e5f1b8
Create Date: 2026-07-11

"""

from alembic import op
from sqlalchemy import column

revision = "f8d2a6c4b7e1"
down_revision = "d7a3c9e5f1b8"
branch_labels = None
depends_on = None

CHECK_CONSTRAINTS = [
    ("user", "height"),
    ("body_fat", "chest"),
    ("body_fat", "abdominal"),
    ("body_fat", "thigh"),
    ("body_fat", "tricep"),
    ("body_fat", "subscapular"),
    ("body_fat", "suprailiac"),
    ("body_fat", "midaxillary"),
]


def upgrade() -> None:
    for table_name, column_name in CHECK_CONSTRAINTS:
        with op.batch_alter_table(table_name) as batch_op:
            batch_op.create_check_constraint(f"{column_name}_le_255", column(column_name) <= 255)


def downgrade() -> None:
    for table_name, column_name in CHECK_CONSTRAINTS:
        with op.batch_alter_table(table_name) as batch_op:
            batch_op.drop_constraint(f"ck_{table_name}_{column_name}_le_255")
