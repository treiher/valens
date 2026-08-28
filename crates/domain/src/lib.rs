#![warn(clippy::pedantic)]
#![allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]

pub mod catalog;

pub use auth::{AuthMethod, AuthRepository, AuthService, Passkey, PasskeyID};
pub use body_fat::{BodyFat, BodyFatRepository, BodyFatService};
pub use body_weight::{
    BodyWeight, BodyWeightRepository, BodyWeightService, avg_body_weight, avg_weekly_change,
};
pub use error::{
    CreateError, DeleteError, ReadError, Recoverable, StorageError, SyncError, Unreachable,
    UpdateError, ValidationError,
};
pub use exercise::{
    Assistance, AssistanceError, CatalogMatch, CatalogProperties, CatalogUpdate, CatalogUpdateMode,
    Category, CategoryError, Equipment, EquipmentError, Exercise, ExerciseFilter, ExerciseID,
    ExerciseMuscle, ExerciseProperty, ExerciseRepository, ExerciseService, Force, ForceError,
    Laterality, LateralityError, Mechanic, MechanicError, MuscleID, MuscleIDError, Property,
    PropertyChange, PropertyValue, Stimulus, StimulusError, catalog_update, catalog_updates,
    name_or_none,
};
pub use ffmi::ffmi;
pub use name::{Name, NameError};
pub use period::{
    CurrentCycle, Cycle, Intensity, IntensityError, Period, PeriodRepository, PeriodService,
    current_cycle, cycle_stats, cycles,
};
pub use routine::{
    Rounds, Routine, RoutineID, RoutinePart, RoutinePartPath, RoutineRepository, RoutineService,
    routines_sorted_by_last_use,
};
pub use schedule::{
    Rotation, RotationError, RotationID, Schedule, ScheduleError, ScheduleRepository,
    ScheduleService, ScheduleSlot, Weekday, WeekdayError,
};
pub use service::Service;
pub use session::{SessionRepository, SessionService, SignOut};
pub use statistics::{
    DefaultInterval, Interval, centered_moving_average, centered_moving_max, centered_moving_min,
    centered_moving_total, init_interval, value_based_centered_moving_average,
};
pub use training::{
    RIR, RPE, Reps, Time, TrainingStats, Weight, drop_set_weights, one_rep_max,
    reps_for_percentage, round_drop_to_increment, training_stats,
};
pub use training_session::{
    Set, TrainingSession, TrainingSessionElement, TrainingSessionID, TrainingSessionRepository,
    TrainingSessionSection, TrainingSessionService, most_recent_best_set_for_one_rep_max,
};
pub use user::{Role, Sex, User, UserID, UserRepository, UserService};
pub use version::{VersionRepository, VersionService};

mod auth;
mod body_fat;
mod body_weight;
mod error;
mod exercise;
mod ffmi;
mod name;
mod period;
mod routine;
mod schedule;
mod service;
mod session;
mod statistics;
mod training;
mod training_session;
mod user;
mod version;
