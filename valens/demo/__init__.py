from __future__ import annotations

import datetime
import random

from valens import app, database as db
from valens.demo import health, training
from valens.demo.profiles import PROFILES, Profile
from valens.models import User


def run(
    database: str,
    host: str = "127.0.0.1",
    port: int = 5000,
    today: datetime.date | None = None,
    seed: int | None = None,
) -> None:
    app.config["DATABASE"] = database
    app.config["SECRET_KEY"] = b"TEST_KEY"
    app.config["PUBLIC_URL"] = f"http://localhost:{port}"
    with app.app_context():
        for user in users(today, seed):
            db.session.add(user)
        db.session.commit()
        app.run(host, port)


def users(today: datetime.date | None = None, seed: int | None = None) -> list[User]:
    """Create example data, which is reproducible for the same `today` and `seed`."""
    if today is None:
        today = datetime.date.today()

    rng = random.Random(seed)

    # Each profile is generated from its own generator, so that a change to one of them does not
    # affect the data of the others
    return [_user(profile, today, random.Random(rng.randrange(2**32))) for profile in PROFILES]


def _user(profile: Profile, today: datetime.date, rng: random.Random) -> User:
    data = training.training(profile, today, rng)
    return User(
        id=profile.id,
        name=profile.name,
        sex=profile.sex,
        height=profile.height,
        role=profile.role,
        body_weight=health.body_weight(profile, today, rng),
        body_fat=health.body_fat(profile, today, rng),
        period=health.period(profile, today, rng),
        exercises=data.exercises,
        routines=data.routines,
        workouts=data.workouts,
        schedule_rotations=data.schedule_rotations,
        schedule_slots=data.schedule_slots,
    )
