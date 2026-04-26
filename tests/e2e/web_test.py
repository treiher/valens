from __future__ import annotations

import os
from collections.abc import Generator
from pathlib import Path
from subprocess import PIPE, STDOUT, Popen
from tempfile import TemporaryDirectory

import pytest
from playwright.sync_api import Page

import tests.utils
from valens import app, models
from valens.config import create_config_file

from .const import (
    BASE_URL,
    CURRENT_WORKOUT_EXERCISES,
    PORT,
    PREVIOUS_WORKOUT_EXERCISES,
    TODAY,
    USER,
    USERNAMES,
    VALENS,
)
from .io import wait_for_output
from .pages import (
    BodyFatPage,
    BodyWeightPage,
    ExercisePage,
    ExercisesPage,
    HomePage,
    LoginPage,
    MenstrualCyclePage,
    MusclesPage,
    RoutinePage,
    RoutineRest,
    RoutineSet,
    RoutinesPage,
    TrainingPage,
    TrainingSessionPage,
)


@pytest.fixture(autouse=True)
def _backend_server() -> Generator[None, None, None]:
    """Start the backend server with a fresh database for each test."""

    with TemporaryDirectory() as tmp_dir:
        data_dir = Path(tmp_dir)
        db_file = data_dir / "test.db"
        config = create_config_file(data_dir, db_file)

        with app.app_context():
            app.config["DATABASE"] = f"sqlite:///{db_file}"
            app.config["SECRET_KEY"] = b"TEST_KEY"
            tests.utils.init_db_data(today=TODAY)

        with Popen(
            f"{VALENS} run --port {PORT}".split(),
            stdout=PIPE,
            stderr=STDOUT,
            env={"VALENS_CONFIG": str(config), **os.environ},
        ) as p:
            assert p.stdout
            wait_for_output(p.stdout, "Running on")
            yield
            p.terminate()


def login(page: Page) -> None:
    login_page = LoginPage(page)
    login_page.goto()
    login_page.login(USERNAMES[0])


def test_login(page: Page) -> None:
    login_page = LoginPage(page)
    login_page.goto()

    assert login_page.users() == USERNAMES

    login_page.login(USERNAMES[0])

    HomePage(page).expect_page()


def test_logout(page: Page) -> None:
    login(page)

    home_page = HomePage(page)
    home_page.expect_page()
    home_page.logout()

    LoginPage(page).expect_page()


def test_home_links(page: Page) -> None:
    login(page)

    home_page = HomePage(page)
    home_page.expect_page()

    home_page.go_to_training()
    training_page = TrainingPage(page)
    training_page.expect_page()
    training_page.go_back()
    home_page.expect_page()

    home_page.go_to_routines()
    routine_page = RoutinesPage(page)
    routine_page.expect_page()
    routine_page.go_back()
    home_page.expect_page()

    home_page.go_to_exercises()
    exercises_page = ExercisesPage(page)
    exercises_page.expect_page()
    exercises_page.go_back()
    home_page.expect_page()

    home_page.go_to_muscles()
    muscles_page = MusclesPage(page)
    muscles_page.expect_page()
    muscles_page.go_back()
    home_page.expect_page()

    home_page.go_to_body_weight()
    body_weight_page = BodyWeightPage(page)
    body_weight_page.expect_page()
    body_weight_page.go_back()
    home_page.expect_page()

    home_page.go_to_body_fat()
    body_fat_page = BodyFatPage(page)
    body_fat_page.expect_page()
    body_fat_page.go_back()
    home_page.expect_page()

    home_page.go_to_menstrual_cycle()
    menstrual_cycle_page = MenstrualCyclePage(page)
    menstrual_cycle_page.expect_page()
    menstrual_cycle_page.go_back()
    home_page.expect_page()


def test_body_weight_add(page: Page) -> None:
    login(page)
    p = BodyWeightPage(page)
    p.goto()
    p.fab().click()
    p.dialog.wait_until_open()

    date = p.dialog.get_date()
    weight = "123.4"

    p.dialog.cancel()

    assert p.table.get_value(1, 1, 1) != date
    assert p.table.get_value(1, 1, 2) != weight

    p.fab().click()
    p.dialog.wait_until_open()

    p.dialog.set_weight(weight)

    assert p.table.get_value(1, 1, 1) != date
    assert p.table.get_value(1, 1, 2) != weight

    p.dialog.save()

    p.table.expect_value(1, 1, 1, date)
    p.table.expect_value(1, 1, 2, weight)


