use chrono::{Duration, Local, NaiveDate};
use valens_domain as domain;

pub static USERS: std::sync::LazyLock<Vec<domain::User>> =
    std::sync::LazyLock::new(|| vec![USER.clone(), USER_2.clone()]);

pub static USER: std::sync::LazyLock<domain::User> = std::sync::LazyLock::new(|| domain::User {
    id: 1.into(),
    name: domain::Name::new("Alice").unwrap(),
    sex: domain::Sex::FEMALE,
});

pub static USER_2: std::sync::LazyLock<domain::User> = std::sync::LazyLock::new(|| domain::User {
    id: 2.into(),
    name: domain::Name::new("Bob").unwrap(),
    sex: domain::Sex::MALE,
});

pub const BODY_WEIGHTS: &[domain::BodyWeight; 2] = &[BODY_WEIGHT, BODY_WEIGHT_2];

pub const BODY_WEIGHT: domain::BodyWeight = domain::BodyWeight {
    date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
    weight: 80.0,
};

pub const BODY_WEIGHT_2: domain::BodyWeight = domain::BodyWeight {
    date: NaiveDate::from_ymd_opt(2020, 2, 3).unwrap(),
    weight: 80.8,
};

pub const BODY_FATS: &[domain::BodyFat; 2] = &[BODY_FAT, BODY_FAT_2];

pub const BODY_FAT: domain::BodyFat = domain::BodyFat {
    date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
    chest: Some(1),
    abdominal: Some(2),
    thigh: Some(3),
    tricep: Some(4),
    subscapular: Some(5),
    suprailiac: Some(6),
    midaxillary: Some(7),
};

pub const BODY_FAT_2: domain::BodyFat = domain::BodyFat {
    date: NaiveDate::from_ymd_opt(2020, 2, 3).unwrap(),
    chest: Some(2),
    abdominal: Some(3),
    thigh: Some(4),
    tricep: Some(5),
    subscapular: Some(6),
    suprailiac: Some(7),
    midaxillary: Some(8),
};

pub const PERIODS: &[domain::Period; 2] = &[PERIOD, PERIOD_2];

pub const PERIOD: domain::Period = domain::Period {
    date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
    intensity: domain::Intensity::Light,
};

pub const PERIOD_2: domain::Period = domain::Period {
    date: NaiveDate::from_ymd_opt(2020, 2, 3).unwrap(),
    intensity: domain::Intensity::Medium,
};

pub static EXERCISES: std::sync::LazyLock<Vec<domain::Exercise>> =
    std::sync::LazyLock::new(|| vec![EXERCISE.clone(), EXERCISE_2.clone()]);

pub static EXERCISE: std::sync::LazyLock<domain::Exercise> =
    std::sync::LazyLock::new(|| domain::Exercise {
        id: 1.into(),
        name: domain::Name::new("A").unwrap(),
        muscles: vec![domain::ExerciseMuscle {
            muscle_id: domain::MuscleID::Abs,
            stimulus: domain::Stimulus::PRIMARY,
        }],
    });

pub static EXERCISE_2: std::sync::LazyLock<domain::Exercise> =
    std::sync::LazyLock::new(|| domain::Exercise {
        id: 2.into(),
        name: domain::Name::new("B").unwrap(),
        muscles: vec![domain::ExerciseMuscle {
            muscle_id: domain::MuscleID::Pecs,
            stimulus: domain::Stimulus::PRIMARY,
        }],
    });

pub static ROUTINES: std::sync::LazyLock<Vec<domain::Routine>> =
    std::sync::LazyLock::new(|| vec![ROUTINE.clone(), ROUTINE_2.clone()]);

pub static ROUTINE: std::sync::LazyLock<domain::Routine> =
    std::sync::LazyLock::new(|| domain::Routine {
        id: 1.into(),
        name: domain::Name::new("A").unwrap(),
        notes: String::from("B"),
        archived: false,
        sections: vec![
            domain::RoutinePart::RoutineSection {
                rounds: domain::Rounds::new(2).unwrap(),
                parts: vec![
                    domain::RoutinePart::RoutineActivity {
                        exercise_id: 1.into(),
                        reps: domain::Reps::new(10).unwrap(),
                        time: domain::Time::new(2).unwrap(),
                        weight: domain::Weight::new(30.0).unwrap(),
                        rpe: domain::RPE::TEN,
                        automatic: false,
                    },
                    domain::RoutinePart::RoutineActivity {
                        exercise_id: domain::ExerciseID::nil(),
                        reps: domain::Reps::default(),
                        time: domain::Time::new(60).unwrap(),
                        weight: domain::Weight::default(),
                        rpe: domain::RPE::ZERO,
                        automatic: true,
                    },
                ],
            },
            domain::RoutinePart::RoutineSection {
                rounds: domain::Rounds::new(2).unwrap(),
                parts: vec![
                    domain::RoutinePart::RoutineActivity {
                        exercise_id: 2.into(),
                        reps: domain::Reps::new(10).unwrap(),
                        time: domain::Time::default(),
                        weight: domain::Weight::default(),
                        rpe: domain::RPE::ZERO,
                        automatic: false,
                    },
                    domain::RoutinePart::RoutineActivity {
                        exercise_id: domain::ExerciseID::nil(),
                        reps: domain::Reps::default(),
                        time: domain::Time::new(30).unwrap(),
                        weight: domain::Weight::default(),
                        rpe: domain::RPE::ZERO,
                        automatic: true,
                    },
                ],
            },
        ],
    });

