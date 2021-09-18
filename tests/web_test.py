# pylint: disable = too-many-lines

import datetime
import re
from pathlib import Path
from typing import Generator

import pytest
from werkzeug.datastructures import MultiDict
from werkzeug.middleware.dispatcher import DispatcherMiddleware
from werkzeug.test import Client, TestResponse

import tests.data
import tests.utils
from valens import app, database as db, web
from valens.models import BodyWeight, Sex, User


@pytest.fixture(name="client")
def fixture_client(tmp_path: Path) -> Generator[Client, None, None]:
    app.config["DATABASE"] = f"sqlite:///{tmp_path}/valens.db"
    app.config["SECRET_KEY"] = b"TEST_KEY"
    app.config["TESTING"] = True
    app.wsgi_app = DispatcherMiddleware(app.wsgi_app, {"/test": app.wsgi_app})  # type: ignore

    with app.test_client() as client:
        with app.app_context():
            db.init_db()
            yield client


def assert_resources_available(client: Client, data: bytes) -> None:
    for r in re.findall(r' (?:href|src)="([^"]*)"', data.decode("utf-8")):
        if "logout" in r:
            continue
        assert client.get(r, follow_redirects=True).status_code == 200, f"{r} not found"


def login(client: Client, user_id: int = 1, path: str = "") -> TestResponse:
    return client.post(
        f"{path}/login", data=dict(username=tests.data.users()[user_id - 1].name, next="/")
    )


@pytest.mark.parametrize("path", ["", "/test"])
def test_login(client: Client, path: str) -> None:
    tests.utils.init_db_data()

    resp = client.get(f"{path}/")
    assert resp.status_code == 302

    resp = client.get(f"{path}/login")
    assert resp.status_code == 200

    resp = login(client, path=path)
    assert resp.status_code == 302

    resp = client.get(f"{path}/")
    assert resp.status_code == 200

    resp = client.get(f"{path}/logout")
    assert resp.status_code == 302


@pytest.mark.parametrize(
    "route",
    [
        "/login",
        "/offline",
        "/service-worker.js",
        "/users",
    ],
)
@pytest.mark.parametrize("path", ["", "/test"])
def test_availability_wihout_login(client: Client, path: str, route: str) -> None:
    tests.utils.init_db_data()

    url = path + route
    resp = client.get(url)
    assert resp.status_code == 200

    resp = login(client, path=path)
    assert resp.status_code == 302

    resp = client.get(url)
    assert resp.status_code == 200, f"{url} not found"
    assert_resources_available(client, resp.data)

    resp = client.get(f"{path}/logout")
    assert resp.status_code == 302


@pytest.mark.parametrize(
    "route",
    [
        "/",
        "/bodyweight",
        "/bodyweight?first=2002-01-01&last=2002-12-31",
        "/bodyfat",
        "/bodyfat?first=2002-01-01&last=2002-12-31",
        "/period",
        "/period?first=2002-01-01&last=2002-12-31",
        "/exercise/Exercise 1",
        "/exercises",
        "/image/bodyweight",
        "/image/bodyfat",
        "/image/period",
        "/image/exercise",
        "/image/workouts",
        "/routine/R1",
        "/routine/R1/edit",
        "/routine/R1/rename",
        "/routine/R1/copy",
        "/routine/R1/delete",
        "/routines",
        "/workout/1",
        "/workouts",
        "/workouts?first=2002-01-01&last=2002-12-31",
    ],
)
@pytest.mark.parametrize("path", ["", "/test"])
def test_availability_with_login(client: Client, path: str, route: str) -> None:
    tests.utils.init_db_data()

    url = path + route
    resp = client.get(url)
    assert resp.status_code == 302

    resp = login(client, path=path)
    assert resp.status_code == 302

    resp = client.get(url)
    assert resp.status_code == 200, f"{url} not found"
    assert_resources_available(client, resp.data)

    resp = client.get(f"{path}/logout")
    assert resp.status_code == 302


@pytest.mark.parametrize(
    "url",
    ["/image/foo"],
)
def test_non_availability(client: Client, url: str) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get(url)
    assert resp.status_code == 404, f"{url} found"


