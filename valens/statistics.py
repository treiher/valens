import numpy as np
import pandas as pd

from valens import database as db
from valens.models import Exercise, Routine, Workout, WorkoutSet


def workouts(user_id: int) -> pd.DataFrame:
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
        .rename(columns={"name": "exercise", "name_1": "routine"})
        .loc[
            :,
            [
                "workout_id",
                "date",
                "routine_id",
                "routine",
                "exercise",
                "reps",
                "time",
                "weight",
                "rpe",
            ],
        ]
        .rename(columns={"workout_id": "id"})
        .astype(
            {
                "routine_id": "object",
                **{col: "Int64" for col in ["reps", "time"]},
                **{col: "Float64" for col in ["weight", "rpe"]},
            }
        )
    )

    df["date"] = df["date"].apply(lambda x: x.isoformat())
    df["tut"] = df["reps"].replace(np.nan, 1) * df["time"]

    df_grouped = df.groupby([df.id, df.date, df.routine_id, df.routine.fillna("")], dropna=False)
    df_sum = df_grouped.sum()
    df_mean = df_grouped.mean().rename(
        columns={
            "reps": "avg_reps",
            "time": "avg_time",
            "weight": "avg_weight",
            "rpe": "avg_rpe",
        }
    )

    df = df_mean
    df["tut"] = df_sum["tut"]
    df["volume"] = df_sum["reps"]
    df.reset_index(inplace=True)
    df = df.astype({"routine_id": "Int64"}, copy=False)

    return df
