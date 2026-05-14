from __future__ import annotations

import tests.data
from valens.models import Exercise, RoutineActivity, RoutineSection, WorkoutSet


def test_data_integrity() -> None:  # noqa: C901
    for user in tests.data.users():
        user_exercises = set(user.exercises)
        user_routine_ids = {r.id for r in user.routines}

        for bw in user.body_weight:
            assert bw.user_id == user.id, f"BodyWeight {bw.date} has wrong user_id"

        for bf in user.body_fat:
            assert bf.user_id == user.id, f"BodyFat {bf.date} has wrong user_id"

        for exercise in user.exercises:
            assert exercise.user_id == user.id, f"Exercise {exercise.id} has wrong user_id"
            for muscle in exercise.muscles:
                assert (
                    muscle.user_id == user.id
                ), f"ExerciseMuscle on exercise {exercise.id} has wrong user_id"

        for routine in user.routines:
            assert routine.user_id == user.id, f"Routine {routine.id} has wrong user_id"
            for section in routine.sections:
                _check_section(section, user_exercises)

        for workout in user.workouts:
            assert workout.user_id == user.id, f"Workout {workout.id} has wrong user_id"
            if workout.routine_id is not None:
                assert workout.routine_id in user_routine_ids, (
                    f"Workout {workout.id} references routine {workout.routine_id} "
                    f"not owned by user {user.id}"
                )
            for element in workout.elements:
                if isinstance(element, WorkoutSet) and element.exercise is not None:
                    assert element.exercise in user_exercises, (
                        f"Workout {workout.id} set at position {element.position} "
                        f"references exercise {element.exercise.id} "
                        f"(user_id={element.exercise.user_id}) not owned by user {user.id}"
                    )


def _check_section(section: RoutineSection, user_exercises: set[Exercise]) -> None:
    for part in section.parts:
        if isinstance(part, RoutineActivity):
            if part.exercise is not None:
                assert part.exercise in user_exercises, (
                    f"RoutineActivity in routine section references exercise "
                    f"{part.exercise.id} (user_id={part.exercise.user_id}) "
                    f"not owned by the user"
                )
        elif isinstance(part, RoutineSection):
            _check_section(part, user_exercises)
