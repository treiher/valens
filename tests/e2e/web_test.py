from __future__ import annotations

import os
from collections.abc import Generator
from pathlib import Path
from subprocess import PIPE, STDOUT, Popen
from tempfile import TemporaryDirectory

import pytest
from selenium import webdriver

import tests.data
import tests.utils
from valens import app, models
from valens.config import create_config_file

from .const import PORT, VALENS
from .io import wait_for_output
from .page import (
    BodyFatPage,
    BodyWeightPage,
    ExercisePage,
    ExercisesPage,
    HomePage,
    LoginPage,
    MenstrualCyclePage,
    RoutinePage,
    RoutinesPage,
    TrainingPage,
    TrainingSessionPage,
)

USERS = tests.data.users()
USER = USERS[0]
USERNAMES = [user.name for user in USERS]


@pytest.fixture(autouse=True)
def _fixture_backend() -> Generator[None, None, None]:
    with TemporaryDirectory() as tmp_dir:
        data_dir = Path(tmp_dir)
        db_file = data_dir / "test.db"
        config = create_config_file(data_dir, db_file)

        with app.app_context():
            app.config["DATABASE"] = f"sqlite:///{db_file}"
            app.config["SECRET_KEY"] = b"TEST_KEY"
            tests.utils.init_db_data()

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


@pytest.fixture(name="driver_args")
def fixture_driver_args() -> list[str]:
    return ["--log-level=ALL"]


@pytest.fixture(name="session_capabilities", scope="session")
def fixture_session_capabilities(
    session_capabilities: webdriver.DesiredCapabilities.CHROME,
) -> Generator[webdriver.DesiredCapabilities.CHROME, None, None]:
    session_capabilities["loggingPrefs"] = {"browser": "ALL"}
    return session_capabilities


def login(driver: webdriver.Chrome) -> None:
    login_page = LoginPage(driver)
    login_page.load()
    login_page.login(USERNAMES[0])


def test_login(driver: webdriver.Chrome) -> None:
    login_page = LoginPage(driver)
    login_page.load()

    assert login_page.users() == USERNAMES

    login_page.login(USERNAMES[0])


def test_home_links(driver: webdriver.Chrome) -> None:
    login(driver)

    home_page = HomePage(driver, USERNAMES[0])

    home_page.click_training()
    training_page = TrainingPage(driver)
    training_page.wait_until_loaded()
    training_page.click_up_button()
    home_page.wait_until_loaded()

    home_page.click_body_weight()
    body_weight_page = BodyWeightPage(driver)
    body_weight_page.wait_until_loaded()
    body_weight_page.click_up_button()
    home_page.wait_until_loaded()

    home_page.click_body_fat()
    body_fat_page = BodyFatPage(driver)
    body_fat_page.wait_until_loaded()
    body_fat_page.click_up_button()
    home_page.wait_until_loaded()

    home_page.click_menstrual_cycle()
    menstrual_cycle_page = MenstrualCyclePage(driver)
    menstrual_cycle_page.wait_until_loaded()
    menstrual_cycle_page.click_up_button()
    home_page.wait_until_loaded()


def test_body_weight_add(driver: webdriver.Chrome) -> None:
    login(driver)
    page = BodyWeightPage(driver)
    page.load()
    page.click_fab()

    page.wait_for_dialog()

    date = page.body_weight_dialog.get_date()
    weight = "123.4"

    page.body_weight_dialog.click_cancel()

    assert page.get_table_value(1) != date
    assert page.get_table_value(2) != weight

    page.click_fab()

    page.wait_for_dialog()

    page.body_weight_dialog.set_weight(weight)

    assert page.get_table_value(1) != date
    assert page.get_table_value(2) != weight

    page.body_weight_dialog.click_save()

    page.wait_for_table_value(1, date)
    page.wait_for_table_value(2, weight)


