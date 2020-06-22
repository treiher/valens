import pathlib
from typing import Dict

import yaml

CONFIG_FILE = pathlib.Path.home() / ".config/valens/valens.yml"


def parse_config() -> Dict[str, str]:
    with open(CONFIG_FILE) as f:
        return yaml.safe_load(f)
