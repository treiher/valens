import tests.data
from valens import storage


def initialize_users() -> None:
    storage.initialize()
    storage.write_users(tests.data.USERS_DF)


def initialize_data() -> None:
    initialize_users()
    storage.write_routine_sets(tests.data.ROUTINE_SETS_DF, 1)
    storage.write_routines(tests.data.ROUTINES_DF, 1)
    storage.write_sets(tests.data.SETS_DF, 1)
    storage.write_workouts(tests.data.WORKOUTS_DF, 1)
    storage.write_bodyweight(tests.data.BODYWEIGHT_DF, 1)
    storage.write_bodyfat(tests.data.BODYFAT_DF, 1)
    storage.write_period(tests.data.PERIOD_DF, 1)
    storage.write_routine_sets(tests.data.ROUTINE_SETS_DF, 2)
    storage.write_routines(tests.data.ROUTINES_DF, 2)
    storage.write_sets(tests.data.SETS_DF, 2)
    storage.write_workouts(tests.data.WORKOUTS_DF, 2)
    storage.write_bodyweight(tests.data.BODYWEIGHT_DF, 2)
    storage.write_bodyfat(tests.data.BODYFAT_DF, 2)
    storage.write_period(tests.data.PERIOD_DF, 2)