def test_body_weight_edit(page: Page) -> None:
    date = str(USER.body_weight[-1].date)
    weight = str(USER.body_weight[-1].weight)
    new_weight = "123.4"

    login(page)
    p = BodyWeightPage(page)
    p.goto()

    p.table.expect_value(1, 1, 1, date)
    p.table.expect_value(1, 1, 2, weight)

    p.edit_item(0)
    p.dialog.set_weight(new_weight)
    p.dialog.cancel()

    p.table.expect_value(1, 1, 1, date)
    p.table.expect_value(1, 1, 2, weight)

    p.edit_item(0)
    p.dialog.set_weight(new_weight)
    p.dialog.save()

    p.table.expect_value(1, 1, 1, date)
    p.table.expect_value(1, 1, 2, new_weight)


def test_body_weight_delete(page: Page) -> None:
    date_1 = str(USER.body_weight[-1].date)
    weight = str(USER.body_weight[-1].weight)
    date_2 = str(USER.body_weight[-2].date)

    login(page)
    p = BodyWeightPage(page)
    p.goto()

    p.table.expect_value(1, 1, 1, date_1)
    p.table.expect_value(1, 1, 2, weight)

    p.delete_item(0)
    p.dialog.no()

    p.table.expect_value(1, 1, 1, date_1)
    p.table.expect_value(1, 1, 2, weight)

    p.delete_item(0)
    p.dialog.delete()

    p.table.expect_value(1, 1, 1, date_2)


def test_body_fat_add(page: Page) -> None:
    login(page)
    p = BodyFatPage(page)
    p.goto()
    p.fab().click()
    p.dialog.wait_until_open()

    p.dialog.cancel()

    current_values = {
        v.date.isoformat(): {
            "tricep": v.tricep or "-",
            "suprailiac": v.suprailiac or "-",
            "thigh": v.thigh or "-",
            "chest": v.chest or "-",
            "abdominal": v.abdominal or "-",
            "subscapular": v.subscapular or "-",
            "midaxillary": v.midaxillary or "-",
        }
        for v in USER.body_fat
    }
    headers = {k.lower().split(" ")[0]: v for k, v in p.table.get_headers().items()}

    for row in range(1, 2):
        row_date = p.table.get_value(1, row, 1)
        for entry, value in current_values[row_date].items():
            assert p.table.get_value(1, row, headers[entry]) == str(value)

    items = (
        "tricep",
        "suprailiac",
        "thigh",
        "chest",
        "abdominal",
        "subscapular",
        "midaxillary",
    )
    values = ("1", "2", "3", "4", "5", "6", "7")

    p.fab().click()
    p.dialog.wait_until_open()
    p.dialog.set_jp7(values)
    p.dialog.save()

    p.table.expect_value(1, 1, 1, TODAY.isoformat())
    for item, value in zip(items, values, strict=False):
        p.table.expect_value(1, 1, headers[item], value)


def test_body_fat_edit(page: Page) -> None:
    body_fat = USER.body_fat[-1]
    date = str(body_fat.date)
    values = (
        str(body_fat.tricep) if body_fat.tricep else "-",
        str(body_fat.suprailiac) if body_fat.suprailiac else "-",
        str(body_fat.thigh) if body_fat.thigh else "-",
        str(body_fat.chest) if body_fat.chest else "-",
        str(body_fat.abdominal) if body_fat.abdominal else "-",
        str(body_fat.subscapular) if body_fat.subscapular else "-",
        str(body_fat.midaxillary) if body_fat.midaxillary else "-",
    )
    new_values = ("1", "2", "3", "4", "5", "6", "7")

    login(page)
    p = BodyFatPage(page)
    p.goto()

    p.table.expect_value(1, 1, 1, date)
    for i, v in enumerate(values, start=4):
        p.table.expect_value(1, 1, i, v)

    p.edit_item(0)
    p.dialog.set_jp7(new_values)
    p.dialog.cancel()

    p.table.expect_value(1, 1, 1, date)
    for i, v in enumerate(values, start=4):
        p.table.expect_value(1, 1, i, v)

    p.edit_item(0)
    p.dialog.set_jp7(new_values)
    p.dialog.save()

    p.table.expect_value(1, 1, 1, date)
    for i, v in enumerate(new_values, start=4):
        p.table.expect_value(1, 1, i, v)