def test_body_weight_edit(driver: webdriver.Chrome) -> None:
    date = str(USER.body_weight[-1].date)
    weight = str(USER.body_weight[-1].weight)
    new_weight = "123.4"

    login(driver)
    page = BodyWeightPage(driver)
    page.load()

    page.wait_for_table_value(1, date)
    page.wait_for_table_value(2, weight)

    page.click_edit(0)
    page.body_weight_dialog.set_weight(new_weight)
    page.body_weight_dialog.click_cancel()

    page.wait_for_table_value(1, date)
    page.wait_for_table_value(2, weight)

    page.click_edit(0)
    page.body_weight_dialog.set_weight(new_weight)
    page.body_weight_dialog.click_save()

    page.wait_for_table_value(1, date)
    page.wait_for_table_value(2, new_weight)


def test_body_weight_delete(driver: webdriver.Chrome) -> None:
    date_1 = str(USER.body_weight[-1].date)
    weight = str(USER.body_weight[-1].weight)
    date_2 = str(USER.body_weight[-2].date)

    login(driver)
    page = BodyWeightPage(driver)
    page.load()

    page.wait_for_table_value(1, date_1)
    page.wait_for_table_value(2, weight)

    page.click_delete(0)
    page.delete_dialog.click_no()

    page.wait_for_table_value(1, date_1)
    page.wait_for_table_value(2, weight)

    page.click_delete(0)
    page.delete_dialog.click_yes()

    page.wait_for_table_value(1, date_2)


def test_body_fat_add(driver: webdriver.Chrome) -> None:
    login(driver)
    page = BodyFatPage(driver)
    page.load()
    page.click_fab()

    page.wait_for_dialog()

    date = page.body_fat_dialog.get_date()
    values = ("1", "2", "3", "4", "5", "6", "7")

    page.body_fat_dialog.click_cancel()

    assert page.get_table_value(1) != date
    for i, v in enumerate(values, start=4):
        assert page.get_table_value(i) != v

    page.click_fab()

    page.wait_for_dialog()

    page.body_fat_dialog.set_jp7(values)

    assert page.get_table_value(1) != date
    for i, v in enumerate(values, start=4):
        assert page.get_table_value(i) != v

    page.body_fat_dialog.click_save()

    page.wait_for_table_value(1, date)
    for i, v in enumerate(values, start=4):
        page.wait_for_table_value(i, v)


def test_body_fat_edit(driver: webdriver.Chrome) -> None:
    body_fat = USER.body_fat[-1]
    date = str(body_fat.date)
    values = (
        str(body_fat.tricep) if body_fat.tricep else "-",
        str(body_fat.suprailiac) if body_fat.suprailiac else "-",
        str(body_fat.tigh) if body_fat.tigh else "-",
        str(body_fat.chest) if body_fat.chest else "-",
        str(body_fat.abdominal) if body_fat.abdominal else "-",
        str(body_fat.subscapular) if body_fat.subscapular else "-",
        str(body_fat.midaxillary) if body_fat.midaxillary else "-",
    )
    new_values = ("1", "2", "3", "4", "5", "6", "7")

    login(driver)
    page = BodyFatPage(driver)
    page.load()

    page.wait_for_table_value(1, date)
    for i, v in enumerate(values, start=4):
        page.wait_for_table_value(i, v)

    page.click_edit(0)
    page.body_fat_dialog.set_jp7(new_values)
    page.body_fat_dialog.click_cancel()

    page.wait_for_table_value(1, date)
    for i, v in enumerate(values, start=4):
        page.wait_for_table_value(i, v)

    page.click_edit(0)
    page.body_fat_dialog.set_jp7(new_values)
    page.body_fat_dialog.click_save()

    page.wait_for_table_value(1, date)
    for i, v in enumerate(new_values, start=4):
        page.wait_for_table_value(i, v)


def test_body_fat_delete(driver: webdriver.Chrome) -> None:
    date_1 = str(USER.body_fat[-1].date)
    date_2 = str(USER.body_fat[-2].date)

    login(driver)
    page = BodyFatPage(driver)
    page.load()

    page.wait_for_table_value(1, date_1)

    page.click_delete(0)
    page.delete_dialog.click_no()

    page.wait_for_table_value(1, date_1)

    page.click_delete(0)
    page.delete_dialog.click_yes()

    page.wait_for_table_value(1, date_2)


