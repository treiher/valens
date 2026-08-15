"""Hand-authored personas the example data is generated from."""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from valens.models import Role, Sex


@dataclass(frozen=True)
class ExerciseConfig:
    """
    Training parameters of one exercise of one persona.

    `rep_range` and `rep_step` are given in seconds for exercises that are tracked by time only.
    An `increment` of zero marks an exercise whose progression is limited to reps.
    """

    name: str
    start_load: float = 0.0
    increment: float = 0.0
    rep_range: tuple[int, int] = (8, 12)
    rep_step: int = 1
    target_rpe: float = 8.0
    alternative: str | None = None


@dataclass(frozen=True)
class Section:
    """
    A block of exercises, performed for `rounds` rounds.

    More than one exercise forms a superset.
    """

    exercises: tuple[str, ...]
    rounds: int


@dataclass(frozen=True)
class RoutinePlan:
    name: str
    sections: tuple[Section, ...]
    notes: str | None = None
    archived: bool = False


@dataclass(frozen=True)
class Schedule:
    """
    Weekly training slots.

    Without `rotation`, the routines are assigned to the weekdays one by one. With `rotation`, they
    are cycled through the slots, which desynchronizes them from the week whenever the number of
    slots is not a multiple of the number of routines.
    """

    weekdays: tuple[int, ...]
    routines: tuple[str, ...]
    rotation: str | None = None


@dataclass(frozen=True)
class Profile:
    """
    One example user, and the shape of the training log and body metrics generated for it.

    The event counts place rare occurrences deterministically, so that the generated data covers
    every kind of entry regardless of the seed.
    """

    id: int
    name: str
    sex: Sex
    height: int
    role: Role
    start_weight: float
    # Length in weeks, change of the body weight and change of the fat mass in kg per week of each
    # phase, covering the whole history. The difference of the two changes is the change of the
    # lean mass.
    weight_phases: tuple[tuple[int, float, float], ...]
    skinfolds: Mapping[str, int]
    skinfold_sites: tuple[str, ...]
    exercises: Sequence[ExerciseConfig]
    routines: Sequence[RoutinePlan]
    schedule: Schedule
    deload_interval: int
    # Number of times an exercise is performed before its reps advance
    progression_interval: int
    # Number of trailing weeks without progression
    plateau_weeks: int = 0
    weight_gaps: int = 2
    shifted_sessions: int = 8
    dropped_sections: int = 4
    dropped_rounds: int = 6
    substitutions: int = 3
    stalls: int = 5
    sessions_without_rests: int = 0
    session_notes: int = 6
    exercise_notes: int = 4


ALICE = Profile(
    id=1,
    name="Alice",
    sex=Sex.FEMALE,
    height=168,
    role=Role.ADMIN,
    start_weight=62.0,
    weight_phases=((10, 0.03, -0.04), (12, -0.4, -0.36), (30, 0.03, -0.03)),
    skinfolds={
        "chest": 12,
        "abdominal": 18,
        "thigh": 26,
        "tricep": 20,
        "subscapular": 14,
        "suprailiac": 16,
        "midaxillary": 12,
    },
    skinfold_sites=("tricep", "suprailiac", "thigh"),
    exercises=[
        ExerciseConfig(
            "Barbell Squat",
            start_load=60.0,
            increment=2.5,
            rep_range=(5, 8),
            alternative="Goblet Squat",
        ),
        ExerciseConfig(
            "Barbell Bench Press",
            start_load=40.0,
            increment=2.5,
            rep_range=(5, 8),
            alternative="Machine Chest Press",
        ),
        ExerciseConfig(
            "Barbell Deadlift",
            start_load=75.0,
            increment=5.0,
            rep_range=(3, 6),
            target_rpe=8.5,
            alternative="Leg Press",
        ),
        ExerciseConfig(
            "Barbell Shoulder Press",
            start_load=27.5,
            increment=2.5,
            rep_range=(5, 8),
            alternative="Dumbbell Shoulder Press",
        ),
        ExerciseConfig("Barbell Romanian Deadlift", start_load=50.0, increment=2.5),
        ExerciseConfig("Barbell Hip Thrust", start_load=60.0, increment=5.0),
        ExerciseConfig("Cable Row", start_load=40.0, increment=2.5),
        ExerciseConfig("Lat Pulldown", start_load=35.0, increment=2.5),
        ExerciseConfig("Seated Leg Curl", start_load=30.0, increment=2.0),
        ExerciseConfig("Leg Extension", start_load=35.0, increment=2.0, rep_range=(10, 15)),
        ExerciseConfig(
            "Machine Standing Calf Raise", start_load=50.0, increment=2.0, rep_range=(10, 15)
        ),
        ExerciseConfig("Dumbbell Curl", start_load=10.0, increment=1.0),
        ExerciseConfig("Barbell Skull Crusher", start_load=20.0, increment=1.25),
        ExerciseConfig("Dumbbell Lateral Raise", start_load=6.0, increment=1.0, rep_range=(10, 15)),
        ExerciseConfig("Cable Rope Face Pull", start_load=20.0, increment=2.5, rep_range=(12, 15)),
        ExerciseConfig("Dip", rep_range=(5, 12), target_rpe=9.0),
        ExerciseConfig("Plank", rep_range=(30, 90), rep_step=5),
        ExerciseConfig("Goblet Squat", start_load=20.0, increment=2.0),
        ExerciseConfig("Machine Chest Press", start_load=30.0, increment=2.0),
        ExerciseConfig("Dumbbell Shoulder Press", start_load=12.0, increment=1.0),
        ExerciseConfig("Leg Press", start_load=100.0, increment=5.0),
        ExerciseConfig("Machine Hip Abduction", start_load=30.0, increment=2.0, rep_range=(12, 15)),
        ExerciseConfig("Push Up", rep_range=(8, 20)),
    ],
    routines=[
        RoutinePlan(
            "Upper A",
            (
                Section(("Barbell Bench Press",), 4),
                Section(("Cable Row",), 3),
                Section(("Dumbbell Lateral Raise", "Cable Rope Face Pull"), 3),
                Section(("Barbell Skull Crusher",), 3),
            ),
            notes="Press day. Keep the last set two reps shy of failure.",
        ),
        RoutinePlan(
            "Lower A",
            (
                Section(("Barbell Squat",), 4),
                Section(("Barbell Romanian Deadlift",), 3),
                Section(("Leg Extension",), 3),
                Section(("Machine Standing Calf Raise",), 3),
            ),
        ),
        RoutinePlan(
            "Upper B",
            (
                Section(("Barbell Shoulder Press",), 4),
                Section(("Lat Pulldown",), 3),
                Section(("Dip",), 3),
                Section(("Dumbbell Curl",), 3),
            ),
        ),
        RoutinePlan(
            "Lower B",
            (
                Section(("Barbell Deadlift",), 3),
                Section(("Barbell Hip Thrust",), 3),
                Section(("Seated Leg Curl",), 3),
                Section(("Plank",), 3),
            ),
            notes="Pull day. Reset the setup between deadlift reps.",
        ),
        RoutinePlan(
            "Full Body",
            (
                Section(("Barbell Squat",), 3),
                Section(("Barbell Bench Press",), 3),
                Section(("Cable Row",), 3),
            ),
            archived=True,
        ),
    ],
    schedule=Schedule(weekdays=(1, 2, 4, 5), routines=("Upper A", "Lower A", "Upper B", "Lower B")),
    deload_interval=5,
    progression_interval=4,
)

