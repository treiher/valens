use std::collections::BTreeMap;

use crate::{Assistance, Category, Equipment, Force, Laterality, Mechanic, Muscle, MuscleStimulus};

#[derive(Clone)]
pub struct Exercise {
    pub name: &'static str,
    pub muscles: &'static [(Muscle, MuscleStimulus)],
    pub force: Force,
    pub mechanic: Mechanic,
    pub laterality: Laterality,
    pub assistance: Assistance,
    pub equipment: &'static [Equipment],
    pub category: Category,
}

impl From<BaseExercise> for Exercise {
    fn from(value: BaseExercise) -> Self {
        Exercise {
            name: value.name,
            muscles: value.muscles,
            force: value.force,
            mechanic: value.mechanic,
            laterality: value.laterality,
            assistance: value.assistance,
            equipment: value.equipment,
            category: value.category,
        }
    }
}

#[derive(Clone)]
struct BaseExercise {
    pub name: &'static str,
    pub muscles: &'static [(Muscle, MuscleStimulus)],
    pub force: Force,
    pub mechanic: Mechanic,
    pub laterality: Laterality,
    pub assistance: Assistance,
    pub equipment: &'static [Equipment],
    pub category: Category,
    pub variants: &'static [ExerciseVariant],
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct ExerciseVariant {
    pub name: &'static str,
    pub muscles: Option<&'static [(Muscle, MuscleStimulus)]>,
    pub force: Option<Force>,
    pub mechanic: Option<Mechanic>,
    pub laterality: Option<Laterality>,
    pub assistance: Option<Assistance>,
    pub equipment: Option<&'static [Equipment]>,
    pub category: Option<Category>,
}

impl ExerciseVariant {
    const fn default() -> Self {
        Self {
            name: "",
            muscles: None,
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: None,
            category: None,
        }
    }
}

pub(crate) static EXERCISES: std::sync::LazyLock<BTreeMap<&'static str, Exercise>> =
    std::sync::LazyLock::new(|| {
        let mut exercises = EXERCISE_VARIANTS
            .into_iter()
            .map(std::convert::Into::into)
            .chain(EXERCISE_VARIANTS.iter().flat_map(|e| {
                e.variants.iter().map(|v| Exercise {
                    name: v.name,
                    force: v.force.unwrap_or(e.force),
                    mechanic: v.mechanic.unwrap_or(e.mechanic),
                    laterality: v.laterality.unwrap_or(e.laterality),
                    assistance: v.assistance.unwrap_or(e.assistance),
                    equipment: v.equipment.unwrap_or(e.equipment),
                    muscles: v.muscles.unwrap_or(e.muscles),
                    category: v.category.unwrap_or(e.category),
                })
            }))
            .collect::<Vec<Exercise>>();
        exercises.sort_by(|a, b| a.name.cmp(b.name));
        exercises
            .into_iter()
            .map(|e| (e.name, e))
            .collect::<BTreeMap<_, _>>()
    });

const EXERCISE_VARIANTS: [BaseExercise; 54] = [
    BaseExercise {
        name: "Back Extension",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (Muscle::ErectorSpinae, MuscleStimulus::Primary),
            (Muscle::Glutes, MuscleStimulus::Secondary),
            (Muscle::Hamstrings, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Band Pull Apart",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::ResistanceBand],
        muscles: &[
            (Muscle::RearDelts, MuscleStimulus::Primary),
            (Muscle::Lats, MuscleStimulus::Secondary),
            (Muscle::Traps, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Barbell Ab Rollout",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[(Muscle::Abs, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Kneeling Barbell Ab Rollout",
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Barbell Bench Press",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (Muscle::Pecs, MuscleStimulus::Primary),
            (Muscle::FrontDelts, MuscleStimulus::Primary),
            (Muscle::Triceps, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Decline Bench Press",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Incline Bench Press",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Cable Chest Press",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Bench Press",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Decline Bench Press",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Incline Bench Press",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Machine Chest Press",
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Smith Machine Bench Press",
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Barbell Curl",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[(Muscle::Biceps, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Drag Curl",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Preacher Curl",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Cable Curl",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Cable Preacher Curl",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Cable Rope Hammer Curl",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Curl",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Hammer Curl",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Incline Curl",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Lying Curl",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Preacher Curl",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Machine Preacher Curl",
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Barbell Deadlift",
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (Muscle::Glutes, MuscleStimulus::Primary),
            (Muscle::ErectorSpinae, MuscleStimulus::Primary),
            (Muscle::Quads, MuscleStimulus::Secondary),
            (Muscle::Hamstrings, MuscleStimulus::Secondary),
            (Muscle::Adductors, MuscleStimulus::Secondary),
            (Muscle::Traps, MuscleStimulus::Secondary),
            (Muscle::Forearms, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Deficit Deadlift",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Deficit Romanian Deadlift",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Deficit Stiff-Legged Deadlift",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Romanian Deadlift",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Stiff-Legged Deadlift",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Deadlift",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Deficit Deadlift",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Deficit Romanian Deadlift",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Deficit Stiff-Legged Deadlift",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Romanian Deadlift",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Stiff-Legged Deadlift",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Kettlebell Single-Leg Deadlift",
                laterality: Some(Laterality::Unilateral),
                equipment: Some(&[Equipment::Kettlebell]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Barbell Floor Press",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (Muscle::Pecs, MuscleStimulus::Primary),
            (Muscle::FrontDelts, MuscleStimulus::Primary),
            (Muscle::Triceps, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Dumbbell Floor Press",
            equipment: Some(&[Equipment::Dumbbell]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Barbell Good Morning",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (Muscle::Glutes, MuscleStimulus::Primary),
            (Muscle::ErectorSpinae, MuscleStimulus::Primary),
            (Muscle::Hamstrings, MuscleStimulus::Primary),
            (Muscle::Adductors, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Good Morning",
                equipment: Some(&[]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Smith Machine Good Morning",
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Barbell Hip Thrust",
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (Muscle::Glutes, MuscleStimulus::Primary),
            (Muscle::Quads, MuscleStimulus::Secondary),
            (Muscle::Adductors, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Machine Hip Thrust",
            equipment: Some(&[Equipment::Machine]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Barbell Overhead Triceps Extension",
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[(Muscle::Triceps, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Cable Incline Triceps Extension",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Cable Overhead Triceps Extension",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Overhead Triceps Extension",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Machine Overhead Triceps Extension",
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Barbell Pullover",
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (Muscle::Lats, MuscleStimulus::Primary),
            (Muscle::Pecs, MuscleStimulus::Primary),
        ],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Dumbbell Pullover",
            equipment: Some(&[Equipment::Dumbbell]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Barbell Rear Delt Row",
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (Muscle::RearDelts, MuscleStimulus::Primary),
            (Muscle::Traps, MuscleStimulus::Primary),
            (Muscle::Biceps, MuscleStimulus::Secondary),
            (Muscle::Lats, MuscleStimulus::Secondary),
            (Muscle::Forearms, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Cable Rope Rear Delt Row",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Rear Delt Row",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Barbell Shoulder Press",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (Muscle::FrontDelts, MuscleStimulus::Primary),
            (Muscle::SideDelts, MuscleStimulus::Secondary),
            (Muscle::Triceps, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Arnold Press",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Cable Shoulder Press",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Shoulder Press",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Machine Shoulder Press",
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Barbell Shrug",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (Muscle::Traps, MuscleStimulus::Primary),
            (Muscle::Forearms, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Cable Shrug",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Shrug",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Barbell Skull Crusher",
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[(Muscle::Triceps, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Band Skull Crusher",
                equipment: Some(&[Equipment::ResistanceBand]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Skull Crusher",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Box Jump",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Box],
        muscles: &[
            (Muscle::Quads, MuscleStimulus::Primary),
            (Muscle::Glutes, MuscleStimulus::Primary),
            (Muscle::Adductors, MuscleStimulus::Primary),
            (Muscle::Calves, MuscleStimulus::Secondary),
        ],
        category: Category::Plyometrics,
        variants: &[
            ExerciseVariant {
                name: "Lunge Jump",
                equipment: Some(&[]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Squat Jump",
                equipment: Some(&[]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Tuck Jump",
                equipment: Some(&[]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Cable Crossover",
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Cable],
        muscles: &[
            (Muscle::Pecs, MuscleStimulus::Primary),
            (Muscle::FrontDelts, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Cable Rope Face Pull",
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Cable],
        muscles: &[
            (Muscle::RearDelts, MuscleStimulus::Primary),
            (Muscle::SideDelts, MuscleStimulus::Secondary),
            (Muscle::Traps, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Cable Row",
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Cable],
        muscles: &[
            (Muscle::Lats, MuscleStimulus::Primary),
            (Muscle::Traps, MuscleStimulus::Primary),
            (Muscle::RearDelts, MuscleStimulus::Primary),
            (Muscle::Biceps, MuscleStimulus::Secondary),
            (Muscle::Forearms, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Cable Flexion Row",
            muscles: Some(&[
                (Muscle::ErectorSpinae, MuscleStimulus::Primary),
                (Muscle::Lats, MuscleStimulus::Primary),
                (Muscle::Traps, MuscleStimulus::Primary),
                (Muscle::RearDelts, MuscleStimulus::Primary),
                (Muscle::Biceps, MuscleStimulus::Secondary),
                (Muscle::Forearms, MuscleStimulus::Secondary),
            ]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Cossack Squat",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (Muscle::Glutes, MuscleStimulus::Primary),
            (Muscle::Quads, MuscleStimulus::Primary),
            (Muscle::Adductors, MuscleStimulus::Secondary),
            (Muscle::Hamstrings, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Crunch",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(Muscle::Abs, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Cable Crunch",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Machine Crunch",
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Dead Bug",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(Muscle::Abs, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Dip",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::ParallelBars],
        muscles: &[
            (Muscle::Pecs, MuscleStimulus::Primary),
            (Muscle::FrontDelts, MuscleStimulus::Primary),
            (Muscle::Triceps, MuscleStimulus::Primary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Bench Dip",
                equipment: Some(&[]),
                muscles: Some(&[
                    (Muscle::Triceps, MuscleStimulus::Primary),
                    (Muscle::Pecs, MuscleStimulus::Secondary),
                    (Muscle::FrontDelts, MuscleStimulus::Secondary),
                ]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Machine Dip",
                equipment: Some(&[Equipment::Machine]),
                muscles: Some(&[
                    (Muscle::Triceps, MuscleStimulus::Primary),
                    (Muscle::Pecs, MuscleStimulus::Secondary),
                    (Muscle::FrontDelts, MuscleStimulus::Secondary),
                ]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Ring Dip",
                equipment: Some(&[Equipment::GymnasticRings]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Dumbbell Fly",
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[
            (Muscle::Pecs, MuscleStimulus::Primary),
            (Muscle::FrontDelts, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Cable Fly",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Machine Fly",
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Dumbbell Incline Row",
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[
            (Muscle::Traps, MuscleStimulus::Primary),
            (Muscle::Lats, MuscleStimulus::Primary),
            (Muscle::RearDelts, MuscleStimulus::Primary),
            (Muscle::Biceps, MuscleStimulus::Secondary),
            (Muscle::Forearms, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Dumbbell Lateral Raise",
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[
            (Muscle::SideDelts, MuscleStimulus::Primary),
            (Muscle::FrontDelts, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Cable Lateral Raise",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Machine Lateral Raise",
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Dumbbell Reverse Fly",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[
            (Muscle::RearDelts, MuscleStimulus::Primary),
            (Muscle::Traps, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Cable Reverse Fly",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Machine Reverse Fly",
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Dumbbell Reverse Wrist Curl",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[(Muscle::Forearms, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Cable Reverse Wrist Curl",
            equipment: Some(&[Equipment::Cable]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Dumbbell Wrist Curl",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[(Muscle::Forearms, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Cable Wrist Curl",
            equipment: Some(&[Equipment::Cable]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Glute Bridge",
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (Muscle::Glutes, MuscleStimulus::Primary),
            (Muscle::Hamstrings, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Single-Leg Glute Bridge",
            laterality: Some(Laterality::Unilateral),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Handstand Push Up",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (Muscle::FrontDelts, MuscleStimulus::Primary),
            (Muscle::SideDelts, MuscleStimulus::Secondary),
            (Muscle::Triceps, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Hanging Leg Raise",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(Muscle::Abs, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Hanging Knee Raise",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Lying Leg Raise",
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Inverted Row",
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (Muscle::Lats, MuscleStimulus::Primary),
            (Muscle::Traps, MuscleStimulus::Primary),
            (Muscle::RearDelts, MuscleStimulus::Primary),
            (Muscle::Biceps, MuscleStimulus::Secondary),
            (Muscle::Forearms, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Ring Row",
            equipment: Some(&[Equipment::GymnasticRings]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Kettlebell Swing",
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Kettlebell],
        muscles: &[
            (Muscle::Glutes, MuscleStimulus::Primary),
            (Muscle::ErectorSpinae, MuscleStimulus::Primary),
            (Muscle::Hamstrings, MuscleStimulus::Secondary),
            (Muscle::Adductors, MuscleStimulus::Secondary),
            (Muscle::Traps, MuscleStimulus::Secondary),
            (Muscle::Forearms, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Lat Pulldown",
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (Muscle::Lats, MuscleStimulus::Primary),
            (Muscle::Biceps, MuscleStimulus::Secondary),
            (Muscle::RearDelts, MuscleStimulus::Secondary),
            (Muscle::Forearms, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Leg Extension",
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[(Muscle::Quads, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Leg Press",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (Muscle::Quads, MuscleStimulus::Primary),
            (Muscle::Glutes, MuscleStimulus::Primary),
            (Muscle::Adductors, MuscleStimulus::Primary),
            (Muscle::Hamstrings, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Lunge",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (Muscle::Quads, MuscleStimulus::Primary),
            (Muscle::Glutes, MuscleStimulus::Primary),
            (Muscle::Adductors, MuscleStimulus::Primary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Lunge",
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Walking Lunge",
                laterality: Some(Laterality::Bilateral),
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Lunge",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Side Lunge",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Walking Lunge",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Side Lunge",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Walking Lunge",
                laterality: Some(Laterality::Bilateral),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Machine Hack Squat",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (Muscle::Quads, MuscleStimulus::Primary),
            (Muscle::Glutes, MuscleStimulus::Secondary),
            (Muscle::Adductors, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Barbell Hack Squat",
            equipment: Some(&[Equipment::Barbell]),
            muscles: Some(&[
                (Muscle::Quads, MuscleStimulus::Primary),
                (Muscle::Glutes, MuscleStimulus::Secondary),
                (Muscle::Adductors, MuscleStimulus::Secondary),
                (Muscle::ErectorSpinae, MuscleStimulus::Secondary),
                (Muscle::Traps, MuscleStimulus::Secondary),
                (Muscle::Forearms, MuscleStimulus::Secondary),
                (Muscle::Calves, MuscleStimulus::Secondary),
            ]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Machine Hip Abduction",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (Muscle::Abductors, MuscleStimulus::Primary),
            (Muscle::Glutes, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Band Hip Abduction",
                laterality: Some(Laterality::Unilateral),
                equipment: Some(&[Equipment::ResistanceBand]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Cable Hip Abduction",
                laterality: Some(Laterality::Unilateral),
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Machine Hip Adduction",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[(Muscle::Adductors, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Band Hip Adduction",
                laterality: Some(Laterality::Unilateral),
                equipment: Some(&[Equipment::ResistanceBand]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Cable Hip Adduction",
                laterality: Some(Laterality::Unilateral),
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Machine Standing Calf Raise",
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[(Muscle::Calves, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Seated Calf Raise",
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Standing Calf Raise",
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Machine Seated Calf Raise",
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Machine-Assisted Dip",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Assisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (Muscle::Pecs, MuscleStimulus::Primary),
            (Muscle::FrontDelts, MuscleStimulus::Primary),
            (Muscle::Triceps, MuscleStimulus::Primary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Band-Assisted Dip",
                equipment: Some(&[Equipment::ResistanceBand, Equipment::PullUpBar]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Band-Assisted Ring Dip",
                equipment: Some(&[Equipment::ResistanceBand, Equipment::GymnasticRings]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Machine-Assisted Pull Up",
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Assisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (Muscle::Lats, MuscleStimulus::Primary),
            (Muscle::Traps, MuscleStimulus::Secondary),
            (Muscle::RearDelts, MuscleStimulus::Secondary),
            (Muscle::Biceps, MuscleStimulus::Secondary),
            (Muscle::Forearms, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Band-Assisted Pull Up",
                equipment: Some(&[Equipment::ResistanceBand, Equipment::PullUpBar]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Band-Assisted Ring Pull Up",
                equipment: Some(&[Equipment::ResistanceBand, Equipment::GymnasticRings]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Nordic Curl",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(Muscle::Hamstrings, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Plank",
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(Muscle::Abs, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Reverse Plank",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Side Plank",
                laterality: Some(Laterality::Unilateral),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Pull Up",
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::PullUpBar],
        muscles: &[
            (Muscle::Lats, MuscleStimulus::Primary),
            (Muscle::Traps, MuscleStimulus::Secondary),
            (Muscle::RearDelts, MuscleStimulus::Secondary),
            (Muscle::Biceps, MuscleStimulus::Secondary),
            (Muscle::Forearms, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "One-Arm Pull Up",
                laterality: Some(Laterality::Unilateral),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Ring Pull Up",
                equipment: Some(&[Equipment::GymnasticRings]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Push Up",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (Muscle::Pecs, MuscleStimulus::Primary),
            (Muscle::FrontDelts, MuscleStimulus::Primary),
            (Muscle::Triceps, MuscleStimulus::Secondary),
            (Muscle::Abs, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Decline Push Up",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Incline Push Up",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Kneeling Push Up",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "One-Arm Push Up",
                laterality: Some(Laterality::Unilateral),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Ring Push Up",
                equipment: Some(&[Equipment::GymnasticRings]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Reverse Nordic Curl",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(Muscle::Quads, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Seated Leg Curl",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[(Muscle::Hamstrings, MuscleStimulus::Primary)],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Lying Leg Curl",
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Slider Hamstring Curl",
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Sliders],
        muscles: &[
            (Muscle::Hamstrings, MuscleStimulus::Primary),
            (Muscle::Glutes, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Ball Hamstring Curl",
                equipment: Some(&[Equipment::ExerciseBall]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Ring Hamstring Curl",
                equipment: Some(&[Equipment::GymnasticRings]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Split Squat",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (Muscle::Quads, MuscleStimulus::Primary),
            (Muscle::Glutes, MuscleStimulus::Primary),
            (Muscle::Adductors, MuscleStimulus::Primary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Bulgarian Split Squat",
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Split Squat",
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Bulgarian Split Squat",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Bulgarian Split Squat",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Split Squat",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Smith Machine Bulgarian Split Squat",
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Smith Machine Split Squat",
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Squat",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (Muscle::Quads, MuscleStimulus::Primary),
            (Muscle::Glutes, MuscleStimulus::Primary),
            (Muscle::Adductors, MuscleStimulus::Primary),
            (Muscle::ErectorSpinae, MuscleStimulus::Primary),
            (Muscle::Calves, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Squat",
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Squat",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Goblet Squat",
                equipment: Some(&[Equipment::Kettlebell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Smith Machine Squat",
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Step Up",
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (Muscle::Quads, MuscleStimulus::Primary),
            (Muscle::Glutes, MuscleStimulus::Primary),
            (Muscle::Adductors, MuscleStimulus::Primary),
            (Muscle::Hamstrings, MuscleStimulus::Secondary),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Step Up",
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Step Up",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashSet;

    #[test]
    fn test_exercise_variant() {
        assert_eq!(
            ExerciseVariant::default(),
            ExerciseVariant {
                name: "",
                muscles: None,
                force: None,
                mechanic: None,
                laterality: None,
                assistance: None,
                equipment: None,
                category: None,
            }
        );
    }

    #[test]
    fn test_exercises() {
        for (name, exercise) in EXERCISES.iter() {
            assert_eq!(*name, exercise.name);

            assert!(match exercise.mechanic {
                Mechanic::Isolation =>
                    exercise
                        .muscles
                        .iter()
                        .filter(|(_, s)| *s == MuscleStimulus::Primary)
                        .collect::<Vec<_>>()
                        .len()
                        == 1,
                Mechanic::Compound =>
                    !exercise
                        .muscles
                        .iter()
                        .filter(|(_, s)| *s == MuscleStimulus::Primary)
                        .collect::<Vec<_>>()
                        .is_empty()
                        && exercise.muscles.len() > 1,
            });

            if name.contains("One-") {
                assert!(exercise.laterality == Laterality::Unilateral);
            }

            assert!(match exercise.assistance {
                Assistance::Assisted => name.contains("Assisted"),
                Assistance::Unassisted => !name.contains("Assisted"),
            });
            if name.contains("Barbell") {
                assert!(exercise.equipment.contains(&Equipment::Barbell));
            }
            if name.contains("Box") {
                assert!(exercise.equipment.contains(&Equipment::Box));
            }
            if name.contains("Cable") {
                assert!(exercise.equipment.contains(&Equipment::Cable));
            }
            if name.contains("Dumbbell") {
                assert!(exercise.equipment.contains(&Equipment::Dumbbell));
            }
            if name.contains("Ball") {
                assert!(exercise.equipment.contains(&Equipment::ExerciseBall));
            }
            if name.contains("Ring") {
                assert!(exercise.equipment.contains(&Equipment::GymnasticRings));
            }
            if name.contains("Kettlebell") {
                assert!(exercise.equipment.contains(&Equipment::Kettlebell));
            }
            if name.contains("Machine") {
                assert!(exercise.equipment.contains(&Equipment::Machine));
            }
            if name.contains("Band") {
                assert!(exercise.equipment.contains(&Equipment::ResistanceBand));
            }
            if name.contains("Slider") {
                assert!(exercise.equipment.contains(&Equipment::Sliders));
            }
        }
    }

    #[test]
    fn test_exercise_variants_order() {
        let exercise_names = EXERCISE_VARIANTS.iter().map(|e| e.name).collect::<Vec<_>>();
        let mut exercise_names_sorted = exercise_names.clone();
        exercise_names_sorted.sort_unstable();
        assert_eq!(exercise_names, exercise_names_sorted, "unsorted");

        for exercise in EXERCISE_VARIANTS {
            let variant_names = exercise.variants.iter().map(|e| e.name).collect::<Vec<_>>();
            let mut variant_names_sorted = variant_names.clone();
            variant_names_sorted.sort_unstable();
            assert_eq!(variant_names, variant_names_sorted, "unsorted");
        }
    }

    #[test]
    fn test_exercise_variants_duplicate_names() {
        let mut exercise_names = HashSet::new();

        for exercise in EXERCISE_VARIANTS {
            let name = exercise.name;
            assert!(!exercise_names.contains(name), "duplicate name {name}");
            exercise_names.insert(name);

            for variant in exercise.variants {
                let name = variant.name;
                assert!(!exercise_names.contains(name), "duplicate name {name}");
                exercise_names.insert(name);
            }
        }
    }

    #[test]
    fn test_exercise_variants_duplicate_muscles() {
        for exercise in EXERCISE_VARIANTS {
            let muscles: HashSet<Muscle> =
                exercise.muscles.iter().map(|(m, _)| m).copied().collect();
            assert_eq!(
                exercise.muscles.len(),
                muscles.len(),
                "duplicate muscle entries for \"{}\"",
                exercise.name
            );

            for variant in exercise.variants {
                let muscles: HashSet<Muscle> = variant
                    .muscles
                    .unwrap_or_default()
                    .iter()
                    .map(|(m, _)| m)
                    .copied()
                    .collect();
                assert_eq!(
                    variant.muscles.unwrap_or_default().len(),
                    muscles.len(),
                    "duplicate muscle entries for \"{}\"",
                    exercise.name
                );
            }
        }
    }

    #[test]
    fn test_exercise_variants_invalid_muscles() {
        for exercise in EXERCISE_VARIANTS {
            for (muscle, _) in exercise
                .muscles
                .iter()
                .chain(exercise.variants.iter().filter_map(|v| v.muscles).flatten())
            {
                assert_ne!(*muscle, Muscle::None);
            }
        }
    }

    #[test]
    fn test_exercise_variants_invalid_equipment() {
        for exercise in EXERCISE_VARIANTS {
            for equipment in exercise.equipment.iter().chain(
                exercise
                    .variants
                    .iter()
                    .filter_map(|v| v.equipment)
                    .flatten(),
            ) {
                assert_ne!(*equipment, Equipment::None);
            }
        }
    }
}
