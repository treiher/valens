"""Generation of body weight, body fat and period entries."""

from __future__ import annotations

import datetime
import random

from valens.demo.events import event_indices
from valens.demo.profiles import Profile
from valens.models import BodyFat, BodyWeight, Period, Sex

DAYS = 365

SKINFOLD_SITES = (
    "chest",
    "abdominal",
    "thigh",
    "tricep",
    "subscapular",
    "suprailiac",
    "midaxillary",
)

# Relative change of the skinfold thicknesses per kg of fat mass
SKINFOLD_SENSITIVITY = 0.06

# Days by which the skinfold thicknesses follow the fat mass
SKINFOLD_LAG = 14

# Days between two skinfold measurements
SKINFOLD_INTERVAL = 7

# One measurement out of this many is left out
SKINFOLD_SKIP_RATE = 8

# One measurement out of this many records all sites instead of the sex-appropriate subset
SKINFOLD_FULL_RATE = 4

# Share of days with a body weight entry, outside of the deliberate gaps
WEIGH_IN_RATE = 0.85


def body_weight(profile: Profile, today: datetime.date, rng: random.Random) -> list[BodyWeight]:
    first = today - datetime.timedelta(days=DAYS - 1)
    last = DAYS - 1 - rng.randint(0, 3)
    gaps = _gaps(rng, profile.weight_gaps, last)
    result = []

    for day in range(last + 1):
        if day != last and (day in gaps or rng.random() > WEIGH_IN_RATE):
            continue
        weight = _trend(profile, day) + rng.gauss(0, 0.4)
        result.append(
            BodyWeight(
                user_id=profile.id,
                date=first + datetime.timedelta(days=day),
                weight=round(weight, 1),
            )
        )

    return result


def body_fat(profile: Profile, today: datetime.date, rng: random.Random) -> list[BodyFat]:
    first = today - datetime.timedelta(days=DAYS - 1)
    days = []
    day = rng.randint(0, SKINFOLD_INTERVAL - 1)

    while day < DAYS:
        days.append(day)
        day += SKINFOLD_INTERVAL + rng.randint(-1, 1)

    skipped = event_indices(rng, len(days) // SKINFOLD_SKIP_RATE, len(days))
    result = []

    for index, day in enumerate(days):
        if index in skipped:
            continue
        factor = 1 + _fat_change(profile, day - SKINFOLD_LAG) * SKINFOLD_SENSITIVITY
        sites = SKINFOLD_SITES if index % SKINFOLD_FULL_RATE == 3 else profile.skinfold_sites
        result.append(
            BodyFat(
                user_id=profile.id,
                date=first + datetime.timedelta(days=day),
                **{
                    site: max(1, round(profile.skinfolds[site] * factor) + rng.randint(-1, 1))
                    for site in sites
                },
            )
        )

    return result


def period(profile: Profile, today: datetime.date, rng: random.Random) -> list[Period]:
    if profile.sex != Sex.FEMALE:
        return []

    first = today - datetime.timedelta(days=DAYS - 1)
    cycles = []
    day = rng.randint(0, 20)

    while day < DAYS - 6:
        cycles.append((day, rng.randint(3, 6), rng.randint(0, 1)))
        day += rng.randint(26, 32)

    unlogged = event_indices(rng, len(cycles) // 12, len(cycles))

    return [
        Period(
            user_id=profile.id,
            date=first + datetime.timedelta(days=day + offset),
            intensity=min(4, max(1, 4 - abs(offset - peak))),
        )
        for index, (day, duration, peak) in enumerate(cycles)
        if index not in unlogged
        for offset in range(duration)
    ]


def _trend(profile: Profile, day: int) -> float:
    """Return the body weight trend in kg, `day` days after the start of the history."""
    return profile.start_weight + _change(profile, day, fat=False)


def _fat_change(profile: Profile, day: int) -> float:
    """Return the change of the fat mass in kg, `day` days after the start of the history."""
    return _change(profile, day, fat=True)


def _change(profile: Profile, day: int, *, fat: bool) -> float:
    change = 0.0
    remaining = day

    for weeks, weight_slope, fat_slope in profile.weight_phases:
        slope = fat_slope if fat else weight_slope
        change += slope * min(max(remaining, 0), weeks * 7) / 7
        remaining -= weeks * 7

    return change


def _gaps(rng: random.Random, count: int, total: int) -> frozenset[int]:
    """Return the days of `count` gaps of 5 to 10 days each."""
    return frozenset(
        day
        for index in event_indices(rng, count, total)
        for day in range(index, index + rng.randint(5, 10))
    )
