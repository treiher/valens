from logging.config import fileConfig

from alembic import context

from valens import app, database as db, models

config = context.config

if config.config_file_name:
    fileConfig(config.config_file_name)

TARGET_METADATA = models.Base.metadata

assert not context.is_offline_mode()

with app.app_context():
    # Prevent integrity errors and data loss caused by ON DELETE cascades
    app.config["SQLITE_FOREIGN_KEY_SUPPORT"] = False

    connectable = context.config.attributes.get("connection", None)

    if connectable is None:
        connectable = db.get_engine()

    with connectable.connect() as connection:
        context.configure(
            connection=connection,
            target_metadata=TARGET_METADATA,
            render_as_batch=True,
        )

        with context.begin_transaction():
            context.run_migrations()

    app.config["SQLITE_FOREIGN_KEY_SUPPORT"] = True
