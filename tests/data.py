from __future__ import annotations

import datetime

from valens.models import (
    BodyFat,
    BodyWeight,
    Exercise,
    ExerciseMuscle,
    Period,
    Routine,
    RoutineActivity,
    RoutineSection,
    Sex,
    User,
    Workout,
    WorkoutSet,
)


def users_only() -> list[User]:
    return [
        User(id=1, name="Alice", sex=Sex.FEMALE),
        User(id=2, name="Bob", sex=Sex.MALE),
    ]


def users(today: datetime.date = datetime.date(2002, 3, 12)) -> list[User]:

    def days_ago(days: int) -> datetime.date:
        return today - datetime.timedelta(days)

    exercise_1 = Exercise(
        id=1,
        user_id=1,
        name="Exercise 1",
        muscles=[ExerciseMuscle(user_id=1, muscle_id=11, stimulus=100)],
    )
    exercise_2 = Exercise(id=2, user_id=2, name="Exercise 2")
    exercise_3 = Exercise(id=3, user_id=1, name="Exercise 3")
    exercise_4 = Exercise(id=4, user_id=2, name="Exercise 4")
    exercise_5 = Exercise(id=5, user_id=1, name="Unused Exercise")

    return [
        User(
            id=1,
            name="Alice",
            sex=Sex.FEMALE,
            body_weight=[
                BodyWeight(user_id=1, date=days_ago(20), weight=67.5),
                BodyWeight(user_id=1, date=days_ago(19), weight=67.7),
                BodyWeight(user_id=1, date=days_ago(18), weight=67.3),
            ],
            body_fat=[
                BodyFat(
                    user_id=1,
                    date=days_ago(20),
                    chest=1,
                    abdominal=2,
                    thigh=3,
                    tricep=4,
                    subscapular=5,
                    suprailiac=6,
                    midaxillary=7,
                ),
                BodyFat(
                    user_id=1,
                    date=days_ago(19),
                    chest=None,
                    abdominal=None,
                    thigh=10,
                    tricep=11,
                    subscapular=None,
                    suprailiac=13,
                    midaxillary=None,
                ),
            ],
            period=[
                Period(date=days_ago(20), intensity=2),
                Period(date=days_ago(19), intensity=4),
                Period(date=days_ago(18), intensity=1),
            ],
            exercises=[
                exercise_1,
                exercise_3,
                exercise_5,
            ],
            routines=[
                Routine(
                    id=1,
                    user_id=1,
                    name="R1",
                    notes="First Routine",
                    sections=[
                        RoutineSection(
                            position=1,
                            rounds=1,
                            parts=[
                                RoutineActivity(
                                    position=1,
                                    exercise=exercise_3,
                                    reps=0,
                                    time=0,
                                    weight=0.0,
                                    rpe=0.0,
                                    automatic=False,
                                ),
                                RoutineActivity(
                                    position=2,
                                    reps=0,
                                    time=30,
                                    weight=0.0,
                                    rpe=0.0,
                                    automatic=False,
                                ),
                            ],
                        ),
                        RoutineSection(
                            position=2,
                            rounds=2,
                            parts=[
                                RoutineActivity(
                                    position=1,
                                    exercise=exercise_1,
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
                                    automatic=False,
                                ),
                                RoutineSection(
                                    position=3,
                                    rounds=2,
                                    parts=[
                                        RoutineActivity(
                                            position=1,
                                            exercise=exercise_1,
                                            reps=0,
                                            time=0,
                                            weight=0.0,
                                            rpe=0.0,
                                            automatic=False,
                                        ),
                                        RoutineActivity(
                                            position=2,
                                            reps=0,
                                            time=30,
                                            weight=0.0,
                                            rpe=0.0,
                                            automatic=False,
                                        ),
                                    ],
                                ),
                            ],
                        ),
                        RoutineSection(
                            position=3,
                            rounds=3,
                            parts=[
                                RoutineActivity(
                                    position=1,
                                    exercise=exercise_3,
                                    reps=0,
                                    time=20,
                                    weight=0.0,
                                    rpe=0.0,
                                    automatic=True,
                                ),
                                RoutineActivity(
                                    position=2,
                                    reps=0,
                                    time=10,
                                    weight=0.0,
                                    rpe=0.0,
                                    automatic=True,
                                ),
                            ],
                        ),
                    ],
                    archived=False,
                ),
                Routine(
                    id=3,
                    user_id=1,
                    name="R2",
                    notes=None,
                    sections=[
                        RoutineSection(
                            position=1,
                            rounds=5,
                            parts=[
                                RoutineActivity(
                                    position=1,
                                    exercise=exercise_3,
                                    reps=0,
                                    time=20,
                                    weight=0.0,
                                    rpe=0.0,
                                    automatic=True,
                                ),
                                RoutineActivity(
                                    position=2,
                                    reps=0,
                                    time=10,
                                    weight=0.0,
                                    rpe=0.0,
                                    automatic=True,
                                ),
                            ],
                        ),
                    ],
                    archived=False,
                ),
            ],
            workouts=[
                Workout(
                    id=1,
                    user_id=1,
                    routine_id=1,
                    date=days_ago(60),
                    notes="First Workout",
                    elements=[
                        WorkoutSet(position=1, exercise=exercise_3, reps=10, time=4, rpe=8.0),
                        WorkoutSet(position=2, exercise=exercise_1, reps=9, time=4, rpe=8.5),
                        WorkoutSet(position=3, exercise=exercise_1, time=60, rpe=9.0),
                    ],
                ),
                Workout(
                    id=3,
                    user_id=1,
                    date=days_ago(18),
                    notes=None,
                    elements=[
                        WorkoutSet(position=1, exercise=exercise_3, reps=9),
                        WorkoutSet(position=2, exercise=exercise_3, reps=8),
                        WorkoutSet(position=3, exercise=exercise_3, reps=7),
                        WorkoutSet(position=4, exercise=exercise_4, reps=6),
                        WorkoutSet(position=5, exercise=exercise_4, reps=5),
                    ],
                ),
                Workout(
                    id=4,
                    user_id=1,
                    routine_id=1,
                    date=days_ago(16),
                    notes=None,
                    elements=[
                        WorkoutSet(position=1, exercise=exercise_3, reps=11, time=4, rpe=8.5),
                    ],
                ),
            ],
        ),
        User(
            id=2,
            name="Bob",
            sex=Sex.MALE,
            body_weight=[
                BodyWeight(user_id=2, date=days_ago(20), weight=100),
                BodyWeight(user_id=2, date=days_ago(19), weight=101),
                BodyWeight(user_id=2, date=days_ago(18), weight=102),
                BodyWeight(user_id=2, date=days_ago(16), weight=104),
                BodyWeight(user_id=2, date=days_ago(15), weight=105),
                BodyWeight(user_id=2, date=days_ago(14), weight=106),
                BodyWeight(user_id=2, date=days_ago(12), weight=108),
                BodyWeight(user_id=2, date=days_ago(11), weight=109),
                BodyWeight(user_id=2, date=days_ago(10), weight=110),
                BodyWeight(user_id=2, date=days_ago(9), weight=111),
                BodyWeight(user_id=2, date=days_ago(7), weight=113),
                BodyWeight(user_id=2, date=days_ago(6), weight=114),
                BodyWeight(user_id=2, date=days_ago(5), weight=115),
                BodyWeight(user_id=2, date=days_ago(4), weight=116),
                BodyWeight(user_id=2, date=days_ago(2), weight=118),
                BodyWeight(user_id=2, date=days_ago(1), weight=119),
                BodyWeight(user_id=2, date=days_ago(0), weight=120),
            ],
            body_fat=[
                BodyFat(
                    user_id=2,
                    date=days_ago(20),
                    chest=15,
                    abdominal=16,
                    thigh=17,
                    tricep=18,
                    subscapular=19,
                    suprailiac=20,
                    midaxillary=21,
                ),
                BodyFat(
                    user_id=2,
                    date=days_ago(18),
                    chest=22,
                    abdominal=23,
                    thigh=24,
                    tricep=25,
                    subscapular=26,
                    suprailiac=27,
                    midaxillary=28,
                ),
            ],
            exercises=[
                exercise_2,
                exercise_4,
            ],
            routines=[
                Routine(
                    id=2,
                    user_id=2,
                    name="R1",
                    notes="",
                    sections=[
                        RoutineSection(
                            position=1,
                            rounds=3,
                            parts=[
                                RoutineActivity(
                                    position=1,
                                    exercise=exercise_2,
                                    reps=0,
                                    time=0,
                                    weight=0.0,
                                    rpe=0.0,
                                    automatic=False,
                                ),
                            ],
                        ),
                        RoutineSection(
                            position=2,
                            rounds=4,
                            parts=[
                                RoutineActivity(
                                    position=1,
                                    exercise=exercise_4,
                                    reps=0,
                                    time=0,
                                    weight=0.0,
                                    rpe=0.0,
                                    automatic=False,
                                ),
                            ],
                        ),
                    ],
                    archived=False,
                ),
                Routine(id=4, user_id=2, name="Empty", notes="TBD", archived=False),
            ],
            workouts=[
                Workout(
                    id=2,
                    user_id=2,
                    date=days_ago(20),
                    notes="",
                    elements=[
                        WorkoutSet(
                            position=1, exercise=exercise_2, reps=10, time=4, weight=10, rpe=8.5
                        ),
                        WorkoutSet(
                            position=2, exercise=exercise_2, reps=9, time=4, weight=8, rpe=9.0
                        ),
                        WorkoutSet(
                            position=3, exercise=exercise_2, reps=8, time=4, weight=6, rpe=9.5
                        ),
                        WorkoutSet(position=4, exercise=exercise_4, reps=7, weight=7.5),
                        WorkoutSet(position=5, exercise=exercise_4, reps=6, weight=7.5),
                        WorkoutSet(position=6, exercise=exercise_4, reps=5, weight=7.5),
                        WorkoutSet(position=7, exercise=exercise_4, reps=4, weight=7.5),
                    ],
                ),
            ],
        ),
    ]


def user(user_id: int = 1) -> User:
    return users()[user_id - 1]
