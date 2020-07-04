import datetime
from typing import Final

import pandas as pd

WORKOUTS: Final = {
    datetime.date(2002, 2, 20): {"E1": ["12@8", "11@8", "10@8"], "E2": ["9@9", "8@9"]},
    datetime.date(2002, 2, 22): {"E4": ["7@8", "6@8", "5@8"], "E3": ["4@9", "3@9"]},
}

WORKOUTS_DF: Final = pd.DataFrame(
    {
        "date": [datetime.date(2002, 2, 20)] * 5 + [datetime.date(2002, 2, 22)] * 5,
        "exercise": ["E1", "E1", "E1", "E2", "E2", "E4", "E4", "E4", "E3", "E3"],
        "reps": list(map(float, range(12, 2, -1))),
        "time": [float("nan")] * 10,
        "weight": [float("nan")] * 10,
        "rpe": [8.0, 8.0, 8.0, 9.0, 9.0, 8.0, 8.0, 8.0, 9.0, 9.0],
        "rir": [2.0, 2.0, 2.0, 1.0, 1.0, 2.0, 2.0, 2.0, 1.0, 1.0],
    }
)

BODYWEIGHT: Final = {datetime.date(2002, 2, 20): 81.2, datetime.date(2002, 2, 22): 82.4}

BODYWEIGHT_DF: Final = pd.DataFrame(
    {"date": [datetime.date(2002, 2, 20), datetime.date(2002, 2, 22)], "weight": [81.2, 82.4]}
)

TEMPLATES: Final = {
    "T2": {"E4": [None, None, None], "E3": [None, None]},
    "T1": {"E1": [None, None, None], "E2": [None, None]},
}

TEMPLATES_DF: Final = {
    "T2": pd.DataFrame(
        {
            "exercise": ["E4", "E4", "E4", "E3", "E3"],
            "reps": [float("nan")] * 5,
            "time": [float("nan")] * 5,
            "weight": [float("nan")] * 5,
            "rpe": [float("nan")] * 5,
        }
    ),
    "T1": pd.DataFrame(
        {
            "exercise": ["E1", "E1", "E1", "E2", "E2"],
            "reps": [float("nan")] * 5,
            "time": [float("nan")] * 5,
            "weight": [float("nan")] * 5,
            "rpe": [float("nan")] * 5,
        }
    ),
}
