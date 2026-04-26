import datetime
import os

import tests.data
from valens import models

VALENS = "build/venv/bin/valens"
HOST = "127.0.0.1"
PORT = 53535 + int(os.getenv("PYTEST_XDIST_WORKER", "gw0")[2:])
BASE_URL = f"http://{HOST}:{PORT}"

TODAY = datetime.date.today()
USERS = tests.data.users(today=TODAY)
USER = USERS[0]
USERNAMES = [user.name for user in USERS]
EXERCISES_IN_CURRENT_WORKOUTS = {
    e.exercise.name
    for w in USER.workouts
    for e in w.elements
    if isinstance(e, models.WorkoutSet) and w.date >= TODAY - datetime.timedelta(31)
}
PREVIOUS_WORKOUT_EXERCISES = {
    e.exercise.name
    for w in USER.workouts
    for e in w.elements
    if isinstance(e, models.WorkoutSet) and e.exercise.name not in EXERCISES_IN_CURRENT_WORKOUTS
}
CURRENT_WORKOUT_EXERCISES = {
    e.name
    for e in USER.exercises
    if e.name in EXERCISES_IN_CURRENT_WORKOUTS or e.name not in PREVIOUS_WORKOUT_EXERCISES
}
