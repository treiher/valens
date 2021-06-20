from __future__ import annotations

from flask import session
from sqlalchemy import select
from sqlalchemy.exc import NoResultFound

from valens import database as db
from valens.models import Exercise, Routine, Workout


def get_exercises() -> list[Exercise]:
    return (
        db.session.execute(select(Exercise).where(Exercise.user_id == session["user_id"]))
        .scalars()
        .all()
    )


def get_exercise(name: str) -> Exercise:
    return (
        db.session.execute(
            select(Exercise)
            .where(Exercise.user_id == session["user_id"])
            .where(Exercise.name == name)
        )
        .scalars()
        .one()
    )


def get_or_create_exercise(name: str) -> Exercise:
    try:
        exercise = get_exercise(name)
    except NoResultFound:
        exercise = Exercise(user_id=session["user_id"], name=name)
        db.session.add(exercise)
    return exercise


def get_routines() -> list[Routine]:
    return (
        db.session.execute(select(Routine).where(Routine.user_id == session["user_id"]))
        .scalars()
        .all()
    )


def get_routine(name: str) -> Routine:
    return (
        db.session.execute(
            select(Routine).where(Routine.user_id == session["user_id"]).where(Routine.name == name)
        )
        .scalars()
        .one()
    )


def get_or_create_routine(name: str) -> Routine:
    try:
        routine = get_routine(name)
    except NoResultFound:
        routine = Routine(user_id=session["user_id"], name=name)
        db.session.add(routine)
        db.session.commit()
    return routine


def get_workouts() -> list[Workout]:
    return (
        db.session.execute(select(Workout).where(Workout.user_id == session["user_id"]))
        .scalars()
        .all()
    )


def get_workout(workout_id: int) -> Workout:
    return (
        db.session.execute(
            select(Workout)
            .where(Workout.user_id == session["user_id"])
            .where(Workout.id == workout_id)
        )
        .scalars()
        .one()
    )
