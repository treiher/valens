from importlib.metadata import version


def get() -> str:
    return version("valens")