BOB = Profile(
    id=2,
    name="Bob",
    sex=Sex.MALE,
    height=182,
    role=Role.USER,
    start_weight=78.0,
    weight_phases=((20, 0.15, 0.05), (12, 0.0, -0.04), (20, 0.15, 0.06)),
    skinfolds={
        "chest": 14,
        "abdominal": 24,
        "thigh": 18,
        "tricep": 12,
        "subscapular": 18,
        "suprailiac": 20,
        "midaxillary": 14,
    },
    skinfold_sites=("chest", "abdominal", "thigh"),
    exercises=[
        ExerciseConfig(
            "Barbell Squat",
            start_load=20.0,
            increment=2.5,
            rep_range=(5, 8),
            alternative="Goblet Squat",
        ),
        ExerciseConfig(
            "Barbell Bench Press",
            start_load=20.0,
            increment=2.5,
            rep_range=(5, 8),
            alternative="Machine Chest Press",
        ),
        ExerciseConfig("Barbell Deadlift", start_load=40.0, increment=5.0, rep_range=(5, 8)),
        ExerciseConfig("Dumbbell Shoulder Press", start_load=10.0, increment=1.0),
        ExerciseConfig("Cable Row", start_load=25.0, increment=2.5),
        ExerciseConfig("Goblet Squat", start_load=12.0, increment=2.0),
        ExerciseConfig("Machine Chest Press", start_load=25.0, increment=2.0),
        ExerciseConfig("Lat Pulldown", start_load=30.0, increment=2.5),
        ExerciseConfig("Leg Press", start_load=80.0, increment=5.0),
        ExerciseConfig("Crunch", rep_range=(10, 20)),
    ],
    routines=[
        RoutinePlan(
            "Full Body A",
            (
                Section(("Barbell Squat",), 3),
                Section(("Barbell Bench Press",), 3),
                Section(("Cable Row",), 3),
            ),
            notes="Add 2.5 kg to the squat whenever all sets hit the top of the rep range.",
        ),
        RoutinePlan(
            "Full Body B",
            (
                Section(("Barbell Squat",), 3),
                Section(("Dumbbell Shoulder Press",), 3),
                Section(("Barbell Deadlift",), 2),
            ),
        ),
        RoutinePlan(
            "Machine Circuit",
            (
                Section(("Leg Press",), 3),
                Section(("Lat Pulldown",), 3),
                Section(("Crunch",), 3),
            ),
            archived=True,
        ),
    ],
    schedule=Schedule(weekdays=(1, 3, 5), routines=("Full Body A", "Full Body B"), rotation="A/B"),
    deload_interval=6,
    progression_interval=4,
    plateau_weeks=8,
    shifted_sessions=5,
    dropped_sections=3,
    dropped_rounds=4,
    substitutions=2,
    stalls=3,
    sessions_without_rests=6,
    session_notes=4,
    exercise_notes=3,
)

PROFILES = (ALICE, BOB)