def test_body_fat_delete(page: Page) -> None:
    date_1 = str(USER.body_fat[-1].date)
    date_2 = str(USER.body_fat[-2].date)

    login(page)
    p = BodyFatPage(page)
    p.goto()

    p.table.expect_value(1, 1, 1, date_1)

    p.delete_item(0)
    p.dialog.no()

    p.table.expect_value(1, 1, 1, date_1)

    p.delete_item(0)
    p.dialog.delete()

    p.table.expect_value(1, 1, 1, date_2)


def test_menstrual_cycle_add(page: Page) -> None:
    login(page)
    p = MenstrualCyclePage(page)
    p.goto()
    p.fab().click()
    p.dialog.wait_until_open()

    date = p.dialog.get_date()
    intensity = "4"

    p.dialog.cancel()

    assert p.table.get_value(1, 1, 1) != date
    assert p.table.get_value(1, 1, 2) != intensity

    p.fab().click()
    p.dialog.wait_until_open()

    p.dialog.set_intensity(intensity)

    assert p.table.get_value(1, 1, 1) != date
    assert p.table.get_value(1, 1, 2) != intensity

    p.dialog.save()

    p.table.expect_value(1, 1, 1, date)
    p.table.expect_value(1, 1, 2, intensity)


def test_menstrual_cycle_edit(page: Page) -> None:
    period = USER.period[-1]
    date = str(period.date)
    intensity = str(period.intensity)
    new_intensity = "4"

    assert intensity != new_intensity

    login(page)
    p = MenstrualCyclePage(page)
    p.goto()

    p.table.expect_value(1, 1, 1, date)
    p.table.expect_value(1, 1, 2, intensity)

    p.edit_item(0)
    p.dialog.set_intensity(new_intensity)
    p.dialog.cancel()

    p.table.expect_value(1, 1, 1, date)
    p.table.expect_value(1, 1, 2, intensity)

    p.edit_item(0)
    p.dialog.set_intensity(new_intensity)
    p.dialog.save()

    p.table.expect_value(1, 1, 1, date)
    p.table.expect_value(1, 1, 2, new_intensity)


def test_menstrual_cycle_delete(page: Page) -> None:
    period = USER.period[-1]
    date_1 = str(period.date)
    intensity = str(period.intensity)
    date_2 = str(USER.period[-2].date)

    login(page)
    p = MenstrualCyclePage(page)
    p.goto()

    p.table.expect_value(1, 1, 1, date_1)
    p.table.expect_value(1, 1, 2, intensity)

    p.delete_item(0)
    p.dialog.no()

    p.table.expect_value(1, 1, 1, date_1)
    p.table.expect_value(1, 1, 2, intensity)

    p.delete_item(0)
    p.dialog.delete()

    p.table.expect_value(1, 1, 1, date_2)


def test_training_add(page: Page) -> None:
    routine = USER.routines[-1].name

    login(page)
    training_page = TrainingPage(page)
    training_page.goto()
    training_page.fab().click()
    training_page.dialog.wait_until_open()
    date = training_page.dialog.get_date()
    training_page.dialog.cancel()

    assert training_page.table.get_value(1, 1, 1) != date
    assert training_page.table.get_value(1, 1, 2) != routine

    training_page.add_training_session(routine)

    session_page = TrainingSessionPage(page, 0)
    session_page.expect_page()

    session_page.go_back()

    training_page.expect_page()
    assert training_page.table.get_value(1, 1, 1) == date
    assert training_page.table.get_value(1, 1, 2) == routine


def test_training_delete(page: Page) -> None:
    workout = USER.workouts[-1]
    date_1 = str(workout.date)
    routine = (
        next(r.name for r in USER.routines if r.id == workout.routine_id)
        if workout.routine_id
        else "-"
    )
    date_2 = str(USER.workouts[-2].date)

    login(page)
    p = TrainingPage(page)
    p.goto()

    p.table.expect_value(1, 1, 1, date_1)
    p.table.expect_value(1, 1, 2, routine)

    p.delete_item(0)
    p.dialog.no()

    p.table.expect_value(1, 1, 1, date_1)
    p.table.expect_value(1, 1, 2, routine)

    p.delete_item(0)
    p.dialog.delete()

    p.table.expect_value(1, 1, 1, date_2)