def test_menstrual_cycle_add(driver: webdriver.Chrome) -> None:
    login(driver)
    page = MenstrualCyclePage(driver)
    page.load()
    page.click_fab()

    page.wait_for_dialog()

    date = page.period_dialog.get_date()
    intensity = "4"

    page.period_dialog.click_cancel()

    assert page.get_table_value(1) != date
    assert page.get_table_value(2) != intensity

    page.click_fab()

    page.wait_for_dialog()

    page.period_dialog.set_period(intensity)

    assert page.get_table_value(1) != date
    assert page.get_table_value(2) != intensity

    page.period_dialog.click_save()

    page.wait_for_table_value(1, date)
    page.wait_for_table_value(2, intensity)


def test_menstrual_cycle_edit(driver: webdriver.Chrome) -> None:
    period = USER.period[-1]
    date = str(period.date)
    intensity = str(period.intensity)
    new_intensity = "4"

    assert intensity != new_intensity

    login(driver)
    page = MenstrualCyclePage(driver)
    page.load()

    page.wait_for_table_value(1, date)
    page.wait_for_table_value(2, intensity)

    page.click_edit(0)
    page.period_dialog.set_period(new_intensity)
    page.period_dialog.click_cancel()

    page.wait_for_table_value(1, date)
    page.wait_for_table_value(2, intensity)

    page.click_edit(0)
    page.period_dialog.set_period(new_intensity)
    page.period_dialog.click_save()

    page.wait_for_table_value(1, date)
    page.wait_for_table_value(2, new_intensity)


def test_menstrual_cycle_delete(driver: webdriver.Chrome) -> None:
    period = USER.period[-1]
    date_1 = str(period.date)
    intensity = str(period.intensity)
    date_2 = str(USER.period[-2].date)

    login(driver)
    page = MenstrualCyclePage(driver)
    page.load()

    page.wait_for_table_value(1, date_1)
    page.wait_for_table_value(2, intensity)

    page.click_delete(0)
    page.delete_dialog.click_no()

    page.wait_for_table_value(1, date_1)
    page.wait_for_table_value(2, intensity)

    page.click_delete(0)
    page.delete_dialog.click_yes()

    page.wait_for_table_value(1, date_2)


def test_training_links(driver: webdriver.Chrome) -> None:
    login(driver)
    page = TrainingPage(driver)
    page.load()

    page.click_routines()
    routines_page = RoutinesPage(driver)
    routines_page.wait_until_loaded()
    routines_page.click_up_button()
    page.wait_until_loaded()

    page.click_exercises()
    exercises_page = ExercisesPage(driver)
    exercises_page.wait_until_loaded()
    exercises_page.click_up_button()
    page.wait_until_loaded()


def test_training_add(driver: webdriver.Chrome) -> None:
    routine = USER.routines[-1].name

    login(driver)
    page = TrainingPage(driver)
    page.load()
    page.click_fab()

    page.wait_for_dialog()

    date = page.training_dialog.get_date()

    page.training_dialog.click_cancel()

    assert page.get_table_value(1) != date
    assert page.get_table_value(2) != routine

    page.click_fab()

    page.wait_for_dialog()

    page.training_dialog.set_routine(routine)

    assert page.get_table_value(1) != date
    assert page.get_table_value(2) != routine

    page.training_dialog.click_save()

    training_session_page = TrainingSessionPage(driver, 0)

    training_session_page.wait_until_loaded()

    page.wait_for_title(str(date))


