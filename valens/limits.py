"""Values mirrored from the domain crate, kept in sync by hand."""

# Mirrors `MuscleID` in `crates/domain/src/exercise.rs`
MUSCLE_IDS = frozenset({1, 11, 21, 22, 31, 32, 33, 41, 42, 51, 61, 62, 71, 72, 81, 82, 83, 91})

# Mirrors `height` in `crates/domain/src/user.rs`
HEIGHT_MIN = 1
HEIGHT_MAX = 255

# Mirrors the skinfold fields in `crates/domain/src/body_fat.rs`
SKINFOLD_MIN = 1
SKINFOLD_MAX = 255

# Mirrors `Intensity` in `crates/domain/src/period.rs`
INTENSITY_MIN = 1
INTENSITY_MAX = 4

# Mirrors `Stimulus` in `crates/domain/src/exercise.rs`. The domain also permits 0, which must
# not be stored.
STIMULUS_MIN = 1
STIMULUS_MAX = 100

# Mirrors `Rounds` in `crates/domain/src/routine.rs`
ROUNDS_MIN = 1
ROUNDS_MAX = 999

# Mirrors `Reps` in `crates/domain/src/training.rs`
REPS_MAX = 999

# Mirrors `Time` in `crates/domain/src/training.rs`
TIME_MAX = 999

# Mirrors `Weight` in `crates/domain/src/training.rs`
WEIGHT_MAX = 999.99
WEIGHT_RESOLUTION = 0.01

# Mirrors `RPE` in `crates/domain/src/training.rs`
RPE_MIN = 0
RPE_MAX = 10
RPE_RESOLUTION = 0.5

# Mirrors `Weekday` in `crates/domain/src/schedule.rs`
WEEKDAY_MIN = 1
WEEKDAY_MAX = 7
