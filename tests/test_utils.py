import tempfile
from typing import Any

from valens import utils


def test_parse_config(monkeypatch: Any) -> None:
    with tempfile.NamedTemporaryFile() as tmp_file:
        tmp_file.write(b"foo: bar")
        tmp_file.flush()

        monkeypatch.setattr(utils, "CONFIG_FILE", tmp_file.name)

        assert utils.parse_config() == {"foo": "bar"}