pub static ROUTINE_2: std::sync::LazyLock<domain::Routine> =
    std::sync::LazyLock::new(|| domain::Routine {
        id: 2.into(),
        name: domain::Name::new("B").unwrap(),
        notes: String::from("C"),
        archived: false,
        sections: vec![domain::RoutinePart::RoutineSection {
            rounds: domain::Rounds::new(1).unwrap(),
            parts: vec![
                domain::RoutinePart::RoutineActivity {
                    exercise_id: 1.into(),
                    reps: domain::Reps::default(),
                    time: domain::Time::default(),
                    weight: domain::Weight::default(),
                    rpe: domain::RPE::ZERO,
                    automatic: false,
                },
                domain::RoutinePart::RoutineActivity {
                    exercise_id: domain::ExerciseID::nil(),
                    reps: domain::Reps::default(),
                    time: domain::Time::default(),
                    weight: domain::Weight::default(),
                    rpe: domain::RPE::ZERO,
                    automatic: false,
                },
            ],
        }],
    });

pub static TODAY: std::sync::LazyLock<NaiveDate> =
    std::sync::LazyLock::new(|| Local::now().date_naive());

pub static TRAINING_SESSIONS: std::sync::LazyLock<Vec<domain::TrainingSession>> =
    std::sync::LazyLock::new(|| vec![TRAINING_SESSION.clone(), TRAINING_SESSION_2.clone()]);

pub static TRAINING_SESSION: std::sync::LazyLock<domain::TrainingSession> =
    std::sync::LazyLock::new(|| domain::TrainingSession {
        id: 1.into(),
        routine_id: 2.into(),
        date: *TODAY - Duration::days(10),
        notes: String::from("A"),
        elements: vec![
            domain::TrainingSessionElement::Set {
                exercise_id: 1.into(),
                reps: Some(domain::Reps::new(10).unwrap()),
                time: Some(domain::Time::new(3).unwrap()),
                weight: Some(domain::Weight::new(30.0).unwrap()),
                rpe: Some(domain::RPE::EIGHT),
                target_reps: Some(domain::Reps::new(8).unwrap()),
                target_time: Some(domain::Time::new(4).unwrap()),
                target_weight: Some(domain::Weight::new(40.0).unwrap()),
                target_rpe: Some(domain::RPE::NINE),
                automatic: false,
            },
            domain::TrainingSessionElement::Rest {
                target_time: Some(domain::Time::new(60).unwrap()),
                automatic: true,
            },
            domain::TrainingSessionElement::Set {
                exercise_id: 2.into(),
                reps: Some(domain::Reps::new(5).unwrap()),
                time: Some(domain::Time::new(4).unwrap()),
                weight: None,
                rpe: Some(domain::RPE::FOUR),
                target_reps: None,
                target_time: None,
                target_weight: None,
                target_rpe: None,
                automatic: false,
            },
            domain::TrainingSessionElement::Rest {
                target_time: Some(domain::Time::new(60).unwrap()),
                automatic: true,
            },
            domain::TrainingSessionElement::Set {
                exercise_id: 2.into(),
                reps: None,
                time: Some(domain::Time::new(60).unwrap()),
                weight: None,
                rpe: None,
                target_reps: None,
                target_time: None,
                target_weight: None,
                target_rpe: None,
                automatic: false,
            },
            domain::TrainingSessionElement::Rest {
                target_time: Some(domain::Time::new(60).unwrap()),
                automatic: true,
            },
        ],
    });

pub static TRAINING_SESSION_2: std::sync::LazyLock<domain::TrainingSession> =
    std::sync::LazyLock::new(|| domain::TrainingSession {
        id: 2.into(),
        routine_id: 2.into(),
        date: *TODAY - Duration::days(8),
        notes: String::default(),
        elements: vec![
            domain::TrainingSessionElement::Set {
                exercise_id: 1.into(),
                reps: Some(domain::Reps::new(5).unwrap()),
                time: Some(domain::Time::new(4).unwrap()),
                weight: Some(domain::Weight::new(60.0).unwrap()),
                rpe: Some(domain::RPE::EIGHT),
                target_reps: None,
                target_time: None,
                target_weight: None,
                target_rpe: None,
                automatic: false,
            },
            domain::TrainingSessionElement::Rest {
                target_time: None,
                automatic: true,
            },
        ],
    });
