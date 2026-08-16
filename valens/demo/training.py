"""Generation of exercises, routines, schedules and training history."""

from __future__ import annotations

import datetime
import random
from dataclasses import dataclass, field

from valens.demo.events import event_indices
from valens.demo.profiles import ExerciseConfig, Profile, RoutinePlan
from valens.models import (
    Exercise,
    ExerciseMuscle,
    Routine,
    RoutineActivity,
    RoutineSection,
    ScheduleRotation,
    ScheduleRotationRoutine,
    ScheduleSlot,
    Workout,
    WorkoutElement,
    WorkoutExerciseNote,
    WorkoutRest,
    WorkoutSet,
)

# Muscle IDs mirror `MuscleID` in `crates/domain/src/exercise.rs`
PECS = 11
TRAPS = 21
LATS = 22
FRONT_DELTS = 31
SIDE_DELTS = 32
REAR_DELTS = 33
BICEPS = 41
TRICEPS = 42
FOREARMS = 51
ABS = 61
ERECTOR_SPINAE = 62
GLUTES = 71
ABDUCTORS = 72
QUADS = 81
HAMSTRINGS = 82
ADDUCTORS = 83
CALVES = 91

# Stimulus values mirror `Stimulus` in `crates/domain/src/exercise.rs`
PRIMARY = 100
SECONDARY = 50

SECONDS_PER_REP = 3

# Rest between rounds in seconds, by exercise class
LONG_REST = 180
REST = 150
SHORT_REST = 90

# Rests up to this duration are counted down automatically
AUTOMATIC_REST = SHORT_REST

RPE_STEP = 0.5

# Shift of the ratings of a session, drawn once per session
RATING_OFFSETS = (-0.5, 0.0, 0.0, 0.5)

DELOAD_FACTOR = 0.85

SESSION_NOTES = (
    "Everything moved well today.",
    "Rushed, cut the accessories short.",
    "Slept badly, kept the loads conservative.",
    "Gym was crowded, longer rests than planned.",
)

EXERCISE_NOTES = (
    "Left knee felt off, stayed with the same load.",
    "Grip gave out before the last reps.",
    "Bar path drifted forward on the last round.",
)


@dataclass(frozen=True)
class ExerciseType:
    reps: bool
    time: bool
    weight: bool
    rpe: bool


@dataclass(frozen=True)
class ExerciseDefinition:
    type: ExerciseType
    muscles: tuple[tuple[int, int], ...]
    rest: int


@dataclass
class Training:
    exercises: list[Exercise]
    routines: list[Routine]
    workouts: list[Workout]
    schedule_rotations: list[ScheduleRotation]
    schedule_slots: list[ScheduleSlot]


