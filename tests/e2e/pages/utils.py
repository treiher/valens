import re


def parse_int(text: str) -> int | None:
    try:
        return int(parse_numeric(text))
    except ValueError:
        return None


def parse_float(text: str) -> float | None:
    try:
        return float(parse_numeric(text))
    except ValueError:
        return None


def parse_numeric(text: str) -> str:
    match = re.search(r"\d+\.?\d*", text)
    return match.group() if match else ""