def test_training_delete(driver: webdriver.Chrome) -> None:
    workout = USER.workouts[-1]
    date_1 = str(workout.date)
    routine = (
        [r.name for r in USER.routines if r.id == workout.routine_id][0]
        if workout.routine_id
        else "-"
    )
    date_2 = str(USER.workouts[-2].date)

    login(driver)
    page = TrainingPage(driver)
    page.load()

    page.wait_for_table_value(1, date_1)
    page.wait_for_table_value(2, routine)

    page.click_delete(0)
    page.delete_dialog.click_no()

    page.wait_for_table_value(1, date_1)
    page.wait_for_table_value(2, routine)

    page.click_delete(0)
    page.delete_dialog.click_yes()

    page.wait_for_table_value(1, date_2)


def test_training_session_change_entries(driver: webdriver.Chrome) -> None:
    workout = USER.workouts[-1]
    sets = [
        [
            str(s.reps) if s.reps is not None else "",
            str(s.time) if s.time is not None else "",
            str(s.weight) if s.weight is not None else "",
            (str(int(s.rpe) if s.rpe % 1 == 0 else s.rpe)) if s.rpe is not None else "",
        ]
        for s in workout.elements
        if isinstance(s, models.WorkoutSet)
    ]
    new_values = ["1", "2", "3", "4"]

    login(driver)
    page = TrainingSessionPage(driver, workout.id)
    page.load()

    page.wait_for_title(str(workout.date))
    assert page.get_sets() == sets

    page.set_set(0, new_values)

    assert page.get_sets() == [new_values, *sets[1:]]

    page.load(accept_unsaved_changes=True)

    page.wait_for_title(str(workout.date))
    assert page.get_sets() == sets

    page.set_set(0, new_values)

    assert page.get_sets() == [new_values, *sets[1:]]

    page.click_save()
    page.load()

    page.wait_for_title(str(workout.date))
    assert page.get_sets() == [new_values, *sets[1:]]


def test_training_session_change_notes(driver: webdriver.Chrome) -> None:
    workout = USER.workouts[0]
    sets = [
        [
            str(s.reps) if s.reps is not None else "",
            str(s.time) if s.time is not None else "",
            str(s.weight) if s.weight is not None else "",
            (str(int(s.rpe) if s.rpe % 1 == 0 else s.rpe)) if s.rpe is not None else "",
        ]
        for s in workout.elements
        if isinstance(s, models.WorkoutSet)
    ]
    notes = workout.notes if workout.notes is not None else ""
    new_notes = "Test"

    assert notes != new_notes

    login(driver)
    page = TrainingSessionPage(driver, workout.id)
    page.load()

    page.wait_for_title(str(workout.date))
    assert page.get_sets() == sets
    assert page.get_notes() == notes

    page.set_notes(new_notes)

    assert page.get_sets() == sets
    assert page.get_notes() == new_notes

    page.load(accept_unsaved_changes=True)

    page.wait_for_title(str(workout.date))
    assert page.get_sets() == sets
    assert page.get_notes() == notes

    page.set_notes(new_notes)

    assert page.get_sets() == sets
    assert page.get_notes() == new_notes

    page.click_save()
    page.load()

    page.wait_for_title(str(workout.date))
    assert page.get_sets() == sets
    assert page.get_notes() == new_notes


def test_routines_add(driver: webdriver.Chrome) -> None:
    name = USER.routines[-1].name
    new_name = "New Routine"

    assert name != new_name

    login(driver)
    page = RoutinesPage(driver)
    page.load()
    page.click_fab()

    page.wait_for_dialog()

    page.routines_dialog.click_cancel()

    page.wait_for_table_value(1, name)

    page.click_fab()

    page.wait_for_dialog()

    page.routines_dialog.set_name(new_name)
    page.routines_dialog.click_save()

    page.wait_for_table_value(1, new_name)


def test_routines_edit(driver: webdriver.Chrome) -> None:
    name = str(USER.routines[-1].name)
    new_name = "Changed Routine"

    assert name != new_name

    login(driver)
    page = RoutinesPage(driver)
    page.load()

    page.wait_for_table_value(1, name)

    page.click_edit(0)
    page.routines_dialog.set_name(new_name)
    page.routines_dialog.click_cancel()

    page.wait_for_table_value(1, name)

    page.click_edit(0)
    page.routines_dialog.set_name(new_name)
    page.routines_dialog.click_save()

    page.wait_for_table_value(1, new_name)