# Names and muscles are taken from `crates/domain/src/catalog.rs` and kept in sync by hand
EXERCISES = {
    "Barbell Squat": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        (
            (QUADS, PRIMARY),
            (GLUTES, PRIMARY),
            (ADDUCTORS, PRIMARY),
            (ERECTOR_SPINAE, PRIMARY),
            (CALVES, SECONDARY),
        ),
        LONG_REST,
    ),
    "Goblet Squat": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        (
            (QUADS, PRIMARY),
            (GLUTES, PRIMARY),
            (ADDUCTORS, PRIMARY),
            (ERECTOR_SPINAE, PRIMARY),
            (CALVES, SECONDARY),
        ),
        REST,
    ),
    "Leg Press": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        ((QUADS, PRIMARY), (GLUTES, PRIMARY), (ADDUCTORS, PRIMARY), (HAMSTRINGS, SECONDARY)),
        REST,
    ),
    "Barbell Deadlift": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        (
            (GLUTES, PRIMARY),
            (ERECTOR_SPINAE, PRIMARY),
            (QUADS, SECONDARY),
            (HAMSTRINGS, SECONDARY),
            (ADDUCTORS, SECONDARY),
            (TRAPS, SECONDARY),
            (FOREARMS, SECONDARY),
        ),
        LONG_REST,
    ),
    "Barbell Romanian Deadlift": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        (
            (GLUTES, PRIMARY),
            (ERECTOR_SPINAE, PRIMARY),
            (QUADS, SECONDARY),
            (HAMSTRINGS, SECONDARY),
            (ADDUCTORS, SECONDARY),
            (TRAPS, SECONDARY),
            (FOREARMS, SECONDARY),
        ),
        REST,
    ),
    "Barbell Hip Thrust": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        ((GLUTES, PRIMARY), (QUADS, SECONDARY), (ADDUCTORS, SECONDARY)),
        REST,
    ),
    "Seated Leg Curl": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        ((HAMSTRINGS, PRIMARY),),
        SHORT_REST,
    ),
    "Leg Extension": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        ((QUADS, PRIMARY),),
        SHORT_REST,
    ),
    "Machine Hip Abduction": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        ((ABDUCTORS, PRIMARY), (GLUTES, SECONDARY)),
        SHORT_REST,
    ),
    "Machine Standing Calf Raise": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        ((CALVES, PRIMARY),),
        SHORT_REST,
    ),
    "Barbell Bench Press": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        ((PECS, PRIMARY), (FRONT_DELTS, PRIMARY), (TRICEPS, SECONDARY)),
        LONG_REST,
    ),
    "Machine Chest Press": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        ((PECS, PRIMARY), (FRONT_DELTS, PRIMARY), (TRICEPS, SECONDARY)),
        REST,
    ),
    "Push Up": ExerciseDefinition(
        ExerciseType(reps=True, time=True, weight=False, rpe=True),
        ((PECS, PRIMARY), (FRONT_DELTS, PRIMARY), (TRICEPS, SECONDARY), (ABS, SECONDARY)),
        SHORT_REST,
    ),
    "Dip": ExerciseDefinition(
        ExerciseType(reps=True, time=True, weight=False, rpe=True),
        ((PECS, PRIMARY), (FRONT_DELTS, PRIMARY), (TRICEPS, PRIMARY)),
        REST,
    ),
    "Barbell Shoulder Press": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        ((FRONT_DELTS, PRIMARY), (SIDE_DELTS, SECONDARY), (TRICEPS, SECONDARY)),
        LONG_REST,
    ),
    "Dumbbell Shoulder Press": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=False),
        ((FRONT_DELTS, PRIMARY), (SIDE_DELTS, SECONDARY), (TRICEPS, SECONDARY)),
        REST,
    ),
    "Dumbbell Lateral Raise": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        ((SIDE_DELTS, PRIMARY), (FRONT_DELTS, SECONDARY)),
        SHORT_REST,
    ),
    "Cable Rope Face Pull": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        ((REAR_DELTS, PRIMARY), (SIDE_DELTS, SECONDARY), (TRAPS, SECONDARY)),
        SHORT_REST,
    ),
    "Cable Row": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        (
            (LATS, PRIMARY),
            (TRAPS, PRIMARY),
            (REAR_DELTS, PRIMARY),
            (BICEPS, SECONDARY),
            (FOREARMS, SECONDARY),
        ),
        REST,
    ),
    "Lat Pulldown": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        (
            (LATS, PRIMARY),
            (BICEPS, SECONDARY),
            (REAR_DELTS, SECONDARY),
            (FOREARMS, SECONDARY),
        ),
        REST,
    ),
    "Dumbbell Curl": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        ((BICEPS, PRIMARY),),
        SHORT_REST,
    ),
    "Barbell Skull Crusher": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=True, rpe=True),
        ((TRICEPS, PRIMARY),),
        SHORT_REST,
    ),
    "Crunch": ExerciseDefinition(
        ExerciseType(reps=True, time=False, weight=False, rpe=False),
        ((ABS, PRIMARY),),
        SHORT_REST,
    ),
    "Plank": ExerciseDefinition(
        ExerciseType(reps=False, time=True, weight=False, rpe=False),
        ((ABS, PRIMARY),),
        SHORT_REST,
    ),
}


@dataclass
class _Session:
    """A scheduled training session, before its sets are generated."""

    date: datetime.date
    week: int
    routine: str


@dataclass
class _Set:
    exercise: str
    reps: int | None
    time: int | None
    weight: float | None
    rpe: float | None
    target_reps: int | None
    target_time: int | None
    target_weight: float | None
    target_rpe: float | None
    rest: int


