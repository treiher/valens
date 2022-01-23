from logging.config import fileConfig

from alembic import context

from valens import app, database as db, models

config = context.config  # pylint: disable = no-member

if config.config_file_name:
    fileConfig(config.config_file_name)

TARGET_METADATA = models.Base.metadata

assert not context.is_offline_mode()  # pylint: disable = no-member

with app.app_context():
    # Prevent integrity errors and data loss caused by ON DELETE cascades
    app.config["SQLITE_FOREIGN_KEY_SUPPORT"] = False

    connectable = context.config.attributes.get("connection", None)  # pylint: disable = no-member

    if connectable is None:
        connectable = db.get_engine()

    with connectable.connect() as connection:
        context.configure(  # pylint: disable = no-member
            connection=connection,
            target_metadata=TARGET_METADATA,
            render_as_batch=True,
        )

        with context.begin_transaction():  # pylint: disable = no-member
            context.run_migrations()  # pylint: disable = no-member

    app.config["SQLITE_FOREIGN_KEY_SUPPORT"] = True
