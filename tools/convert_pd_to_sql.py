#!/usr/bin/env python3

from pathlib import Path
from typing import Sequence

import pandas as pd

from valens import database as db, web
from valens.models import (
    BodyFat,
    BodyWeight,
    Exercise,
    Period,
    Routine,
    RoutineExercise,
    User,
    Workout,
    WorkoutSet,
)

DATA_DIRECTORY = Path.home() / ".config/valens"

USERS_FILE = "users.feather"
ROUTINES_FILE = "routines.feather"
ROUTINE_SETS_FILE = "routine_sets.feather"
WORKOUTS_FILE = "workouts.feather"
SETS_FILE = "sets.feather"
BODYWEIGHT_FILE = "bodyweight.feather"
PERIOD_FILE = "period.feather"
BODYFAT_FILE = "bodyfat.feather"

USERS_COLS = ["user_id", "name", "sex"]
ROUTINES_COLS = ["user_id", "routine", "notes"]
ROUTINE_SETS_COLS = ["user_id", "routine", "exercise", "reps", "time", "weight", "rpe"]
WORKOUTS_COLS = ["user_id", "date", "notes"]
SETS_COLS = ["user_id", "date", "exercise", "reps", "time", "weight", "rpe"]
BODYWEIGHT_COLS = ["user_id", "date", "weight"]
PERIOD_COLS = ["user_id", "date", "intensity"]
BODYFAT_COLS = [
    "user_id",
    "date",
    "chest",
    "abdominal",
    "tigh",
    "tricep",
    "subscapular",
    "suprailiac",
    "midaxillary",
]


def read_users() -> pd.DataFrame:
    return pd.read_feather(DATA_DIRECTORY / USERS_FILE)


def read_routines(user_id: int) -> pd.DataFrame:
    return read_user_part(DATA_DIRECTORY / ROUTINES_FILE, ROUTINES_COLS, user_id)


def read_routine_sets(user_id: int) -> pd.DataFrame:
    return read_user_part(DATA_DIRECTORY / ROUTINE_SETS_FILE, ROUTINE_SETS_COLS, user_id)


def read_workouts(user_id: int) -> pd.DataFrame:
    return read_user_part(DATA_DIRECTORY / WORKOUTS_FILE, WORKOUTS_COLS, user_id)


def read_sets(user_id: int) -> pd.DataFrame:
    df = read_user_part(DATA_DIRECTORY / SETS_FILE, SETS_COLS, user_id)
    df["rir"] = 10 - df["rpe"]
    return df


def read_bodyweight(user_id: int) -> pd.DataFrame:
    return read_user_part(DATA_DIRECTORY / BODYWEIGHT_FILE, BODYWEIGHT_COLS, user_id)


def read_bodyfat(user_id: int) -> pd.DataFrame:
    return read_user_part(DATA_DIRECTORY / BODYFAT_FILE, BODYFAT_COLS, user_id)


def read_period(user_id: int) -> pd.DataFrame:
    return read_user_part(DATA_DIRECTORY / PERIOD_FILE, PERIOD_COLS, user_id)


def read_user_part(storage_file: Path, columns: Sequence[str], user_id: int) -> pd.DataFrame:
    return pd.read_feather(storage_file).loc[lambda x: x["user_id"] == user_id].loc[:, columns[1:]]


def convert() -> None:  # pylint: disable = too-many-locals, too-many-statements, too-many-branches
    with web.app.app_context():
        db.init_db()

        df_users = read_users()

        for _, user_id, user_name, sex in df_users.itertuples():
            user = User(name=user_name, sex=sex)
            df_bodyweight = read_bodyweight(user_id)

            for _, date, weight in df_bodyweight.itertuples():
                user.body_weight.append(BodyWeight(date=date, weight=weight))

            last = (None, None)
            df_bodyfat = read_bodyfat(user_id)

            for (
                _,
                date,
                chest,
                abdominal,
                tigh,
                tricep,
                subscapular,
                suprailiac,
                midaxillary,
            ) in df_bodyfat.itertuples():
                if last == (user_id, date):
                    print(
                        "SKIP DUPLICATE",
                        user_id,
                        date,
                        chest,
                        abdominal,
                        tigh,
                        tricep,
                        subscapular,
                        suprailiac,
                        midaxillary,
                    )
                    continue
                last = (user_id, date)
                user.body_fat.append(
                    BodyFat(
                        date=date,
                        chest=chest,
                        abdominal=abdominal,
                        tigh=tigh,
                        tricep=tricep,
                        subscapular=subscapular,
                        suprailiac=suprailiac,
                        midaxillary=midaxillary,
                    )
                )

            df_period = read_period(user_id)

            for _, date, intensity in df_period.itertuples():
                user.period.append(Period(date=date, intensity=intensity))

            workouts = {}
            df_workouts = read_workouts(user_id)

            for _, date, notes in df_workouts.itertuples():
                assert date not in workouts
                workouts[date] = Workout(date=date, notes=notes)
                user.workouts.append(workouts[date])

            exercises = {}
            position = 0
            last_date = None
            df_sets = read_sets(user_id)

            for _, date, exercise_name, reps, time, weight, rpe, _ in df_sets.itertuples():
                if date != last_date:
                    last_date = date
                    position = 1
                if date not in workouts:
                    workouts[date] = Workout(date=date)
                    user.workouts.append(workouts[date])
                if exercise_name not in exercises:
                    exercises[exercise_name] = Exercise(name=exercise_name)
                    user.exercises.append(exercises[exercise_name])
                workout_set = WorkoutSet(
                    position=position,
                    reps=reps,
                    time=time,
                    weight=weight,
                    rpe=rpe,
                )
                exercises[exercise_name].sets.append(workout_set)
                workouts[date].sets.append(workout_set)
                position += 1

            routines = {}
            df_routines = read_routines(user_id)

            for _, routine_name, notes in df_routines.itertuples():
                assert routine_name not in routines
                routines[routine_name] = Routine(name=routine_name, notes=notes)
                user.routines.append(routines[routine_name])

            position = 0
            last_routine = None
            df_routine_sets = read_routine_sets(user_id)

            for _, routine_name, exercise_name, sets in (
                df_routine_sets.groupby(["routine", "exercise"], sort=False)
                .size()
                .reset_index(name="sets")
                .itertuples()
            ):
                if routine_name != last_routine:
                    last_routine = routine_name
                    position = 1
                if routine_name not in routines:
                    routines[routine_name] = Routine(name=routine_name)
                    user.routines.append(routines[routine_name])
                routine_exercise = RoutineExercise(
                    position=position,
                    exercise=exercises[exercise_name],
                    sets=sets,
                )
                routines[routine_name].exercises.append(routine_exercise)
                position += 1

            db.session.add(user)

        db.session.commit()


convert()