@dataclass
class _Record:
    """A performed training session."""

    date: datetime.date
    routine: str
    sets: list[_Set]
    rests: bool
    notes: str | None = None
    exercise_notes: list[tuple[str, str]] = field(default_factory=list)


@dataclass
class _Progress:
    """The working load of an exercise and the reps or seconds targeted with it."""

    load: float
    counter: int
    performed: int = 0


def training(profile: Profile, today: datetime.date, rng: random.Random) -> Training:
    config = {c.name: c for c in profile.exercises}
    exercises = {
        name: Exercise(
            user_id=profile.id,
            name=name,
            muscles=[
                ExerciseMuscle(user_id=profile.id, muscle_id=muscle, stimulus=stimulus)
                for muscle, stimulus in EXERCISES[name].muscles
            ],
        )
        for name in config
    }
    records, progress = _history(profile, config, _sessions(profile, today, rng), rng)
    routines = _routines(profile, config, exercises, progress)
    rotations, slots = _schedule(profile, routines)
    return Training(
        exercises=list(exercises.values()),
        routines=list(routines.values()),
        workouts=_workouts(profile, records, routines, exercises),
        schedule_rotations=rotations,
        schedule_slots=slots,
    )


def _sessions(profile: Profile, today: datetime.date, rng: random.Random) -> list[_Session]:
    """Return the sessions of the last year, as the schedule places them and attendance allows."""
    schedule = profile.schedule
    start = today - datetime.timedelta(days=364)
    start += datetime.timedelta(days=-start.weekday() % 7)
    sessions: list[_Session] = []

    for week in range((today - start).days // 7 + 1):
        for slot, weekday in enumerate(schedule.weekdays):
            date = start + datetime.timedelta(days=week * 7 + weekday - 1)
            if date > today:
                continue
            if schedule.rotation is None:
                routine = schedule.routines[slot]
            else:
                routine = schedule.routines[len(sessions) % len(schedule.routines)]
            sessions.append(_Session(date=date, week=week, routine=routine))

    sessions = _attended(sessions, rng)

    for index in event_indices(rng, profile.shifted_sessions, len(sessions)):
        session = sessions[index]
        shift = -1 if session.date.isoweekday() + 1 in schedule.weekdays else 1
        session.date += datetime.timedelta(days=shift)

    return sessions


def _attended(sessions: list[_Session], rng: random.Random) -> list[_Session]:
    """Remove the sessions that fall into a holiday or an illness."""
    gaps = [
        (int(len(sessions) * fraction) + rng.randint(-5, 5), rng.randint(*duration))
        for fraction, duration in ((0.25, (3, 5)), (0.6, (7, 10)))
    ]
    absent = {
        sessions[index].date + datetime.timedelta(days=day)
        for index, duration in gaps
        for day in range(duration)
    }
    return [session for session in sessions if session.date not in absent]


def _history(
    profile: Profile,
    config: dict[str, ExerciseConfig],
    sessions: list[_Session],
    rng: random.Random,
) -> tuple[list[_Record], dict[str, _Progress]]:
    plans = {plan.name: plan for plan in profile.routines}
    progress = {
        c.name: _Progress(load=c.start_load, counter=c.rep_range[0]) for c in config.values()
    }
    total = len(sessions)
    dropped_sections = event_indices(rng, profile.dropped_sections, total)
    dropped_rounds = event_indices(rng, profile.dropped_rounds, total)
    substitutions = event_indices(rng, profile.substitutions, total)
    stalls = event_indices(rng, profile.stalls, total)
    without_rests = event_indices(rng, profile.sessions_without_rests, total)
    session_notes = sorted(event_indices(rng, profile.session_notes, total))
    exercise_notes = sorted(event_indices(rng, profile.exercise_notes, total))
    weeks = sessions[-1].week + 1
    records = []

    for index, session in enumerate(sessions):
        plan = plans[session.routine]
        sections = plan.sections[:-1] if index in dropped_sections else plan.sections
        substitution = _substitution(plan, config, substitute=index in substitutions)
        deload = (session.week + 1) % profile.deload_interval == 0
        stalled = index in stalls or session.week >= weeks - profile.plateau_weeks
        rating = rng.choice(RATING_OFFSETS)
        sets = []

        for section in sections:
            rounds = section.rounds
            if deload or index in dropped_rounds:
                rounds = max(1, rounds - 1)
            for round_index in range(rounds):
                for exercise in section.exercises:
                    name = substitution.get(exercise, exercise)
                    performed = _set(
                        config[name],
                        progress[name],
                        rounds=rounds,
                        round_index=round_index,
                        deload=deload,
                        backoff=index in stalls and round_index == rounds - 1,
                    )
                    sets.append(_rated(performed, rating))

        for name in dict.fromkeys(s.exercise for s in sets):
            _advance(config[name], progress[name], profile.progression_interval, stalled=stalled)

        record = _Record(
            date=session.date,
            routine=session.routine,
            sets=sets,
            rests=index not in without_rests,
        )
        if index in session_notes:
            record.notes = SESSION_NOTES[session_notes.index(index) % len(SESSION_NOTES)]
        if index in exercise_notes:
            note = EXERCISE_NOTES[exercise_notes.index(index) % len(EXERCISE_NOTES)]
            record.exercise_notes = [(sets[0].exercise, note)]
        records.append(record)

    return records, progress


def _substitution(
    plan: RoutinePlan, config: dict[str, ExerciseConfig], *, substitute: bool
) -> dict[str, str]:
    """Map the leading exercise of `plan` to the alternative it is replaced by."""
    name = plan.sections[0].exercises[0]
    alternative = config[name].alternative
    if not substitute or alternative is None:
        return {}
    return {name: alternative}


def _set(
    config: ExerciseConfig,
    progress: _Progress,
    *,
    rounds: int,
    round_index: int,
    deload: bool,
    backoff: bool,
) -> _Set:
    definition = EXERCISES[config.name]
    top = config.rep_range[1]
    target = progress.counter
    achieved = target

    # Reps decay across the rounds once the top of the range is targeted
    if round_index > 0 and target == top:
        achieved -= 1
    if backoff:
        achieved -= 1
    achieved = max(1, achieved)

    target_weight = progress.load
    if deload:
        target_weight = _snap(progress.load * DELOAD_FACTOR, config.increment)
    weight = max(config.increment, target_weight - config.increment) if backoff else target_weight

    # The rating rises across the rounds and with the proximity to the top of the rep range
    rpe = config.target_rpe - RPE_STEP * (rounds - 1 - round_index)
    rpe = min(rpe + (RPE_STEP if target == top else 0.0), 10.0)
    if deload:
        rpe -= 1.0
    rpe = max(rpe, RPE_STEP)

    return _Set(
        exercise=config.name,
        reps=achieved if definition.type.reps else None,
        time=_duration(definition, achieved),
        weight=weight if definition.type.weight else None,
        rpe=rpe if definition.type.rpe else None,
        target_reps=target if definition.type.reps else None,
        target_time=_duration(definition, target),
        target_weight=target_weight if definition.type.weight else None,
        target_rpe=config.target_rpe if definition.type.rpe else None,
        rest=definition.rest,
    )


def _rated(performed: _Set, offset: float) -> _Set:
    """Shift the rating of a set by how the day went."""
    if performed.rpe is not None:
        performed.rpe = min(max(performed.rpe + offset, RPE_STEP), 10.0)
    return performed


def _duration(definition: ExerciseDefinition, count: int) -> int | None:
    if not definition.type.time:
        return None
    if definition.type.reps:
        return count * SECONDS_PER_REP
    return count


def _snap(weight: float, increment: float) -> float:
    return round(weight / increment) * increment if increment else 0.0


def _advance(config: ExerciseConfig, progress: _Progress, interval: int, *, stalled: bool) -> None:
    """Apply double progression: raise the reps first, the load once the rep range is exhausted."""
    progress.performed += 1
    if stalled or progress.performed % interval != 0:
        return

    progress.counter += config.rep_step
    if progress.counter <= config.rep_range[1]:
        return

    if config.increment > 0:
        progress.counter = config.rep_range[0]
        progress.load += config.increment
    else:
        progress.counter = config.rep_range[1]


def _routines(
    profile: Profile,
    config: dict[str, ExerciseConfig],
    exercises: dict[str, Exercise],
    progress: dict[str, _Progress],
) -> dict[str, Routine]:
    """Create the routines, targeting the working loads the history ends with."""
    return {
        plan.name: Routine(
            id=profile.id * 100 + index,
            user_id=profile.id,
            name=plan.name,
            notes=plan.notes,
            archived=plan.archived,
            sections=[
                RoutineSection(
                    position=position,
                    rounds=section.rounds,
                    parts=[
                        part
                        for offset, name in enumerate(section.exercises)
                        for part in [
                            _activity(
                                2 * offset + 1, config[name], progress[name], exercises[name]
                            ),
                            _rest(2 * offset + 2, EXERCISES[name].rest),
                        ]
                    ],
                )
                for position, section in enumerate(plan.sections, start=1)
            ],
        )
        for index, plan in enumerate(profile.routines, start=1)
    }


def _activity(
    position: int, config: ExerciseConfig, progress: _Progress, exercise: Exercise
) -> RoutineActivity:
    definition = EXERCISES[config.name]
    return RoutineActivity(
        position=position,
        exercise=exercise,
        reps=progress.counter if definition.type.reps else 0,
        time=_duration(definition, progress.counter) or 0,
        weight=progress.load if definition.type.weight else 0.0,
        rpe=config.target_rpe if definition.type.rpe else 0.0,
        automatic=False,
    )


def _rest(position: int, rest: int) -> RoutineActivity:
    return RoutineActivity(
        position=position,
        reps=0,
        time=rest,
        weight=0.0,
        rpe=0.0,
        automatic=rest <= AUTOMATIC_REST,
    )


def _workouts(
    profile: Profile,
    records: list[_Record],
    routines: dict[str, Routine],
    exercises: dict[str, Exercise],
) -> list[Workout]:
    return [
        Workout(
            user_id=profile.id,
            date=record.date,
            notes=record.notes,
            routine=routines[record.routine],
            elements=_elements(record, exercises),
            exercise_notes=[
                WorkoutExerciseNote(exercise=exercises[name], notes=notes)
                for name, notes in record.exercise_notes
            ],
        )
        for record in records
    ]


def _elements(record: _Record, exercises: dict[str, Exercise]) -> list[WorkoutElement]:
    elements: list[WorkoutElement] = []

    for performed in record.sets:
        elements.append(
            WorkoutSet(
                position=len(elements) + 1,
                exercise=exercises[performed.exercise],
                reps=performed.reps,
                time=performed.time,
                weight=performed.weight,
                rpe=performed.rpe,
                target_reps=performed.target_reps,
                target_time=performed.target_time,
                target_weight=performed.target_weight,
                target_rpe=performed.target_rpe,
            )
        )
        if record.rests:
            elements.append(
                WorkoutRest(
                    position=len(elements) + 1,
                    target_time=performed.rest,
                    automatic=performed.rest <= AUTOMATIC_REST,
                )
            )

    return elements


def _schedule(
    profile: Profile, routines: dict[str, Routine]
) -> tuple[list[ScheduleRotation], list[ScheduleSlot]]:
    schedule = profile.schedule

    if schedule.rotation is None:
        return (
            [],
            [
                ScheduleSlot(
                    user_id=profile.id,
                    weekday=weekday,
                    position=1,
                    routine_id=routines[schedule.routines[slot]].id,
                )
                for slot, weekday in enumerate(schedule.weekdays)
            ],
        )

    rotation = ScheduleRotation(
        id=1,
        user_id=profile.id,
        name=schedule.rotation,
        routines=[
            ScheduleRotationRoutine(position=position, routine_id=routines[name].id)
            for position, name in enumerate(schedule.routines, start=1)
        ],
    )
    return (
        [rotation],
        [
            ScheduleSlot(user_id=profile.id, weekday=weekday, position=1, rotation_id=rotation.id)
            for weekday in schedule.weekdays
        ],
    )