def test_routines_delete(driver: webdriver.Chrome) -> None:
    name_1 = str(USER.routines[-1].name)
    name_2 = str(USER.routines[-2].name)

    login(driver)
    page = RoutinesPage(driver)
    page.load()

    page.wait_for_table_value(1, name_1)

    page.click_delete(0)
    page.delete_dialog.click_no()

    page.wait_for_table_value(1, name_1)

    page.click_delete(0)
    page.delete_dialog.click_yes()

    page.wait_for_table_value(1, name_2)


def test_routine_edit_save(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    page.wait_for_link(exercise_1)
    page.wait_for_link(exercise_2)

    sections_before_editing = page.get_sections()

    page.click_fab()

    page.wait_for_editable_sections()

    page.click_fab()

    page.wait_for_sections()

    sections = page.get_sections()
    assert sections[0][0] == exercise_1
    assert sections[1][1] == exercise_2
    assert sections == sections_before_editing


def test_routine_edit(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    page.wait_for_link(exercise_1)
    page.wait_for_link(exercise_2)

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[0][0] == exercise_1

    page.click_fab()

    page.wait_for_editable_sections()
    page.set_rounds(0, 8)
    page.set_exercise(0, exercise_2)
    page.set_reps(0, "10")
    page.set_time(0, "4")
    page.set_weight(0, "18")
    page.set_rpe(0, "8")
    page.click_auto_button(0)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[0] == ("8", exercise_2, "10", "4 s", "18 kg", "@", "8", "A", "Rest", "30 s")

    page.click_fab()

    page.wait_for_editable_sections()
    page.set_rounds(0, 8)
    page.set_exercise(0, exercise_1)
    page.set_reps(0, "")
    page.set_time(0, "60")
    page.set_weight(0, "5.5")
    page.set_rpe(0, "8.5")
    page.click_auto_button(0)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[0] == ("8", exercise_1, "60 s", "5.5 kg", "@", "8.5", "Rest", "30 s")


def test_routine_create_exercise(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)
    new_exercise = "New Exercise"

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    page.wait_for_link(exercise_1)
    page.wait_for_link(exercise_2)

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[0][0] == exercise_1

    page.click_fab()

    page.wait_for_editable_sections()
    page.create_and_set_exercise(0, new_exercise)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[0][0] == new_exercise


def test_routine_add_section(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    section_rounds = [str(s.rounds) for s in routine.sections if s.rounds > 1]

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    assert [s[0] for s in page.get_sections() if len(str(s[0])) == 1] == section_rounds

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_add_section_button(4)
    page.click_add_section_button(3)
    page.click_add_section_button(2)
    page.click_add_section_button(1)
    page.click_add_section_button(0)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert len(sections) == 4


def test_routine_add_exercise(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)
    new_exercise = USER.exercises[0].name

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    page.wait_for_link(exercise_1)
    page.wait_for_link(exercise_2)

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_add_activity_button(0)
    page.click_add_activity_button(1)
    page.click_add_activity_button(2)
    page.click_add_activity_button(3)

    page.click_fab()

    page.wait_for_sections()

    sections = page.get_sections()
    assert sections[0][0] == exercise_1
    assert sections[1][1] == exercise_2
    assert sections[0][3] == new_exercise
    assert sections[1][8] == new_exercise
    assert sections[1][9] == new_exercise
    assert sections[2][7] == new_exercise


def test_routine_add_rest(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    page.wait_for_link(exercise_1)
    page.wait_for_link(exercise_2)

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_add_rest_button(0)
    page.click_add_rest_button(1)
    page.click_add_rest_button(2)
    page.click_add_rest_button(3)

    page.click_fab()

    page.wait_for_sections()

    sections = page.get_sections()
    assert sections[0][0] == exercise_1
    assert sections[1][1] == exercise_2
    assert sections[0][3] == "Rest"
    assert sections[1][8] == "Rest"
    assert sections[1][11] == "Rest"
    assert sections[2][7] == "Rest"


def test_routine_move_section_up(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    page.wait_for_link(exercise_1)
    page.wait_for_link(exercise_2)

    sections = page.get_sections()
    assert sections[0][0] == exercise_1
    assert sections[1][1] == exercise_2

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_up_button(0)
    page.click_move_part_up_button(0)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[1][0] == exercise_1
    assert sections[2][1] == exercise_2

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_up_button(0)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[0][0] == exercise_1
    assert sections[1][1] == exercise_2


def test_routine_move_section_down(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    page.wait_for_link(exercise_1)
    page.wait_for_link(exercise_2)

    sections = page.get_sections()
    assert sections[0][0] == exercise_1
    assert sections[1][1] == exercise_2

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_down_button(3)
    page.click_move_part_down_button(0)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[1][0] == exercise_1
    assert sections[2][1] == exercise_2

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_down_button(6)
    page.click_move_part_down_button(9)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[0][0] == exercise_1
    assert sections[1][1] == exercise_2


def test_routine_move_nested_section_up(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[1].parts[2], models.RoutineSection)
    assert isinstance(routine.sections[1].parts[2].parts[0], models.RoutineActivity)
    exercise = str(routine.sections[1].parts[2].parts[0].exercise.name)

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    page.wait_for_link(exercise)

    sections = page.get_sections()

    assert sections[1][5] == exercise

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_up_button(6)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[1][3] == exercise

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_up_button(5)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[1][2] == exercise

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_up_button(4)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[1][5] == exercise


def test_routine_move_nested_section_down(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[1].parts[2], models.RoutineSection)
    assert isinstance(routine.sections[1].parts[2].parts[0], models.RoutineActivity)
    exercise = str(routine.sections[1].parts[2].parts[0].exercise.name)

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    page.wait_for_link(exercise)

    sections = page.get_sections()

    assert sections[1][5] == exercise

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_down_button(6)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[1][2] == exercise

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_down_button(4)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[1][3] == exercise

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_down_button(5)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[1][5] == exercise


def test_routine_move_exercise_up(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    page.wait_for_link(exercise_1)
    page.wait_for_link(exercise_2)

    sections = page.get_sections()
    assert sections[0][0] == exercise_1
    assert sections[1][1] == exercise_2

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_up_button(1)
    page.click_move_part_up_button(4)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[0][2] == exercise_1
    assert sections[1][7] == exercise_2

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_up_button(2)
    page.click_move_part_up_button(8)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[0][0] == exercise_1
    assert sections[1][3] == exercise_2


def test_routine_move_exercise_down(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise_1 = str(routine.sections[0].parts[0].exercise.name)
    assert isinstance(routine.sections[1].parts[0], models.RoutineActivity)
    exercise_2 = str(routine.sections[1].parts[0].exercise.name)

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    page.wait_for_link(exercise_1)
    page.wait_for_link(exercise_2)

    sections = page.get_sections()
    assert sections[0][0] == exercise_1
    assert sections[1][1] == exercise_2

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_down_button(1)
    page.click_move_part_down_button(4)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[0][2] == exercise_1
    assert sections[1][3] == exercise_2

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_move_part_down_button(2)
    page.click_move_part_down_button(5)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[0][0] == exercise_1
    assert sections[1][7] == exercise_2


def test_routine_remove_section(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    section_rounds = [str(s.rounds) for s in routine.sections]

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    assert len(page.get_sections()) == len(section_rounds)

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_remove_part_button(0)

    page.click_fab()

    page.wait_for_sections()
    assert [s[0] for s in page.get_sections()] == section_rounds[1:]

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_remove_part_button(6)

    page.click_fab()

    page.wait_for_sections()
    assert [s[0] for s in page.get_sections()] == section_rounds[1:2]


def test_routine_remove_activity(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    assert isinstance(routine.sections[0].parts[0], models.RoutineActivity)
    exercise = str(routine.sections[0].parts[0].exercise.name)

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    page.wait_for_link(exercise)

    sections = page.get_sections()
    assert sections[0][0] == exercise
    assert sections[0][1] == "Rest"
    assert len(sections[0]) == 3

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_remove_part_button(2)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert sections[0][0] == exercise
    assert len(sections[0]) == 1

    page.click_fab()

    page.wait_for_editable_sections()
    page.click_remove_part_button(1)

    page.click_fab()

    page.wait_for_sections()
    sections = page.get_sections()
    assert len(sections[0]) == 0


def test_routine_delete_training_session(driver: webdriver.Chrome) -> None:
    routine = USER.routines[0]
    workouts = sorted(
        {w for w in USER.workouts if w.routine_id == routine.id}, key=lambda x: x.date
    )
    workout_1 = str(workouts[-1].date)
    workout_2 = str(workouts[-2].date)

    login(driver)
    page = RoutinePage(driver, routine.id)
    page.load()

    page.wait_for_title(routine.name)

    page.wait_for_link(workout_1)
    page.wait_for_link(workout_2)

    page.click_delete(0)
    page.delete_dialog.click_no()

    page.wait_for_link(workout_1)
    page.wait_for_link(workout_2)

    page.click_delete(0)
    page.delete_dialog.click_yes()

    page.wait_for_link_not_present(workout_1)
    page.wait_for_link(workout_2)


def test_exercises_add(driver: webdriver.Chrome) -> None:
    exercise = sorted(USER.exercises, key=lambda x: x.name)[0]
    name = exercise.name
    new_name = "A Exercise"

    assert new_name < name

    login(driver)
    page = ExercisesPage(driver)
    page.load()
    page.click_fab()

    page.wait_for_dialog()

    page.exercises_dialog.click_cancel()

    page.wait_for_table_value(1, name)

    page.click_fab()

    page.wait_for_dialog()

    page.exercises_dialog.set_name(new_name)
    page.exercises_dialog.click_save()

    page.wait_for_table_value(1, new_name)


def test_exercises_edit(driver: webdriver.Chrome) -> None:
    exercise = sorted(USER.exercises, key=lambda x: x.name)[0]
    name = str(exercise.name)
    new_name = "Changed Exercise"

    assert name != new_name

    login(driver)
    page = ExercisesPage(driver)
    page.load()

    page.wait_for_table_value(1, name)

    page.click_edit(0)
    page.exercises_dialog.set_name(new_name)
    page.exercises_dialog.click_cancel()

    page.wait_for_table_value(1, name)

    page.click_edit(0)
    page.exercises_dialog.set_name(new_name)
    page.exercises_dialog.click_save()

    page.wait_for_table_value(1, new_name)


def test_exercises_delete(driver: webdriver.Chrome) -> None:
    exercises = sorted(USER.exercises, key=lambda x: x.name)
    name_1 = str(exercises[0].name)
    name_2 = str(exercises[1].name)

    login(driver)
    page = ExercisesPage(driver)
    page.load()

    page.wait_for_table_value(1, name_1)

    page.click_delete(0)
    page.delete_dialog.click_no()

    page.wait_for_table_value(1, name_1)

    page.click_delete(0)
    page.delete_dialog.click_yes()

    page.wait_for_table_value(1, name_2)


def test_exercise_delete_workout(driver: webdriver.Chrome) -> None:
    exercise = sorted(USER.exercises, key=lambda x: x.name)[1]
    workouts = sorted({ws.workout for ws in exercise.sets}, key=lambda x: x.date)
    workout_1 = str(workouts[-1].date)
    workout_2 = str(workouts[-2].date)

    login(driver)
    page = ExercisePage(driver, exercise.id)
    page.load()

    page.wait_for_table_value(1, workout_1)

    page.click_delete(0)
    page.delete_dialog.click_no()

    page.wait_for_table_value(1, workout_1)

    page.click_delete(0)
    page.delete_dialog.click_yes()

    page.wait_for_table_value(1, workout_2)