def test_training_session(page: Page) -> None:
    workout = USER.workouts[-1]
    sets = [
        (
            s.reps,
            s.time,
            s.weight,
            s.rpe,
        )
        for s in workout.elements
        if isinstance(s, models.WorkoutSet)
    ]

    login(page)
    p = TrainingSessionPage(page, workout.id)
    p.goto()

    assert p.get_sets() == sets

    p.edit()


def test_training_session_change_entries(page: Page) -> None:
    workout = USER.workouts[-1]
    sets = [
        (
            s.reps,
            s.time,
            s.weight,
            s.rpe,
        )
        for s in workout.elements
        if isinstance(s, models.WorkoutSet)
    ]
    new_values = (1, 2, 3, 4)

    login(page)
    p = TrainingSessionPage(page, workout.id)
    p.goto()
    p.edit()

    assert p.get_form() == sets

    p.set_form(0, new_values)
    assert p.get_form() == [new_values, *sets[1:]]

    p.reload()

    p.expect_page()
    assert p.get_form() == sets

    p.set_form(0, new_values)
    assert p.get_form() == [new_values, *sets[1:]]

    p.save()
    p.goto()

    assert p.get_form() == [new_values, *sets[1:]]


def test_training_session_change_notes(page: Page) -> None:
    workout = USER.workouts[0]
    sets = [
        (
            s.reps,
            s.time,
            s.weight,
            s.rpe,
        )
        for s in workout.elements
        if isinstance(s, models.WorkoutSet)
    ]
    notes = workout.notes if workout.notes is not None else ""
    new_notes = "Test"

    assert notes != new_notes

    login(page)
    p = TrainingSessionPage(page, workout.id)
    p.goto()
    p.edit()

    assert p.get_form() == sets
    assert p.get_notes() == notes

    p.set_notes(new_notes)
    assert p.get_notes() == new_notes

    p.reload()
    p.expect_page()
    assert p.get_notes() == notes

    p.set_notes(new_notes)
    assert p.get_notes() == new_notes

    p.save()
    p.goto()

    assert p.get_notes() == new_notes


def test_routines_add(page: Page) -> None:
    name = USER.routines[-1].name
    new_name = "New Routine"

    assert name != new_name

    login(page)
    p = RoutinesPage(page)
    p.goto()
    p.fab().click()
    p.dialog.wait_until_open()

    p.dialog.cancel()

    p.table.expect_value(1, 2, 1, name)

    p.fab().click()
    p.dialog.wait_until_open()

    p.dialog.set_name(new_name)
    p.dialog.save()

    p.table.expect_value(1, 2, 1, new_name)


def test_routines_edit(page: Page) -> None:
    name = str(USER.routines[-2].name)
    new_name = "Changed Routine"

    assert name != new_name

    login(page)
    p = RoutinesPage(page)
    p.goto()

    p.table.expect_value(1, 1, 1, name)

    p.rename_item(0)
    p.dialog.set_name(new_name)
    p.dialog.cancel()

    p.table.expect_value(1, 1, 1, name)

    p.rename_item(0)
    p.dialog.set_name(new_name)
    p.dialog.save()

    p.table.expect_value(1, 1, 1, new_name)


def test_routines_delete(page: Page) -> None:
    name_1 = str(USER.routines[-1].name)
    name_2 = str(USER.routines[-2].name)

    login(page)
    p = RoutinesPage(page)
    p.goto()

    p.table.expect_value(1, 1, 1, name_2)
    p.table.expect_value(1, 2, 1, name_1)

    p.delete_item(0)
    p.dialog.no()

    p.table.expect_value(1, 1, 1, name_2)
    p.table.expect_value(1, 2, 1, name_1)

    p.delete_item(0)
    p.dialog.delete()

    p.table.expect_value(1, 1, 1, name_1)


