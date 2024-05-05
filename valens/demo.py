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
    RoutineActivity,
    RoutineSection,
    Sex,
    User,
    Workout,
    WorkoutRest,
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
        body_weight = _body_weight(user_id)
        result.append(
            User(
                id=user_id,
                name=name,
                sex=sex,
                body_weight=body_weight,
                body_fat=_body_fat(body_weight, user_id),
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
        weight += random.gauss(-0.1 if i % 2 == 0 else 0.2, 0.2)
        if random.randint(0, 2) == 0:
            continue
        values.append((datetime.date.today() - datetime.timedelta(days=i), weight))

    return [BodyWeight(user_id=user_id, date=d, weight=w) for d, w in values]


def _body_fat(body_weight: list[BodyWeight], user_id: int = 1) -> list[BodyFat]:
    initial_bw = body_weight[-1].weight
    initial_bf: tuple[int, ...] = (
        random.randint(5, 20),
        random.randint(10, 30),
        random.randint(10, 30),
        random.randint(10, 30),
        random.randint(5, 20),
        random.randint(5, 20),
        random.randint(5, 20),
    )
    bf = []
    i = 0

    while i < len(body_weight):
        date = body_weight[i].date
        factor = body_weight[i].weight / initial_bw
        bf.append((date, tuple(int(e * factor) + random.randint(0, 1) for e in initial_bf)))
        i += 7

    return [
        BodyFat(
            user_id=user_id,
            date=date,
            chest=che,
            abdominal=abd,
            thigh=thi,
            tricep=tri,
            subscapular=sub,
            suprailiac=sup,
            midaxillary=mid,
        )
        for date, (che, abd, thi, tri, sub, sup, mid) in bf
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
        "Barbell Back Squat": ExerciseType(reps=True, time=False, weight=True, rpe=True),
        "Barbell Bench Press": ExerciseType(reps=True, time=False, weight=True, rpe=True),
        "Barbell Deadlift": ExerciseType(reps=True, time=False, weight=True, rpe=True),
        "Barbell Lunge": ExerciseType(reps=True, time=False, weight=True, rpe=True),
        "Barbell Romanian Deadlift": ExerciseType(reps=True, time=False, weight=True, rpe=True),
        "Barbell Row": ExerciseType(reps=True, time=True, weight=True, rpe=True),
        "Dip": ExerciseType(reps=True, time=True, weight=False, rpe=True),
        "Dumbbell Curl": ExerciseType(reps=True, time=False, weight=True, rpe=True),
        "Dumbbell Press": ExerciseType(reps=True, time=False, weight=True, rpe=True),
        "Dumbbell Shoulder Press": ExerciseType(reps=True, time=False, weight=True, rpe=False),
        "Hip Thrust": ExerciseType(reps=True, time=False, weight=True, rpe=True),
        "Lat Pulldown": ExerciseType(reps=True, time=True, weight=True, rpe=True),
        "Plank": ExerciseType(reps=False, time=True, weight=False, rpe=False),
        "Pull Up": ExerciseType(reps=True, time=True, weight=False, rpe=True),
        "Push Up": ExerciseType(reps=True, time=True, weight=False, rpe=True),
        "Seated Leg Curl": ExerciseType(reps=True, time=False, weight=True, rpe=True),
        "Seated Leg Extension": ExerciseType(reps=True, time=False, weight=True, rpe=True),
        "Walking Lunge": ExerciseType(reps=True, time=False, weight=True, rpe=False),
    }
    exercises = [
        Exercise(user_id=user_id, name=name)
        for name in random.sample(sorted(exercise_names.keys()), k=len(exercise_names))
    ]

    routine_names = ["A", "B", "C", "D"]
    routines = [
        Routine(
            id=(user_id - 1) * len(routine_names) + i,
            user_id=user_id,
            name=f"Training {t}",
            sections=[
                RoutineSection(
                    position=p,
                    rounds=random.randint(2, 5),
                    parts=[
                        RoutineActivity(
                            position=1,
                            exercise=e,
                            reps=0,
                            time=0,
                            weight=0.0,
                            rpe=0.0,
                            automatic=False,
                        ),
                        RoutineActivity(
                            position=2,
                            reps=0,
                            time=60,
                            weight=0.0,
                            rpe=0.0,
                            automatic=True,
                        ),
                    ],
                )
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
            elements=[
                element
                for position, (exercise, reps, time, weight, rpe) in enumerate(
                    [
                        (routine_activity.exercise, reps, time, weight, rpe)
                        for routine_section in routines[quarter].sections
                        for routine_activity in routine_section.parts
                        if isinstance(routine_activity, RoutineActivity)
                        and routine_activity.exercise is not None
                        for reps, time, weight, rpe in (
                            [
                                (
                                    (
                                        5 + week + random.randint(0, 1)
                                        if exercise_names[routine_activity.exercise.name].reps
                                        else None
                                    ),
                                    (
                                        (
                                            random.randint(3, 4)
                                            if exercise_names[routine_activity.exercise.name].reps
                                            else 10 + 5 * week + 5 * random.randint(0, 2)
                                        )
                                        if exercise_names[routine_activity.exercise.name].time
                                        else None
                                    ),
                                    (
                                        5 + (week + random.randint(0, 1)) * 0.5
                                        if exercise_names[routine_activity.exercise.name].weight
                                        else None
                                    ),
                                    (
                                        min(7 + (week % 4) + (random.randint(0, 1) * 0.5), 10)
                                        if exercise_names[routine_activity.exercise.name].rpe
                                        else None
                                    ),
                                )
                            ]
                        )
                        for _ in range(routine_section.rounds)
                    ],
                    start=0,
                )
                for element in [
                    WorkoutSet(
                        position=2 * position + 1,
                        exercise=exercise,
                        reps=reps,
                        time=time,
                        weight=weight,
                        rpe=rpe,
                    ),
                    WorkoutRest(
                        position=2 * position + 2,
                        target_time=60,
                    ),
                ]
            ],
            routine=routines[quarter],
        )
        for quarter in range(len(routines))
        for week in range(13)
        for day in [0, 3]
    ]
    return (exercises, routines, workouts)
