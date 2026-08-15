import datetime
from pathlib import Path

import pytest

from valens import app, demo, limits
from valens.demo.profiles import PROFILES
from valens.models import RoutineActivity, Sex, User, Workout, WorkoutSet

TODAY = datetime.date(2002, 3, 12)


def test_run(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(app, "run", lambda x, y: None)
    demo.run(f"sqlite:///{tmp_path}/db")


def test_users_are_reproducible() -> None:
    users = demo.users(TODAY, seed=0)
    other_users = demo.users(TODAY, seed=0)

    assert values(users) == values(other_users)
    assert training(users) == training(other_users)


def test_users_are_not_created_after_given_date() -> None:
    users = demo.users(TODAY, seed=0)

    assert all(date <= TODAY for _, date, _ in values(users))
    assert all(
        TODAY - max(date for kind, date, _ in values([user]) if kind == "body_weight")
        <= datetime.timedelta(days=3)
        for user in users
    )


def test_period_is_only_created_for_female_users() -> None:
    users = demo.users(TODAY, seed=0)

    assert {user.sex for user in users} == {Sex.FEMALE, Sex.MALE}
    assert all(bool(user.period) == (user.sex == Sex.FEMALE) for user in users)


def test_exercises_have_valid_muscles() -> None:
    users = demo.users(TODAY, seed=0)

    for user in users:
        for exercise in user.exercises:
            assert exercise.muscles, exercise.name
            for muscle in exercise.muscles:
                assert muscle.muscle_id in limits.MUSCLE_IDS
                assert limits.STIMULUS_MIN <= muscle.stimulus <= limits.STIMULUS_MAX


def test_body_weight_is_plausible() -> None:
    users = demo.users(TODAY, seed=0)

    for user in users:
        assert user.body_weight
        assert user.height
        for body_weight in user.body_weight:
            bmi = body_weight.weight / (user.height / 100) ** 2
            assert 18 <= bmi <= 27, (user.name, body_weight.date, bmi)


def test_sets_belong_to_the_routine_of_the_session() -> None:
    users = demo.users(TODAY, seed=0)
    alternatives = {
        (profile.id, exercise.name): exercise.alternative
        for profile in PROFILES
        for exercise in profile.exercises
    }

    for user in users:
        for workout in user.workouts:
            expected = {
                name
                for part in routine_activities(workout)
                for name in (part.exercise.name, alternatives[(user.id, part.exercise.name)])
            }
            for element in workout.elements:
                if isinstance(element, WorkoutSet):
                    assert element.exercise.name in expected, (workout.date, element.exercise.name)


def test_sessions_are_scheduled() -> None:
    users = demo.users(TODAY, seed=0)
    weekdays = {profile.id: profile.schedule.weekdays for profile in PROFILES}

    for user in users:
        for workout in user.workouts:
            assert any(
                abs(workout.date.isoweekday() % 7 - weekday % 7) <= 1
                for weekday in weekdays[user.id]
            ), (user.name, workout.date)


def routine_activities(workout: Workout) -> list[RoutineActivity]:
    return [
        part
        for section in workout.routine.sections
        for part in section.parts
        if isinstance(part, RoutineActivity) and part.exercise is not None
    ]


def values(users: list[User]) -> list[tuple[str, datetime.date, float]]:
    return [
        *[("body_weight", b.date, b.weight) for user in users for b in user.body_weight],
        *[("body_fat", f.date, float(f.thigh or 0)) for user in users for f in user.body_fat],
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
