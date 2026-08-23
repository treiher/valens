"""Playwright page objects for end-to-end tests."""

from .about import AboutDialog as AboutDialog
from .admin import AdminDialog as AdminDialog
from .body_fat import BodyFatPage as BodyFatPage
from .body_weight import BodyWeightPage as BodyWeightPage
from .exercise import ExercisePage as ExercisePage
from .exercises import ExercisesPage as ExercisesPage
from .ffmi import FfmiPage as FfmiPage
from .home import HomePage as HomePage
from .login import LoginPage as LoginPage
from .menstrual_cycle import MenstrualCyclePage as MenstrualCyclePage
from .muscles import MusclesPage as MusclesPage
from .profile import ProfileDialog as ProfileDialog
from .registration import PasskeyRegistrationView as PasskeyRegistrationView
from .routine import (
    RoutinePage as RoutinePage,
    RoutineRest as RoutineRest,
    RoutineSection as RoutineSection,
    RoutineSet as RoutineSet,
)
from .routines import RoutinesPage as RoutinesPage
from .schedule import SchedulePage as SchedulePage
from .settings import SettingsDialog as SettingsDialog
from .training_session import (
    DropSetCalculatorDialog as DropSetCalculatorDialog,
    OneRepMaxCalculatorDialog as OneRepMaxCalculatorDialog,
    TrainingSessionPage as TrainingSessionPage,
)
from .training_sessions import TrainingSessionsPage as TrainingSessionsPage
from .update import UpdateDialog as UpdateDialog