def test_routine_edit(page: Page) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    p.wait_for_link(exercise_1)
    p.wait_for_link(exercise_2)

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_1

    p.set_rounds(0, 8)
    p.replace_exercise(0, 0, exercise_2)
    p.set_reps(0, 0, "10")
    p.set_time(0, 0, "4")
    p.set_weight(0, 0, "18")
    p.set_rpe(0, 0, "8")

    sections = p.get_sections()
    section = sections[0]
    assert section.rounds == 8
    assert section.get_set_at(0) == RoutineSet(exercise_2, 10, 4.0, 18.0, 8.0)
    assert section.get_rest_at(1) == RoutineRest(30)

    p.set_rounds(0, 8)
    p.replace_exercise(0, 0, exercise_1)
    p.set_reps(0, 0, "")
    p.set_time(0, 0, "60")
    p.set_weight(0, 0, "5.5")
    p.set_rpe(0, 0, "8.5")
    p.set_automatic(0, 0)

    sections = p.get_sections()
    section = sections[0]
    assert section.rounds == 8
    assert section.get_set_at(0) == RoutineSet(exercise_1, None, 60.0, 5.5, 8.5)
    assert section.get_rest_at(1) == RoutineRest(30)


def test_routine_create_exercise(page: Page) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    new_exercise = "New Exercise"

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    p.wait_for_link(exercise_1)
    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_1

    p.replace_with_new_exercise(0, 0, new_exercise)

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == new_exercise


def test_routine_add_section(page: Page) -> None:
    routine = USER.routines[0]
    section_rounds = [str(s.rounds) for s in routine.sections]

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    assert [str(s.rounds) for s in p.get_sections()] == section_rounds

    p.add_section(4)
    p.add_section(3)
    p.add_section(2)
    p.add_section(1)
    p.add_section(0)

    sections = p.get_sections()
    assert len(sections) == 4


def test_routine_add_exercise(page: Page) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)
    new_exercise = USER.exercises[-1].name

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    p.add_exercise(0, new_exercise)
    p.add_exercise(1, new_exercise)
    p.add_exercise(2, new_exercise)
    p.add_exercise(3, new_exercise)

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_1
    assert sections[0].get_set_at(2).exercise_name == new_exercise
    assert sections[1].get_section_at(2).get_set_at(2).exercise_name == new_exercise
    assert sections[1].get_set_at(0).exercise_name == exercise_2
    assert sections[1].get_set_at(3).exercise_name == new_exercise
    assert sections[2].get_set_at(2).exercise_name == new_exercise


def test_routine_add_rest(page: Page) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_1
    assert sections[1].get_set_at(0).exercise_name == exercise_2
    assert not sections[0].has_part_at(2)
    assert not sections[1].has_part_at(3)
    assert not sections[1].get_section_at(2).has_part_at(2)
    assert not sections[2].has_part_at(2)

    p.add_rest(0)
    p.add_rest(1)
    p.add_rest(2)
    p.add_rest(3)

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_1
    assert sections[1].get_set_at(0).exercise_name == exercise_2
    assert sections[0].is_rest_at(2)
    assert sections[1].is_rest_at(3)
    assert sections[1].get_section_at(2).is_rest_at(2)
    assert sections[2].is_rest_at(2)


def test_routine_move_section_up(page: Page) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)
    assert isinstance(routine.sections[2].parts[0], models.RoutineActivity)
    exercise_3 = str(routine.sections[2].parts[0].exercise.name)

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_1
    assert sections[1].get_set_at(0).exercise_name == exercise_2
    assert sections[2].get_set_at(0).exercise_name == exercise_3

    p.move_up(0)

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_2
    assert sections[1].get_set_at(0).exercise_name == exercise_3
    assert sections[2].get_set_at(0).exercise_name == exercise_1

    p.move_up(0)

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_3
    assert sections[1].get_set_at(0).exercise_name == exercise_1
    assert sections[2].get_set_at(0).exercise_name == exercise_2

    p.move_up(0)

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_1
    assert sections[1].get_set_at(0).exercise_name == exercise_2
    assert sections[2].get_set_at(0).exercise_name == exercise_3


def test_routine_move_section_down(page: Page) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_1
    assert sections[1].get_set_at(0).exercise_name == exercise_2

    p.move_down(1)
    p.move_down(0)

    sections = p.get_sections()
    assert sections[1].get_set_at(0).exercise_name == exercise_1
    assert sections[2].get_set_at(0).exercise_name == exercise_2

    p.move_down(2)
    p.move_down(2)

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_2
    assert sections[1].get_set_at(0).exercise_name == exercise_1


