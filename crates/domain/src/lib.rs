#![warn(clippy::pedantic)]
#![allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]

pub mod catalog;

pub use body_fat::{BodyFat, BodyFatRepository};
pub use body_weight::{BodyWeight, BodyWeightRepository, avg_body_weight, avg_weekly_change};
pub use error::{CreateError, DeleteError, ReadError, StorageError, SyncError, UpdateError};
pub use exercise::{
    Assistance, Category, Equipment, Exercise, ExerciseFilter, ExerciseID, ExerciseMuscle,
    ExerciseRepository, Force, Laterality, Mechanic, MuscleID, MuscleIDError, Property, Stimulus,
    StimulusError,
};
pub use name::{Name, NameError};
pub use period::{
    CurrentCycle, Cycle, Intensity, IntensityError, Period, PeriodRepository, current_cycle,
    cycle_stats, cycles,
};
pub use routine::{
    Routine, RoutineID, RoutinePart, RoutineRepository, routines_sorted_by_last_use,
};
pub use session::SessionRepository;
pub use statistics::{
    DefaultInterval, Interval, centered_moving_average, centered_moving_total, init_interval,
    value_based_centered_moving_average,
};
pub use training::{RIR, RPE, Reps, Time, TrainingStats, Weight, training_stats};
pub use training_session::{
    TrainingSession, TrainingSessionElement, TrainingSessionID, TrainingSessionRepository,
};
pub use user::{Sex, User, UserID, UserRepository};
pub use version::VersionRepository;

mod body_fat;
mod body_weight;
mod error;
mod exercise;
mod name;
mod period;
mod routine;
mod session;
mod statistics;
mod training;
mod training_session;
mod user;
mod version;
