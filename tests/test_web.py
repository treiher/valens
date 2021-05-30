import datetime
import pathlib
import re
import tempfile
from typing import Any

import pandas as pd
import pytest
from werkzeug.datastructures import MultiDict
from werkzeug.middleware.dispatcher import DispatcherMiddleware
from werkzeug.test import Client
from werkzeug.wrappers import BaseResponse

import tests.data
import tests.utils
from valens import config, web


@pytest.fixture(name="client", scope="module")
def fixture_client() -> Client:
    web.app.config["TESTING"] = True
    app = DispatcherMiddleware(web.app, {"/test": web.app})
    return Client(app, BaseResponse)


def assert_resources_available(client: Client, data: bytes) -> None:
    for r in re.findall(r' (?:href|src)="([^"]*)"', data.decode("utf-8")):
        if "logout" in r:
            continue
        assert client.get(r).status_code == 200, f"{r} not found"


@pytest.mark.parametrize("path", ["", "/test"])
def test_login(client: Client, path: str, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.get(f"{path}/login")
        assert resp.status_code == 200

        resp = client.post(f"{path}/login", data={"username": "U1"})
        assert resp.status_code == 302

        resp = client.get(f"{path}/")
        assert resp.status_code == 200

        resp = client.get(f"{path}/logout")
        assert resp.status_code == 302


@pytest.mark.parametrize(
    "route",
    [
        "/login",
        "/users",
    ],
)
@pytest.mark.parametrize("path", ["", "/test"])
def test_availability_wihout_login(client: Client, path: str, route: str, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        url = path + route
        resp = client.get(url)
        assert resp.status_code == 200

        resp = client.post(f"{path}/login", data={"username": "U1"})
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
        "/exercise/foo",
        "/exercises",
        "/image/bodyweight",
        "/image/bodyfat",
        "/image/period",
        "/image/exercise",
        "/image/workouts",
        "/routine/foo",
        "/routines",
        "/workout/2002-02-20",
        "/workouts",
        "/workouts?first=2002-01-01&last=2002-12-31",
    ],
)
@pytest.mark.parametrize("path", ["", "/test"])
def test_availability_with_login(client: Client, path: str, route: str, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        url = path + route
        resp = client.get(url)
        assert resp.status_code == 302

        resp = client.post(f"{path}/login", data={"username": "U1"})
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
def test_non_availability(client: Client, url: str, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        resp = client.get(url)
        assert resp.status_code == 404, f"{url} found"


def test_users(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        resp = client.get("/users")
        assert resp.status_code == 200
        for user in tests.data.USERS.values():
            assert user["name"] in resp.data.decode("utf-8")


def test_users_empty(client: Client, monkeypatch: Any, tmp_path: pathlib.Path) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)

    web.storage.initialize()

    resp = client.get("/users")
    assert resp.status_code == 200


def test_users_add(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_users", lambda x: args.update({"df": x}))
        resp = client.post(
            "/users",
            data=MultiDict(
                [
                    *[("user_id", user_id) for user_id in tests.data.USERS],
                    *[("username", user["name"]) for user in tests.data.USERS.values()],
                    *[("sex", user["sex"]) for user in tests.data.USERS.values()],
                    *[("user_id", 3), ("username", "U3"), ("sex", 0)],
                ]
            ),
        )
        assert resp.status_code == 200
        assert len(args["df"]) == 3
        for user in tests.data.USERS.values():
            assert user["name"] in resp.data.decode("utf-8")
        assert "U3" in resp.data.decode("utf-8")


def test_users_remove(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_users", lambda x: args.update({"df": x}))
        resp = client.post(
            "/users",
            data=MultiDict(
                [
                    *[("user_id", user_id) for user_id in tests.data.USERS][1:],
                    *[("username", user["name"]) for user in tests.data.USERS.values()][1:],
                    *[("sex", user["sex"]) for user in tests.data.USERS.values()][1:],
                ]
            ),
        )
        assert resp.status_code == 200
        assert len(args["df"]) == 1


def test_bodyweight(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        resp = client.get("/bodyweight?first=2002-02-01&last=2002-03-01")
        assert resp.status_code == 200
        for d in tests.data.BODYWEIGHT:
            assert str(d) in resp.data.decode("utf-8")


def test_bodyweight_empty(client: Client, monkeypatch: Any, tmp_path: pathlib.Path) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)

    web.storage.initialize()

    resp = client.get("/bodyweight?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200


def test_bodyweight_add(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_bodyweight", lambda x, y: args.update({"df": x}))
        resp = client.post("/bodyweight", data={"date": "2002-02-24", "weight": "42"})
        assert resp.status_code == 200
        assert len(args["df"]) == 3


def test_bodyweight_remove(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_bodyweight", lambda x, y: args.update({"df": x}))
        resp = client.post("/bodyweight", data={"date": "2002-02-20", "weight": "0"})
        assert resp.status_code == 200
        assert len(args["df"]) == 1


def test_bodyfat_female(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        resp = client.get("/bodyfat?first=2002-02-01&last=2002-03-01")
        assert resp.status_code == 200
        for date, values in tests.data.BODYFAT.items():
            assert str(date) in resp.data.decode("utf-8")
            for v in values:
                assert str(v) in resp.data.decode("utf-8")


def test_bodyfat_male(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U2"})
        assert resp.status_code == 302

        resp = client.get("/bodyfat?first=2002-02-01&last=2002-03-01")
        assert resp.status_code == 200
        for date, values in tests.data.BODYFAT.items():
            assert str(date) in resp.data.decode("utf-8")
            for v in values:
                assert str(v) in resp.data.decode("utf-8")


def test_bodyfat_empty(client: Client, monkeypatch: Any, tmp_path: pathlib.Path) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)

    web.storage.initialize()

    resp = client.get("/bodyfat?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200


def test_bodyfat_add(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        data = {
            "date": "2002-02-24",
            "chest": 25,
            "abdominal": 26,
            "tigh": 27,
            "tricep": 28,
            "subscapular": 29,
            "suprailiac": 30,
            "midaxillary": 31,
        }
        args = {}
        monkeypatch.setattr(web.storage, "write_bodyfat", lambda x, y: args.update({"df": x}))
        resp = client.post(
            "/bodyfat",
            data=data,
        )
        assert resp.status_code == 200
        assert len(args["df"]) == 3


def test_bodyfat_remove(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_bodyfat", lambda x, y: args.update({"df": x}))
        resp = client.post(
            "/bodyfat",
            data={
                "date": "2002-02-20",
                "chest": "",
                "abdominal": "",
                "tigh": "",
                "tricep": "",
                "subscapular": "",
                "suprailiac": "",
                "midaxillary": "",
            },
        )
        assert resp.status_code == 200
        assert len(args["df"]) == 1


def test_period(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        resp = client.get("/period?first=2002-02-01&last=2002-03-01")
        assert resp.status_code == 200
        for d in tests.data.PERIOD:
            assert str(d) in resp.data.decode("utf-8")


def test_period_empty(client: Client, monkeypatch: Any, tmp_path: pathlib.Path) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)

    web.storage.initialize()

    resp = client.get("/period?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200


def test_period_add(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_period", lambda x, y: args.update({"df": x}))
        resp = client.post("/period", data={"date": "2002-02-24", "intensity": "1"})
        assert resp.status_code == 200
        assert len(args["df"]) == 3


def test_period_add_invalid(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_period", lambda x, y: args.update({"df": x}))
        resp = client.post("/period", data={"date": "2002-02-24", "intensity": "42"})
        assert resp.status_code == 200
        assert "Invalid intensity value 42" in resp.data.decode("utf-8")


def test_period_remove(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_period", lambda x, y: args.update({"df": x}))
        resp = client.post("/period", data={"date": "2002-02-20", "intensity": "0"})
        assert resp.status_code == 200
        assert len(args["df"]) == 1


def test_exercise(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        for date, exercises in tests.data.SETS.items():
            for exercise in exercises:
                resp = client.get(f"/exercise/{exercise}?first=2002-02-01&last=2002-03-01")
                assert resp.status_code == 200
                assert str(date) in resp.data.decode("utf-8")


def test_exercise_rename(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args_sets = {}
        args_routines = {}
        monkeypatch.setattr(
            web.storage, "write_sets", lambda x, y: args_sets.update({"df": x, "user_id": y})
        )
        monkeypatch.setattr(
            web.storage,
            "write_routine_sets",
            lambda x, y: args_routines.update({"df": x, "user_id": y}),
        )
        resp = client.post("/exercise/E1", data={"new_name": "NEW"})
        assert resp.status_code == 302
        assert args_sets["user_id"] == 1
        assert len(args_sets["df"]) == len(tests.data.SETS_DF)
        assert "NEW" in str(args_sets["df"])
        assert "E1" not in str(args_sets["df"])
        assert args_routines["user_id"] == 1
        assert len(args_routines["df"]) == len(tests.data.ROUTINE_SETS_DF)
        assert "NEW" in str(args_routines["df"])
        assert "E1" not in str(args_routines["df"])


def test_routines(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        resp = client.get("/routines")
        assert resp.status_code == 200
        for routine_name in tests.data.ROUTINE_SETS:
            assert routine_name in resp.data.decode("utf-8")


def test_routines_empty(client: Client, monkeypatch: Any, tmp_path: pathlib.Path) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)

    web.storage.initialize()

    resp = client.get("/routines")
    assert resp.status_code == 200


def test_routines_add(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        resp = client.post("/routines", data={"name": "Test"})
        assert resp.status_code == 302
        assert "Test" in resp.data.decode("utf-8")


def test_routine(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        for routine_name, exercises in tests.data.ROUTINE_SETS.items():
            resp = client.get(f"/routine/{routine_name}")
            assert resp.status_code == 200
            assert routine_name in resp.data.decode("utf-8")
            for exercise in exercises:
                assert exercise in resp.data.decode("utf-8")


def test_routine_delete(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_routine_sets", lambda x, y: args.update({"df": x}))
        resp = client.post("/routine/T1", data={"delete": ""})
        assert resp.status_code == 302
        assert args["df"].equals(
            tests.data.ROUTINE_SETS_DF[tests.data.ROUTINE_SETS_DF["routine"] != "T1"]
        )


def test_routine_save(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(
            web.storage, "write_routine_sets", lambda x, y: args.update({"df_rs": x})
        )
        monkeypatch.setattr(web.storage, "write_routines", lambda x, y: args.update({"df_r": x}))
        resp = client.post("/routine/T2")
        assert resp.status_code == 200
        assert args["df_rs"].equals(
            tests.data.ROUTINE_SETS_DF[tests.data.ROUTINE_SETS_DF["routine"] != "T2"]
        )
        assert args["df_r"].equals(
            tests.data.ROUTINES_DF[tests.data.ROUTINES_DF["routine"] != "T2"]
        )

        resp = client.post(
            "/routine/T2",
            data={
                "exercise": list(tests.data.ROUTINE_SETS["T2"].keys()),
                "set_count": [len(v) for v in tests.data.ROUTINE_SETS["T2"].values()],
                "notes": tests.data.ROUTINES["T2"]["notes"],
            },
        )
        assert resp.status_code == 200
        assert args["df_rs"].equals(tests.data.ROUTINE_SETS_DF)
        assert args["df_r"].equals(tests.data.ROUTINES_DF)


def test_routine_save_empty_notes(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(
            web.storage, "write_routine_sets", lambda x, y: args.update({"df_rs": x})
        )
        monkeypatch.setattr(web.storage, "write_routines", lambda x, y: args.update({"df_r": x}))
        resp = client.post("/routine/T2")
        assert resp.status_code == 200
        assert args["df_rs"].equals(
            tests.data.ROUTINE_SETS_DF[tests.data.ROUTINE_SETS_DF["routine"] != "T2"]
        )
        assert args["df_r"].equals(
            tests.data.ROUTINES_DF[tests.data.ROUTINES_DF["routine"] != "T2"]
        )

        resp = client.post(
            "/routine/T2",
            data={
                "exercise": list(tests.data.ROUTINE_SETS["T2"].keys()),
                "set_count": [len(v) for v in tests.data.ROUTINE_SETS["T2"].values()],
                "notes": "",
            },
        )
        assert resp.status_code == 200
        assert args["df_rs"].equals(tests.data.ROUTINE_SETS_DF)
        assert args["df_r"].equals(
            tests.data.ROUTINES_DF[tests.data.ROUTINES_DF["routine"] != "T2"]
        )


def test_routine_save_remove_exercise(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(
            web.storage, "write_routine_sets", lambda x, y: args.update({"df_rs": x})
        )
        monkeypatch.setattr(web.storage, "write_routines", lambda x, y: args.update({"df_r": x}))
        resp = client.post("/routine/T2")
        assert resp.status_code == 200
        assert args["df_rs"].equals(
            tests.data.ROUTINE_SETS_DF[tests.data.ROUTINE_SETS_DF["routine"] != "T2"]
        )
        assert args["df_r"].equals(
            tests.data.ROUTINES_DF[tests.data.ROUTINES_DF["routine"] != "T2"]
        )

        resp = client.post(
            "/routine/T2",
            data={
                "exercise": [""] * len(tests.data.ROUTINE_SETS["T2"]),
                "set_count": [len(v) for v in tests.data.ROUTINE_SETS["T2"].values()],
            },
        )
        assert resp.status_code == 200
        assert args["df_rs"].equals(
            tests.data.ROUTINE_SETS_DF[tests.data.ROUTINE_SETS_DF["routine"] != "T2"]
        )
        assert args["df_r"].equals(
            tests.data.ROUTINES_DF[tests.data.ROUTINES_DF["routine"] != "T2"]
        )


def test_workouts(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        resp = client.get("/workouts?first=2002-02-01&last=2002-03-01")
        assert resp.status_code == 200
        for d in tests.data.SETS:
            assert str(d) in resp.data.decode("utf-8")


def test_workouts_empty(client: Client, monkeypatch: Any, tmp_path: pathlib.Path) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)

    web.storage.initialize()

    resp = client.get("/workouts?first=2002-02-01&last=2002-03-01")
    assert resp.status_code == 200


def test_workouts_add(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_sets", lambda x, y: args.update({"df_s": x}))
        monkeypatch.setattr(web.storage, "write_workouts", lambda x, y: args.update({"df_w": x}))
        resp = client.post("/workouts", data={"date": "2002-02-24", "routine": "T2"})
        assert resp.status_code == 302

        routines_df = tests.data.ROUTINE_SETS_DF.loc[
            tests.data.ROUTINE_SETS_DF["routine"] == "T2",
            tests.data.ROUTINE_SETS_DF.columns != "routine",
        ].copy()
        routines_df["date"] = [datetime.date(2002, 2, 24)] * len(routines_df)
        assert args["df_s"].equals(pd.concat([tests.data.SETS_DF, routines_df]))
        assert "df_w" in args


def test_workouts_add_empty_routines(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_sets", lambda x, y: args.update({"df_s": x}))
        monkeypatch.setattr(web.storage, "write_workouts", lambda x, y: args.update({"df_w": x}))
        resp = client.post("/workouts", data={"date": "2002-02-24", "routine": "T1"})
        assert resp.status_code == 302

        routines_df = tests.data.ROUTINE_SETS_DF.loc[
            tests.data.ROUTINE_SETS_DF["routine"] == "T1",
            tests.data.ROUTINE_SETS_DF.columns != "routine",
        ].copy()
        routines_df["date"] = [datetime.date(2002, 2, 24)] * len(routines_df)
        assert args["df_s"].equals(pd.concat([tests.data.SETS_DF, routines_df]))
        assert "df_w" not in args


def test_workouts_add_existing(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_sets", lambda x, y: args.update({"df": x}))
        resp = client.post("/workouts", data={"date": "2002-02-20", "routine": "T1"})
        assert resp.status_code == 200
        assert "df" not in args


def test_workouts_add_undefined(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_sets", lambda x, y: args.update({"df": x}))
        resp = client.post("/workouts", data={"date": "2002-02-24", "routine": "Undefined"})
        assert resp.status_code == 200
        assert "df" not in args


def test_workout_delete(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_sets", lambda x, y: args.update({"df": x}))
        resp = client.post("/workout/2002-02-22", data={"delete": ""})
        assert resp.status_code == 302
        assert args["df"].equals(
            tests.data.SETS_DF[tests.data.SETS_DF["date"] != datetime.date(2002, 2, 22)]
        )


def test_workout_save_sets(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_sets", lambda x, y: args.update({"df": x}))
        resp = client.post("/workout/2002-02-22")
        assert resp.status_code == 200
        assert args["df"].equals(
            tests.data.SETS_DF[tests.data.SETS_DF["date"] != datetime.date(2002, 2, 22)]
        )

        resp = client.post(
            "/workout/2002-02-22",
            data=MultiDict(
                [
                    (f"exercise:{k}", e)
                    for k, v in tests.data.SETS[datetime.date(2002, 2, 22)].items()
                    for e in v
                ]
            ),
        )
        assert resp.status_code == 200
        assert args["df"][["date", "exercise", "reps", "time", "weight", "rpe"]].equals(
            tests.data.SETS_DF[["date", "exercise", "reps", "time", "weight", "rpe"]]
        )


def test_workout_save_workouts(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_workouts", lambda x, y: args.update({"df": x}))
        resp = client.post("/workout/2002-02-22", data={"undefined": "undefined"})
        assert resp.status_code == 200
        assert args["df"].equals(
            tests.data.WORKOUTS_DF[tests.data.WORKOUTS_DF["date"] != datetime.date(2002, 2, 22)]
        )

        resp = client.post(
            "/workout/2002-02-22",
            data=MultiDict(tests.data.WORKOUTS[datetime.date(2002, 2, 22)]),
        )
        assert resp.status_code == 200
        assert args["df"][["date", "notes"]].equals(tests.data.WORKOUTS_DF[["date", "notes"]])


def test_workout_save_error(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        resp = client.post("/login", data={"username": "U1"})
        assert resp.status_code == 302

        args = {}
        monkeypatch.setattr(web.storage, "write_sets", lambda x, y: args.update({"df": x}))
        resp = client.post("/workout/2002-02-22", data={"exercise:E4": "error"})
        assert resp.status_code == 200
        assert "df" not in args
