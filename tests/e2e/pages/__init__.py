"""Playwright page objects for end-to-end tests."""

from .admin import AdminPage as AdminPage
from .body_fat import BodyFatPage as BodyFatPage
from .body_weight import BodyWeightPage as BodyWeightPage
from .exercise import ExercisePage as ExercisePage
from .exercises import ExercisesPage as ExercisesPage
from .home import HomePage as HomePage
from .login import LoginPage as LoginPage
from .menstrual_cycle import MenstrualCyclePage as MenstrualCyclePage
from .muscles import MusclesPage as MusclesPage
from .routine import (
    RoutinePage as RoutinePage,
    RoutineRest as RoutineRest,
    RoutineSection as RoutineSection,
    RoutineSet as RoutineSet,
)
from .routines import RoutinesPage as RoutinesPage
from .training_session import (
    DropSetCalculatorDialog as DropSetCalculatorDialog,
    OneRepMaxCalculatorDialog as OneRepMaxCalculatorDialog,
    TrainingSessionPage as TrainingSessionPage,
)
from .training_sessions import TrainingSessionsPage as TrainingSessionsPage