def test_routine_move_nested_section_up(page: Page) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[1].parts[2], models.RoutineSection)
    assert isinstance(routine.sections[1].parts[2].parts[0], models.RoutineActivity)
    exercise = str(routine.sections[1].parts[2].parts[0].exercise.name)

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    sections = p.get_sections()
    assert sections[1].get_section_at(2).get_set_at(0).exercise_name == exercise

    p.move_up(2)

    sections = p.get_sections()
    assert sections[1].get_section_at(1).get_set_at(0).exercise_name == exercise

    p.move_up(2)

    sections = p.get_sections()
    assert sections[1].get_section_at(0).get_set_at(0).exercise_name == exercise

    p.move_up(2)

    sections = p.get_sections()
    assert sections[1].get_section_at(2).get_set_at(0).exercise_name == exercise


def test_routine_move_nested_section_down(page: Page) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[1].parts[2], models.RoutineSection)
    assert isinstance(routine.sections[1].parts[2].parts[0], models.RoutineActivity)
    exercise = str(routine.sections[1].parts[2].parts[0].exercise.name)

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    sections = p.get_sections()
    assert sections[1].get_section_at(2).get_set_at(0).exercise_name == exercise

    p.move_down(2)

    sections = p.get_sections()
    assert sections[1].get_section_at(0).get_set_at(0).exercise_name == exercise

    p.move_down(2)

    sections = p.get_sections()
    assert sections[1].get_section_at(1).get_set_at(0).exercise_name == exercise

    p.move_down(2)

    sections = p.get_sections()
    assert sections[1].get_section_at(2).get_set_at(0).exercise_name == exercise


def test_routine_move_exercise_up(page: Page) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_1
    assert sections[1].get_set_at(0).exercise_name == exercise_2

    p.move_up(0, 0)
    p.move_up(1, 0)

    sections = p.get_sections()
    assert sections[0].get_set_at(1).exercise_name == exercise_1
    assert sections[1].get_set_at(2).exercise_name == exercise_2

    p.move_up(0, 1)
    p.move_up(1, 3)

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_1
    assert sections[1].get_set_at(1).exercise_name == exercise_2


def test_routine_move_exercise_down(page: Page) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_1
    assert sections[1].get_set_at(0).exercise_name == exercise_2

    p.move_down(0, 0)
    p.move_down(1, 0)

    sections = p.get_sections()
    assert sections[0].get_set_at(1).exercise_name == exercise_1
    assert sections[1].get_set_at(1).exercise_name == exercise_2

    p.move_down(0, 1)
    p.move_down(1, 1)

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise_1
    assert sections[1].get_set_at(2).exercise_name == exercise_2


def test_routine_remove_section(page: Page) -> None:
    routine = USER.routines[0]
    section_rounds = [str(s.rounds) for s in routine.sections]

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    assert len(p.get_sections()) == len(section_rounds)

    p.remove(0)

    assert [str(s.rounds) for s in p.get_sections()] == section_rounds[1:]

    p.remove(2)

    assert [str(s.rounds) for s in p.get_sections()] == section_rounds[1:2]


def test_routine_remove_activity(page: Page) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise = str(routine.sections[0].parts[0].exercise.name)

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise
    assert sections[0].is_rest_at(1)
    assert len(sections[0].parts) == 2

    p.remove(0, 1)

    sections = p.get_sections()
    assert sections[0].get_set_at(0).exercise_name == exercise
    assert len(sections[0].parts) == 1

    p.remove(0, 0)

    sections = p.get_sections()
    assert len(sections[0].parts) == 0


def test_routine_delete_training_session(page: Page) -> None:
    routine = USER.routines[0]
    workouts = sorted(
        {w for w in USER.workouts if w.routine_id == routine.id}, key=lambda x: x.date
    )
    workout_1 = str(workouts[-1].date)
    workout_2 = str(workouts[-2].date)

    login(page)
    p = RoutinePage(page, routine.id)
    p.goto()

    p.wait_for_link(workout_1)
    p.wait_for_link(workout_2)

    p.delete_item(0)
    p.dialog.no()

    p.wait_for_link(workout_1)
    p.wait_for_link(workout_2)

    p.delete_item(0)
    p.dialog.delete()

    p.wait_for_link_not_present(workout_1)
    p.wait_for_link(workout_2)


