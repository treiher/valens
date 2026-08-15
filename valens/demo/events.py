"""
Placement of rare events in the example data.

Events are spread evenly over the available positions and only jittered by the random number
generator, so that a given number of events occurs for every seed.
"""

from __future__ import annotations

import random


def event_indices(rng: random.Random, count: int, total: int) -> frozenset[int]:
    """Return `count` indices, spread over `range(total)` and never at its bounds."""
    step = total // (count + 1)
    return frozenset(
        step * (index + 1) + rng.randint(-(step // 3), step // 3) for index in range(count)
    )