def test_index(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/")
    assert resp.status_code == 200


def test_index_empty(client: Client) -> None:
    tests.utils.init_db_users()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/")
    assert resp.status_code == 200


def test_users(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/users")
    assert resp.status_code == 200
    for user in tests.data.users():
        assert user.name in resp.data.decode("utf-8")


def test_users_empty(client: Client) -> None:
    tests.utils.init_db_users()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/users")
    assert resp.status_code == 200


def test_users_add(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.post(
        "/users",
        data=MultiDict(
            [
                *[
                    e
                    for user in tests.data.users()
                    for e in [
                        ("user_id", user.id),
                        ("username", user.name),
                        ("sex", user.sex.value),
                    ]
                ],
                *[("user_id", 3), ("username", "Carol"), ("sex", Sex.FEMALE.value)],
                *[("user_id", 0), ("username", ""), ("sex", Sex.MALE.value)],
            ]
        ),
    )
    assert resp.status_code == 200
    assert len(db.session.query(User).all()) == len(tests.data.users()) + 1
    for user in tests.data.users():
        assert user.name in resp.data.decode("utf-8")
    assert "Carol" in resp.data.decode("utf-8")


def test_users_remove(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.post(
        "/users",
        data=MultiDict(
            [
                e
                for user in tests.data.users()
                for e in [
                    ("user_id", user.id),
                    ("username", user.name if user.id != 2 else ""),
                    ("sex", user.sex.value),
                ]
            ]
        ),
    )
    assert resp.status_code == 200
    assert len(db.session.query(User).all()) == len(tests.data.users()) - 1
    for user in tests.data.users():
        if user.id == 2:
            assert user.name not in resp.data.decode("utf-8")
        else:
            assert user.name in resp.data.decode("utf-8")


def test_users_add_after_remove(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    user = db.session.query(User).where(User.id == 2).one()

    resp = client.post(
        "/users",
        data=MultiDict(
            [
                e
                for user in tests.data.users()
                for e in [
                    ("user_id", user.id),
                    ("username", user.name if user.id != 2 else ""),
                    ("sex", user.sex.value),
                ]
            ]
        ),
    )
    assert resp.status_code == 200
    assert len(db.session.query(User).all()) == len(tests.data.users()) - 1
    for user in tests.data.users():
        if user.id == 2:
            assert user.name not in resp.data.decode("utf-8")
        else:
            assert user.name in resp.data.decode("utf-8")

    resp = client.post(
        "/users",
        data=MultiDict(
            [
                *[
                    e
                    for user in tests.data.users()
                    if user.id != 2
                    for e in [
                        ("user_id", user.id),
                        ("username", user.name),
                        ("sex", user.sex.value),
                    ]
                ],
                *[("user_id", 2), ("username", "Dave"), ("sex", Sex.MALE.value)],
            ]
        ),
    )
    assert resp.status_code == 200
    assert len(db.session.query(User).all()) == len(tests.data.users())
    for user in tests.data.users():
        if user.id == 2:
            assert "Dave" in resp.data.decode("utf-8")
        else:
            assert user.name in resp.data.decode("utf-8")

    assert len(db.session.query(BodyWeight).where(BodyWeight.user_id == 2).all()) == 0


def test_bodyweight(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/bodyweight?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for body_weight in tests.data.user().body_weight:
        assert str(body_weight.weight) in resp.data.decode("utf-8")


def test_bodyweight_empty(client: Client) -> None:
    tests.utils.init_db_users()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/bodyweight?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200


def test_bodyweight_add(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.post("/bodyweight", data={"date": "2002-02-24", "weight": "42"})
    assert resp.status_code == 200

    resp = client.get("/bodyweight?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for body_weight in tests.data.user().body_weight:
        assert str(body_weight.weight) in resp.data.decode("utf-8")
    assert "42" in resp.data.decode("utf-8")


def test_bodyweight_remove(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.post("/bodyweight", data={"date": "2002-02-20", "weight": "0"})
    assert resp.status_code == 200

    resp = client.get("/bodyweight?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for body_weight in tests.data.user().body_weight:
        if body_weight.date == datetime.date(2002, 2, 20):
            assert str(body_weight.weight) not in resp.data.decode("utf-8")
        else:
            assert str(body_weight.weight) in resp.data.decode("utf-8")


def test_bodyfat_female(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/bodyfat?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for body_fat in tests.data.user().body_fat:
        for attr in [
            "date",
            "tricep",
            "suprailiac",
            "tigh",
            "chest",
            "abdominal",
            "subscapular",
            "midaxillary",
        ]:
            assert f"<td>{getattr(body_fat, attr)}</td>" in resp.data.decode("utf-8")


def test_bodyfat_male(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client, user_id=2)
    assert resp.status_code == 302

    resp = client.get("/bodyfat?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for body_fat in tests.data.user(user_id=2).body_fat:
        for attr in [
            "date",
            "chest",
            "abdominal",
            "tigh",
            "tricep",
            "subscapular",
            "suprailiac",
            "midaxillary",
        ]:
            assert f"<td>{getattr(body_fat, attr)}</td>" in resp.data.decode("utf-8")


def test_bodyfat_empty(client: Client) -> None:
    tests.utils.init_db_users()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/bodyfat?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200


def test_bodyfat_add(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    data = {
        "date": "2002-02-24",
        "chest": "29",
        "abdominal": "30",
        "tigh": "31",
        "tricep": "32",
        "subscapular": "33",
        "suprailiac": "34",
        "midaxillary": "35",
    }
    resp = client.post("/bodyfat", data=data)
    assert resp.status_code == 200

    resp = client.get("/bodyfat?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for body_fat in tests.data.user().body_fat:
        for attr in [
            "date",
            "tricep",
            "suprailiac",
            "tigh",
            "chest",
            "abdominal",
            "subscapular",
            "midaxillary",
        ]:
            assert f"<td>{getattr(body_fat, attr)}</td>" in resp.data.decode("utf-8")
    for value in data.values():
        assert f"<td>{value}</td>" in resp.data.decode("utf-8")


def test_bodyfat_remove(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    data = {
        "date": "2002-02-20",
        "chest": "",
        "abdominal": "",
        "tigh": "",
        "tricep": "",
        "subscapular": "",
        "suprailiac": "",
        "midaxillary": "",
    }
    resp = client.post("/bodyfat", data=data)
    assert resp.status_code == 200

    resp = client.get("/bodyfat?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for body_fat in tests.data.user().body_fat:
        for attr in [
            "date",
            "tricep",
            "suprailiac",
            "tigh",
            "chest",
            "abdominal",
            "subscapular",
            "midaxillary",
        ]:
            if body_fat.date == datetime.date(2002, 2, 20):
                assert f"<td>{getattr(body_fat, attr)}</td>" not in resp.data.decode("utf-8")
            else:
                assert f"<td>{getattr(body_fat, attr)}</td>" in resp.data.decode("utf-8")


def test_period(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/period?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for period in tests.data.user().period:
        assert f"<td>{period.intensity}</td>" in resp.data.decode("utf-8")


def test_period_empty(client: Client) -> None:
    tests.utils.init_db_users()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/period?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200


def test_period_add(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.post("/period", data={"date": "2002-02-24", "intensity": "3"})
    assert resp.status_code == 200

    resp = client.get("/period?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for period in tests.data.user().period:
        assert f"<td>{period.intensity}</td>" in resp.data.decode("utf-8")
    assert "<td>3</td>" in resp.data.decode("utf-8")


def test_period_add_invalid(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.post("/period", data={"date": "2002-02-24", "intensity": "42"})
    assert resp.status_code == 200
    assert "Invalid intensity value 42" in resp.data.decode("utf-8")


def test_period_remove(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.post("/period", data={"date": "2002-02-20", "intensity": "0"})
    assert resp.status_code == 200

    resp = client.get("/period?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for period in tests.data.user().period:
        if period.date == datetime.date(2002, 2, 20):
            assert f"<td>{period.intensity}</td>" not in resp.data.decode("utf-8")
        else:
            assert f"<td>{period.intensity}</td>" in resp.data.decode("utf-8")


def test_exercises(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.post("/exercises", data={"exercise": "Exercise 42"}, follow_redirects=True)
    assert resp.status_code == 200
    assert "Exercise 42" in resp.data.decode("utf-8")


def test_exercise(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    for exercise in tests.data.user().exercises:
        resp = client.get(f"/exercise/{exercise.name}?first=2002-02-01&last=2002-03-01")
        assert resp.status_code == 200
        for workout_set in exercise.sets:
            assert str(workout_set.workout.date) in resp.data.decode("utf-8")


def test_exercise_delete(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    exercises = tests.data.user().exercises
    exercise_name = exercises[-1].name
    assert exercise_name == "Unused Exercise"

    resp = client.get(f"/exercise/{exercise_name}/delete")
    assert resp.status_code == 200

    resp = client.post(f"/exercise/{exercise_name}/delete")
    assert resp.status_code == 302

    resp = client.get("/exercises")
    assert resp.status_code == 200
    for exercise in exercises:
        if exercise.name == exercise_name:
            assert exercise.name not in resp.data.decode("utf-8")
        else:
            assert exercise.name in resp.data.decode("utf-8")


def test_exercise_delete_error(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    exercises = tests.data.user().exercises
    exercise_name = exercises[0].name

    resp = client.get(f"/exercise/{exercise_name}/delete")
    assert resp.status_code == 302

    resp = client.post(f"/exercise/{exercise_name}/delete")
    assert resp.status_code == 302

    resp = client.get("/exercises")
    assert resp.status_code == 200
    for exercise in exercises:
        assert exercise.name in resp.data.decode("utf-8")


def test_exercise_rename(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    exercise_name = tests.data.user().exercises[0].name

    resp = client.get(f"/exercise/{exercise_name}/rename")
    assert resp.status_code == 200

    resp = client.post(f"/exercise/{exercise_name}/rename", data={"new_name": ""})
    assert resp.status_code == 200
    assert exercise_name in resp.data.decode("utf-8")

    resp = client.post(
        f"/exercise/{exercise_name}/rename",
        data={"new_name": "New Exercise"},
        follow_redirects=True,
    )
    assert resp.status_code == 200
    assert exercise_name not in resp.data.decode("utf-8")
    assert "New Exercise" in resp.data.decode("utf-8")


def test_routines(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/routines")
    assert resp.status_code == 200
    for routine in tests.data.user().routines:
        assert routine.name in resp.data.decode("utf-8")


def test_routines_empty(client: Client) -> None:
    tests.utils.init_db_users()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/routines")
    assert resp.status_code == 200


def test_routines_add(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.post("/routines", data={"name": "R42"}, follow_redirects=True)
    assert resp.status_code == 200
    assert "R42" in resp.data.decode("utf-8")

    resp = client.get("/routines")
    assert resp.status_code == 200
    for routine in tests.data.user().routines:
        assert routine.name in resp.data.decode("utf-8")
    assert "R42" in resp.data.decode("utf-8")


def test_routine(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    for routine in tests.data.user().routines:
        resp = client.get(f"/routine/{routine.name}")
        assert resp.status_code == 200
        assert routine.name in resp.data.decode("utf-8")
        for routine_exercise in routine.exercises:
            assert routine_exercise.exercise.name in resp.data.decode("utf-8")


def test_routine_delete(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    routines = tests.data.user().routines
    routine_name = routines[0].name

    resp = client.post(f"/routine/{routine_name}/delete")
    assert resp.status_code == 302

    resp = client.get("/routines")
    assert resp.status_code == 200
    for routine in routines:
        if routine.name == routine_name:
            assert routine.name not in resp.data.decode("utf-8")
        else:
            assert routine.name in resp.data.decode("utf-8")


def test_routine_rename(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    routine_name = tests.data.user().routines[0].name

    resp = client.get(f"/routine/{routine_name}/rename")
    assert resp.status_code == 200

    resp = client.post(f"/routine/{routine_name}/rename", data={"new_name": ""})
    assert resp.status_code == 200
    assert routine_name in resp.data.decode("utf-8")

    resp = client.post(
        f"/routine/{routine_name}/rename", data={"new_name": "New Routine"}, follow_redirects=True
    )
    assert resp.status_code == 200
    assert routine_name not in resp.data.decode("utf-8")
    assert "New Routine" in resp.data.decode("utf-8")

    resp = client.get("/routines")
    assert resp.status_code == 200
    assert routine_name not in resp.data.decode("utf-8")
    assert "New Routine" in resp.data.decode("utf-8")


def test_routine_copy(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    routine_name = tests.data.user().routines[0].name

    resp = client.get(f"/routine/{routine_name}/copy")
    assert resp.status_code == 200

    resp = client.post(f"/routine/{routine_name}/copy", data={"new_name": ""})
    assert resp.status_code == 200
    assert routine_name in resp.data.decode("utf-8")

    resp = client.post(
        f"/routine/{routine_name}/copy", data={"new_name": "Copy of Routine"}, follow_redirects=True
    )
    assert resp.status_code == 200
    assert routine_name not in resp.data.decode("utf-8")
    assert "Copy of Routine" in resp.data.decode("utf-8")

    resp = client.get("/routines")
    assert resp.status_code == 200
    assert routine_name in resp.data.decode("utf-8")
    assert "Copy of Routine" in resp.data.decode("utf-8")


def test_routine_edit(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    for routine in tests.data.user().routines:
        resp = client.get(f"/routine/{routine.name}/edit")
        assert resp.status_code == 200
        assert routine.name in resp.data.decode("utf-8")
        for routine_exercise in routine.exercises:
            assert routine_exercise.exercise.name in resp.data.decode("utf-8")


def test_routine_edit_save_unchanged(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    routines = tests.data.user().routines
    routine = routines[0]

    resp = client.post(
        f"/routine/{routine.name}/edit",
        data={
            "exercise": [routine_exercise.exercise.name for routine_exercise in routine.exercises],
            "set_count": [routine_exercise.sets for routine_exercise in routine.exercises],
            "notes": routine.notes,
        },
    )
    assert resp.status_code == 200
    for routine_exercise in routine.exercises:
        assert routine_exercise.exercise.name in resp.data.decode("utf-8")
    assert routine.notes in resp.data.decode("utf-8")


def test_routine_edit_add_exercise(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    routines = tests.data.user().routines
    routine = routines[0]

    resp = client.post(f"/routine/{routine.name}/edit", data={"exercise": ""})
    assert resp.status_code == 200
    for routine_exercise in routine.exercises:
        assert f'placeholder="{routine_exercise.exercise.name}"' not in resp.data.decode("utf-8")
    assert routine.notes in resp.data.decode("utf-8")

    resp = client.post(f"/routine/{routine.name}/edit", data={"notes": ""})
    assert resp.status_code == 200
    assert routine.notes not in resp.data.decode("utf-8")

    resp = client.post(
        f"/routine/{routine.name}/edit",
        data={
            "exercise": [routine_exercise.exercise.name for routine_exercise in routine.exercises],
            "set_count": [routine_exercise.sets for routine_exercise in routine.exercises],
            "notes": routine.notes,
        },
    )
    assert resp.status_code == 200
    for routine_exercise in routine.exercises:
        assert f'placeholder="{routine_exercise.exercise.name}"' in resp.data.decode("utf-8")
    assert routine.notes in resp.data.decode("utf-8")


def test_routine_edit_remove_exercise(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    routines = tests.data.user().routines
    routine = routines[0]

    resp = client.post(
        f"/routine/{routine.name}/edit",
        data={
            "exercise": [routine_exercise.exercise.name for routine_exercise in routine.exercises],
            "set_count": [
                0 if routine_exercise.position == 1 else routine_exercise.sets
                for routine_exercise in routine.exercises
            ],
        },
    )
    assert resp.status_code == 200
    for routine_exercise in routine.exercises:
        if routine_exercise.position == 1:
            assert f'placeholder="{routine_exercise.exercise.name}"' not in resp.data.decode(
                "utf-8"
            )
        else:
            assert f'placeholder="{routine_exercise.exercise.name}"' in resp.data.decode("utf-8")
    assert routine.notes in resp.data.decode("utf-8")


def test_routine_edit_rename_exercise(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    routines = tests.data.user().routines
    routine = routines[0]

    resp = client.post(
        f"/routine/{routine.name}/edit",
        data={
            "exercise": [
                "X" + routine_exercise.exercise.name for routine_exercise in routine.exercises
            ],
            "set_count": [routine_exercise.sets for routine_exercise in routine.exercises],
        },
    )
    assert resp.status_code == 200
    for routine_exercise in routine.exercises:
        assert "X" + routine_exercise.exercise.name in resp.data.decode("utf-8")
    assert routine.notes in resp.data.decode("utf-8")


def test_workouts(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/workouts?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for workout in tests.data.user().workouts:
        assert str(workout.date) in resp.data.decode("utf-8")


def test_workouts_empty(client: Client) -> None:
    tests.utils.init_db_users()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/workouts?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200


def test_workouts_negative_interval(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.get("/workouts?first=2002-03-01&last=2002-02-20")
    assert resp.status_code == 200
    assert ">2002-02-20<" in resp.data.decode("utf-8")


def test_workouts_add(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    resp = client.post(
        "/workouts", data={"date": "2002-02-24", "routine": "R1"}, follow_redirects=True
    )
    assert resp.status_code == 200
    assert "2002-02-24" in resp.data.decode("utf-8")

    resp = client.get("/workouts?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for workout in tests.data.user().workouts:
        assert str(workout.date) in resp.data.decode("utf-8")
    assert "2002-02-24" in resp.data.decode("utf-8")


def test_workouts_add_empty(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client, user_id=2)
    assert resp.status_code == 302

    resp = client.post(
        "/workouts", data={"date": "2002-02-24", "routine": "Empty"}, follow_redirects=True
    )
    assert resp.status_code == 200
    assert "2002-02-24" in resp.data.decode("utf-8")

    resp = client.get("/workouts?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for workout in tests.data.user(user_id=2).workouts:
        assert str(workout.date) in resp.data.decode("utf-8")
    assert "2002-02-24" in resp.data.decode("utf-8")


def test_workout_delete(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    workout_id = tests.data.user().workouts[0].id

    resp = client.post(f"/workout/{workout_id}/delete")
    assert resp.status_code == 302

    resp = client.get("/workouts?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200
    for workout in tests.data.user().workouts:
        if workout.id == workout_id:
            assert str(workout.date) not in resp.data.decode("utf-8")
        else:
            assert str(workout.date) in resp.data.decode("utf-8")


def test_workout_change(client: Client) -> None:
    tests.utils.init_db_data()

    resp = login(client)
    assert resp.status_code == 302

    workouts = tests.data.user().workouts
    workout = workouts[0]

    resp = client.post(
        f"/workout/{workout.id}",
        data=MultiDict(
            [
                *[(f"set{i}", "") for i, workout_set in enumerate(workout.sets) for _ in range(4)],
                ("notes", ""),
            ]
        ),
    )
    assert resp.status_code == 200
    for workout_set in workout.sets:
        for value in [workout_set.reps, workout_set.time, workout_set.weight, workout_set.rpe]:
            assert f' value="{value}" ' not in resp.data.decode("utf-8")
    assert workout.notes not in resp.data.decode("utf-8")

    resp = client.post(
        f"/workout/{workout.id}",
        data=MultiDict(
            [
                *[
                    (f"set{i}", str(value) if value else "")
                    for i, workout_set in enumerate(workout.sets)
                    for value in [
                        workout_set.reps,
                        workout_set.time,
                        workout_set.weight,
                        workout_set.rpe,
                    ]
                ],
                ("notes", workout.notes),
            ]
        ),
    )
    assert resp.status_code == 200
    for workout_set in workout.sets:
        for value in [workout_set.reps, workout_set.time, workout_set.weight, workout_set.rpe]:
            assert f' value="{str(value) if value else ""}" ' in resp.data.decode("utf-8")
    assert workout.notes in resp.data.decode("utf-8")


def test_days() -> None:
    assert "today" in web.days(datetime.timedelta(days=0))
    assert "yesterday" in web.days(datetime.timedelta(days=1))
    assert "2" in web.days(datetime.timedelta(days=2))
