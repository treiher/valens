import pandas as pd

from valens import database as db
from valens.models import BodyFat, BodyWeight, Exercise, Period, Routine, Workout, WorkoutSet

USERS_COLS = ["user_id", "name", "sex"]
ROUTINES_COLS = ["user_id", "routine", "notes"]
ROUTINE_SETS_COLS = ["user_id", "routine", "exercise", "reps", "time", "weight", "rpe"]
WORKOUTS_COLS = ["user_id", "date", "notes"]
SETS_COLS = [
    "user_id",
    "workout_id",
    "date",
    "routine",
    "exercise",
    "reps",
    "time",
    "weight",
    "rpe",
]
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


def read_sets(user_id: int) -> pd.DataFrame:
    df = (
        pd.read_sql(
            db.session.query(Workout, WorkoutSet, Exercise, Routine)
            .where(Workout.id == WorkoutSet.workout_id)
            .where(WorkoutSet.exercise_id == Exercise.id)
            .where(Workout.user_id == user_id)
            .join(Workout.routine, isouter=True)
            .statement,
            db.session.bind,
            columns=["Workout.user_id"],
        )
        .rename(columns={"name": "exercise"})
        .rename(columns={"name_1": "routine"})
        .loc[:, SETS_COLS[1:]]
    )
    df = df.astype({col: "float" for col in ["reps", "time", "weight", "rpe"]})
    df["rir"] = 10 - df["rpe"]
    return df


def read_bodyweight(user_id: int) -> pd.DataFrame:
    return pd.read_sql(
        db.session.query(BodyWeight).where(BodyWeight.user_id == user_id).statement,
        db.session.bind,
    ).loc[:, BODYWEIGHT_COLS[1:]]


def read_bodyfat(user_id: int) -> pd.DataFrame:
    return pd.read_sql(
        db.session.query(BodyFat).where(BodyFat.user_id == user_id).statement,
        db.session.bind,
    ).loc[:, BODYFAT_COLS[1:]]


def read_period(user_id: int) -> pd.DataFrame:
    return pd.read_sql(
        db.session.query(Period).where(Period.user_id == user_id).statement,
        db.session.bind,
    ).loc[:, PERIOD_COLS[1:]]