def test_exercises_add(page: Page) -> None:
    exercise = sorted(USER.exercises, key=lambda x: x.name)[1]
    name = exercise.name
    new_name = "A"

    assert new_name != name

    login(page)
    p = ExercisesPage(page)
    p.goto()

    expected_current = {e.name for e in USER.exercises if e.name in CURRENT_WORKOUT_EXERCISES}
    expected_previous = {e.name for e in USER.exercises if e.name in PREVIOUS_WORKOUT_EXERCISES}

    assert {e[0] for e in p.table.get_body(1)} == expected_current
    assert {e[0] for e in p.table.get_body(2)} == expected_previous

    p.table.expect_value(1, 1, 1, name)

    p.add_exercise(new_name)

    p.table.expect_value(1, 1, 1, new_name)

    assert {e[0] for e in p.table.get_body(1)} == {new_name, *expected_current}
    assert {e[0] for e in p.table.get_body(2)} == expected_previous


def test_exercises_rename(page: Page) -> None:
    current_name = sorted(CURRENT_WORKOUT_EXERCISES)[0]
    new_name = "Changed Exercise"

    assert current_name != new_name

    login(page)
    p = ExercisesPage(page)
    p.goto()

    p.table.expect_value(1, 1, 1, current_name)

    p.rename_item(0)
    p.dialog.set_name(new_name)
    p.dialog.cancel()

    p.table.expect_value(1, 1, 1, current_name)

    p.rename_item(0)
    p.dialog.set_name(new_name)
    p.dialog.save()

    p.table.expect_value(1, 1, 1, new_name)


def test_exercises_delete(page: Page) -> None:
    current_exercises = sorted(CURRENT_WORKOUT_EXERCISES)
    current_name_1 = current_exercises[0]
    current_name_2 = current_exercises[1]

    login(page)
    p = ExercisesPage(page)
    p.goto()

    p.table.expect_value(1, 1, 1, current_name_1)

    p.delete_item(0)
    p.dialog.no()

    p.table.expect_value(1, 1, 1, current_name_1)

    p.delete_item(0)
    p.dialog.delete()

    p.table.expect_value(1, 1, 1, current_name_2)

    previous_name = sorted(PREVIOUS_WORKOUT_EXERCISES)[0]

    p.table.expect_value(2, 1, 1, previous_name)

    p.delete_item(0)
    p.dialog.no()

    p.table.expect_value(2, 1, 1, previous_name)

    p.delete_item(0)
    p.dialog.delete()

    assert p.table.get_body(1) == [[previous_name, ""]]


def test_exercise_delete_workout(page: Page) -> None:
    exercise = sorted(USER.exercises, key=lambda x: x.name)[1]
    workouts = sorted({ws.workout for ws in exercise.sets}, key=lambda x: x.date)
    workout_1 = str(workouts[-1].date)
    workout_2 = str(workouts[-2].date)

    login(page)
    p = ExercisePage(page, exercise.id)
    p.goto()

    p.table.expect_value(1, 1, 1, workout_1)

    p.delete_item(0)
    p.dialog.no()

    p.table.expect_value(1, 1, 1, workout_1)

    p.delete_item(0)
    p.dialog.delete()

    p.table.expect_value(1, 1, 1, workout_2)


def test_muscles(page: Page) -> None:
    login(page)
    p = MusclesPage(page)
    p.goto()


def test_cache(page: Page) -> None:
    page.goto(BASE_URL)
    page.wait_for_function("() => navigator.serviceWorker.ready.then(() => true)")
    page.reload()
    page.wait_for_function("() => navigator.serviceWorker.controller !== null")

    client = page.context.new_cdp_session(page)
    caches = client.send("CacheStorage.requestCacheNames", {"securityOrigin": BASE_URL})["caches"]
    cached_files = [
        entry["requestURL"].split("/")[-1]
        for cache in caches
        for entry in client.send(
            "CacheStorage.requestEntries",
            {"cacheId": cache["cacheId"]},
        )["cacheDataEntries"]
    ]
    expected_files = [
        "",
        "Roboto-Bold.woff",
        "Roboto-BoldItalic.woff",
        "Roboto-Italic.woff",
        "Roboto-Regular.woff",
        "fa-solid-900.ttf",
        "fa-solid-900.woff2",
        "android-chrome-192x192.png",
        "android-chrome-512x512.png",
        "apple-touch-icon.png",
        "favicon-16x16.png",
        "favicon-32x32.png",
        "main.css",
        "manifest.json",
        "sw.js",
        "valens-web-app-dioxus.js",
        "valens-web-app-dioxus_bg.wasm",
    ]
    assert cached_files == expected_files
