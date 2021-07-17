from datetime import timedelta
from pathlib import Path

PERMANENT_SESSION_LIFETIME = timedelta(weeks=52)
DATABASE = f"sqlite:///{Path.home()}/.config/valens/valens.db"
