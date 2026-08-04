import datetime
from pathlib import Path

import pytest

from valens import app, demo
from valens.models import RoutineActivity, User, WorkoutSet


def test_run(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(app, "run", lambda x, y: None)
    demo.run(f"sqlite:///{tmp_path}/db")


def test_users_are_reproducible() -> None:
    today = datetime.date(2002, 3, 12)

    users = demo.users(today, seed=0)
    other_users = demo.users(today, seed=0)

    assert values(users) == values(other_users)
    assert training(users) == training(other_users)


def test_users_are_not_created_after_given_date() -> None:
    today = datetime.date(2002, 3, 12)

    users = demo.users(today, seed=0)

    assert all(date <= today for _, date, _ in values(users))
    assert max(date for kind, date, _ in values(users) if kind == "body_weight") == today


def values(users: list[User]) -> list[tuple[str, datetime.date, float]]:
    return [
        *[("body_weight", b.date, b.weight) for user in users for b in user.body_weight],
        *[("body_fat", f.date, float(f.chest or 0)) for user in users for f in user.body_fat],
        *[("period", p.date, float(p.intensity)) for user in users for p in user.period],
        *[("workout", w.date, float(len(w.elements))) for user in users for w in user.workouts],
    ]


def training(users: list[User]) -> list[tuple[object, ...]]:
    return [
        *[("exercise", user.id, e.name) for user in users for e in user.exercises],
        *[
            ("routine_activity", user.id, r.name, s.position, s.rounds, p.position, p.exercise.name)
            for user in users
            for r in user.routines
            for s in r.sections
            for p in s.parts
            if isinstance(p, RoutineActivity) and p.exercise is not None
        ],
        *[
            ("set", user.id, w.date, e.position, e.exercise.name, e.reps, e.time, e.weight, e.rpe)
            for user in users
            for w in user.workouts
            for e in w.elements
            if isinstance(e, WorkoutSet)
        ],
    ]
