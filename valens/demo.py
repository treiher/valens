from __future__ import annotations

import datetime
import random
from dataclasses import dataclass

from valens import app, database as db
from valens.models import (
    BodyFat,
    BodyWeight,
    Exercise,
    Period,
    Routine,
    RoutineExercise,
    Sex,
    User,
    Workout,
    WorkoutSet,
)


def run(database: str, host: str = "127.0.0.1", port: int = 5000) -> None:
    app.config["DATABASE"] = database
    app.config["SECRET_KEY"] = b"TEST_KEY"
    with app.app_context():
        for user in users():
            db.session.add(user)
        db.session.commit()
        app.run(host, port)


def users() -> list[User]:
    result = []
    for user_id, name, sex in [(1, "Alice", Sex.FEMALE), (2, "Bob", Sex.MALE)]:
        exercises, routines, workouts = _workouts(user_id)
        result.append(
            User(
                id=user_id,
                name=name,
                sex=sex,
                body_weight=_body_weight(user_id),
                body_fat=_body_fat(user_id),
                period=_period(user_id),
                exercises=exercises,
                routines=routines,
                workouts=workouts,
            )
        )
    return result


def _body_weight(user_id: int = 1) -> list[BodyWeight]:
    day = datetime.date.today()
    weight = random.uniform(50, 100)
    values = [(day, weight)]

    for i in range(1, 365):
        weight += random.gauss(-0.2 if i % 2 == 0 else 0.2, 0.2)
        if random.randint(0, 2) == 0:
            continue
        values.append((datetime.date.today() - datetime.timedelta(days=i), weight))

    return [BodyWeight(user_id=user_id, date=d, weight=w) for d, w in values]


def _body_fat(user_id: int = 1) -> list[BodyFat]:
    day = datetime.date.today() - datetime.timedelta(days=random.randint(0, 7))
    values = []

    previous: tuple[int, ...] = (
        random.randint(5, 20),
        random.randint(10, 30),
        random.randint(10, 30),
        random.randint(10, 30),
        random.randint(5, 20),
        random.randint(5, 20),
        random.randint(5, 20),
    )
    for _ in range(52):
        value = tuple(max(1, abs(e + int(random.gauss(0, 0.8)))) for e in previous)
        previous = value
        values.append((day, value))
        day -= datetime.timedelta(days=7)

    return [
        BodyFat(
            user_id=user_id,
            date=date,
            chest=che,
            abdominal=abd,
            tigh=tig,
            tricep=tri,
            subscapular=sub,
            suprailiac=sup,
            midaxillary=mid,
        )
        for date, (che, abd, tig, tri, sub, sup, mid) in values
    ]


def _period(user_id: int = 1) -> list[Period]:
    day = datetime.date.today() - datetime.timedelta(days=random.randint(7, 33))
    values = []

    for _ in range(13):
        previous = 4
        for d in range(7):
            intensity = random.randint(max(0, 3 - d), min(4, previous))
            previous = intensity

            if intensity == 0:
                continue

            values.append((day + datetime.timedelta(days=d), intensity))

        day -= datetime.timedelta(days=28 + int(random.gauss(0, 3)))

    return [Period(user_id=user_id, date=d, intensity=i) for d, i in values]


@dataclass
class ExerciseType:
    reps: bool
    time: bool
    weight: bool
    rpe: bool


def _workouts(user_id: int = 1) -> tuple[list[Exercise], list[Routine], list[Workout]]:
    exercise_names = {
        "Bench Press": ExerciseType(True, False, True, True),
        "Bodyrow": ExerciseType(True, True, False, True),
        "Burpee": ExerciseType(True, False, False, False),
        "Chin Up": ExerciseType(True, True, True, True),
        "Deadlift": ExerciseType(True, False, True, True),
        "Dip": ExerciseType(True, True, True, True),
        "Glute Bridge": ExerciseType(True, False, False, True),
        "Handstand": ExerciseType(False, True, False, False),
        "Hip Thrust": ExerciseType(True, False, False, True),
        "Lunge": ExerciseType(True, False, True, True),
        "Mountain Climber": ExerciseType(False, True, False, False),
        "Overhead Press": ExerciseType(True, False, True, True),
        "Plank": ExerciseType(False, True, False, False),
        "Pull Up": ExerciseType(True, True, True, True),
        "Push Up": ExerciseType(True, True, False, True),
        "Squat": ExerciseType(True, False, True, True),
        "Step Up": ExerciseType(True, False, True, True),
    }
    exercises = [
        Exercise(user_id=user_id, name=name)
        for name in random.sample(exercise_names.keys(), k=len(exercise_names))
    ]

    routine_names = ["A", "B", "C", "D"]
    routines = [
        Routine(
            id=(user_id - 1) * len(routine_names) + i,
            user_id=user_id,
            name=f"Training {t}",
            exercises=[
                RoutineExercise(position=p, exercise=e, sets=random.randint(1, 5))
                for p, e in enumerate(random.sample(exercises, random.randint(5, 8)), start=1)
            ],
        )
        for i, t in enumerate(routine_names, start=1)
    ]

    workouts = [
        Workout(
            user_id=user_id,
            date=datetime.date.today()
            - datetime.timedelta(days=len(routines) * 13 * 7)
            + datetime.timedelta(days=(quarter * 13 * 7) + (week * 7) + day),
            sets=[
                WorkoutSet(
                    position=p,
                    exercise=e,
                    reps=5 + week + random.randint(0, 2) if exercise_names[e.name].reps else None,
                    time=t,
                    weight=w,
                    rpe=random.randint(10, 20) * 0.5 if exercise_names[e.name].rpe else None,
                )
                for p, (e, t, w) in enumerate(
                    [
                        (routine_exercise.exercise, t, w)
                        for routine_exercise in routines[quarter].exercises
                        for t, w in [
                            (
                                (
                                    random.randint(3, 4)
                                    if exercise_names[routine_exercise.exercise.name].reps
                                    else 10 + 4 * week + 5 * random.randint(0, 2)
                                )
                                if exercise_names[routine_exercise.exercise.name].time
                                else None,
                                5 + week + random.randint(0, 2)
                                if exercise_names[routine_exercise.exercise.name].weight
                                else None,
                            )
                        ]
                        for _ in range(routine_exercise.sets)
                    ],
                    start=1,
                )
            ],
            routine=routines[quarter],
        )
        for quarter in range(len(routines))
        for week in range(13)
        for day in [0, 3]
    ]
    return (exercises, routines, workouts)
