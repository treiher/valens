use std::collections::BTreeMap;

use crate::{
    Assistance, Category, Equipment, Force, Laterality, Mechanic, MuscleID, Name, Stimulus,
};

#[derive(Clone)]
pub struct Exercise {
    pub name: Name,
    /// Alternative names under which the exercise is commonly known.
    pub aliases: &'static [&'static str],
    pub muscles: &'static [(MuscleID, Stimulus)],
    pub stretch: &'static [(MuscleID, Stimulus)],
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
            name: Name::new(value.name).unwrap(),
            aliases: value.aliases,
            muscles: value.muscles,
            stretch: value.stretch,
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
    pub aliases: &'static [&'static str],
    pub muscles: &'static [(MuscleID, Stimulus)],
    pub stretch: &'static [(MuscleID, Stimulus)],
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
    pub aliases: &'static [&'static str],
    pub muscles: Option<&'static [(MuscleID, Stimulus)]>,
    pub stretch: Option<&'static [(MuscleID, Stimulus)]>,
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
            aliases: &[],
            muscles: None,
            stretch: None,
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: None,
            category: None,
        }
    }
}

pub(crate) static EXERCISES: std::sync::LazyLock<BTreeMap<Name, Exercise>> =
    std::sync::LazyLock::new(|| {
        let mut exercises = EXERCISE_VARIANTS
            .into_iter()
            .map(std::convert::Into::into)
            .chain(EXERCISE_VARIANTS.iter().flat_map(|e| {
                e.variants.iter().map(|v| Exercise {
                    name: Name::new(v.name).unwrap(),
                    aliases: v.aliases,
                    force: v.force.unwrap_or(e.force),
                    mechanic: v.mechanic.unwrap_or(e.mechanic),
                    laterality: v.laterality.unwrap_or(e.laterality),
                    assistance: v.assistance.unwrap_or(e.assistance),
                    equipment: v.equipment.unwrap_or(e.equipment),
                    muscles: v.muscles.unwrap_or(e.muscles),
                    stretch: v.stretch.unwrap_or(e.stretch),
                    category: v.category.unwrap_or(e.category),
                })
            }))
            .collect::<Vec<Exercise>>();
        exercises.sort_by(|a, b| a.name.cmp(&b.name));
        exercises
            .into_iter()
            .map(|e| (e.name.clone(), e))
            .collect::<BTreeMap<_, _>>()
    });

#[must_use]
pub fn exercise(name: &Name) -> Option<&'static Exercise> {
    EXERCISES.get(name)
}

/// The words of a normalized name, each with the equipment it names, if any.
///
/// A name is split into words at every character that is neither a letter nor a digit. A phrase of
/// two words naming equipment, such as "trap bar", counts as one word.
pub(crate) fn equipment_words(normalized_name: &str) -> Vec<Option<Equipment>> {
    let words = normalized_name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>();
    let equipment = |phrase: &str| {
        EQUIPMENT_WORDS
            .iter()
            .find(|(word, _)| *word == phrase)
            .map(|(_, equipment)| *equipment)
    };
    let mut result = vec![];
    let mut i = 0;
    while i < words.len() {
        if let Some(e) = words
            .get(i + 1)
            .and_then(|next| equipment(&format!("{} {next}", words[i])))
        {
            result.push(Some(e));
            i += 2;
        } else {
            result.push(equipment(words[i]));
            i += 1;
        }
    }
    result
}

/// The words naming equipment in exercise names.
///
/// The parallel bars and the pull-up bar are left out, as names never name them. They are the
/// apparatus of the bodyweight forms. A bare "ball" is left out, as it also names the medicine ball,
/// the slam ball and the Bosu ball.
const EQUIPMENT_WORDS: [(&str, Equipment); 32] = [
    ("barbell", Equipment::Barbell),
    ("barbells", Equipment::Barbell),
    ("bb", Equipment::Barbell),
    ("box", Equipment::Box),
    ("boxes", Equipment::Box),
    ("cable", Equipment::Cable),
    ("cables", Equipment::Cable),
    ("dumbbell", Equipment::Dumbbell),
    ("dumbbells", Equipment::Dumbbell),
    ("db", Equipment::Dumbbell),
    ("exercise ball", Equipment::ExerciseBall),
    ("exercise balls", Equipment::ExerciseBall),
    ("ring", Equipment::GymnasticRings),
    ("rings", Equipment::GymnasticRings),
    ("gymnastic rings", Equipment::GymnasticRings),
    ("kettlebell", Equipment::Kettlebell),
    ("kettlebells", Equipment::Kettlebell),
    ("kb", Equipment::Kettlebell),
    ("machine", Equipment::Machine),
    ("machines", Equipment::Machine),
    ("smith machine", Equipment::Machine),
    ("smith machines", Equipment::Machine),
    ("band", Equipment::ResistanceBand),
    ("bands", Equipment::ResistanceBand),
    ("resistance band", Equipment::ResistanceBand),
    ("resistance bands", Equipment::ResistanceBand),
    ("slider", Equipment::Sliders),
    ("sliders", Equipment::Sliders),
    ("trap bar", Equipment::TrapBar),
    ("trap bars", Equipment::TrapBar),
    ("hex bar", Equipment::TrapBar),
    ("hex bars", Equipment::TrapBar),
];

const HINGE_MUSCLES: &[(MuscleID, Stimulus)] = &[
    (MuscleID::Glutes, Stimulus::PRIMARY),
    (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
    (MuscleID::Hamstrings, Stimulus::PRIMARY),
    (MuscleID::Adductors, Stimulus::SECONDARY),
    (MuscleID::Traps, Stimulus::SECONDARY),
    (MuscleID::Forearms, Stimulus::SECONDARY),
];

// A name starts with the equipment that sets the exercise apart from its usual form, followed by
// any modifiers. Names without equipment denote the bodyweight form on the usual apparatus, or the
// form implied by an established name, such as the leg press or the goblet squat. Where the
// equipment is commonly left out of the name, the short name is an alias.
const EXERCISE_VARIANTS: [BaseExercise; 120] = [
    BaseExercise {
        name: "90-90 Hip Stretch",
        aliases: &["90/90 Hip Stretch"],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::HipRotators, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::SECONDARY),
            (MuscleID::Adductors, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[ExerciseVariant {
            name: "90-90 Hip Switch",
            muscles: Some(&[
                (MuscleID::HipRotators, Stimulus::SECONDARY),
                (MuscleID::HipFlexors, Stimulus::SECONDARY),
                (MuscleID::Glutes, Stimulus::SECONDARY),
            ]),
            force: Some(Force::Pull),
            laterality: Some(Laterality::Bilateral),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Arm Circle",
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::FrontDelts, Stimulus::SECONDARY),
            (MuscleID::RearDelts, Stimulus::SECONDARY),
            (MuscleID::Pecs, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Back Bridge",
        aliases: &["Wheel Pose"],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Glutes, Stimulus::SECONDARY),
            (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
        ],
        stretch: &[
            (MuscleID::HipFlexors, Stimulus::PRIMARY),
            (MuscleID::Abs, Stimulus::PRIMARY),
            (MuscleID::Lats, Stimulus::SECONDARY),
            (MuscleID::Pecs, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Back Extension",
        aliases: &["Hyperextension"],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::SECONDARY),
            (MuscleID::Hamstrings, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Band Pull Apart",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::ResistanceBand],
        muscles: &[
            (MuscleID::RearDelts, Stimulus::PRIMARY),
            (MuscleID::Lats, Stimulus::SECONDARY),
            (MuscleID::Traps, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Barbell Ab Rollout",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[(MuscleID::Abs, Stimulus::PRIMARY)],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Barbell Kneeling Ab Rollout",
            aliases: &["Kneeling Barbell Ab Rollout"],
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Barbell Bench Press",
        aliases: &["Bench Press"],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (MuscleID::Pecs, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::PRIMARY),
            (MuscleID::Triceps, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Close-Grip Bench Press",
                aliases: &["Close-Grip Bench Press"],
                muscles: Some(&[
                    (MuscleID::Pecs, Stimulus::PRIMARY),
                    (MuscleID::Triceps, Stimulus::PRIMARY),
                    (MuscleID::FrontDelts, Stimulus::SECONDARY),
                ]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Decline Bench Press",
                aliases: &["Decline Bench Press"],
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Incline Bench Press",
                aliases: &["Incline Bench Press"],
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
                aliases: &["Chest Press"],
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
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[(MuscleID::Biceps, Stimulus::PRIMARY)],
        stretch: &[],
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
                muscles: Some(&[
                    (MuscleID::Biceps, Stimulus::PRIMARY),
                    (MuscleID::Forearms, Stimulus::SECONDARY),
                ]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Curl",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Hammer Curl",
                aliases: &["Hammer Curl"],
                equipment: Some(&[Equipment::Dumbbell]),
                muscles: Some(&[
                    (MuscleID::Biceps, Stimulus::PRIMARY),
                    (MuscleID::Forearms, Stimulus::SECONDARY),
                ]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Incline Curl",
                equipment: Some(&[Equipment::Dumbbell]),
                stretch: Some(&[(MuscleID::Biceps, Stimulus::PRIMARY)]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Lying Curl",
                equipment: Some(&[Equipment::Dumbbell]),
                stretch: Some(&[(MuscleID::Biceps, Stimulus::PRIMARY)]),
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
        aliases: &["Deadlift"],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (MuscleID::Glutes, Stimulus::PRIMARY),
            (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
            (MuscleID::Quads, Stimulus::SECONDARY),
            (MuscleID::Hamstrings, Stimulus::SECONDARY),
            (MuscleID::Adductors, Stimulus::SECONDARY),
            (MuscleID::Traps, Stimulus::SECONDARY),
            (MuscleID::Forearms, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Deficit Deadlift",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Deficit Romanian Deadlift",
                muscles: Some(HINGE_MUSCLES),
                stretch: Some(&[(MuscleID::Hamstrings, Stimulus::PRIMARY)]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Deficit Stiff-Legged Deadlift",
                muscles: Some(HINGE_MUSCLES),
                stretch: Some(&[(MuscleID::Hamstrings, Stimulus::PRIMARY)]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Romanian Deadlift",
                aliases: &["Romanian Deadlift", "RDL"],
                muscles: Some(HINGE_MUSCLES),
                stretch: Some(&[(MuscleID::Hamstrings, Stimulus::PRIMARY)]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Stiff-Legged Deadlift",
                aliases: &["Stiff-Legged Deadlift", "Stiff-Leg Deadlift"],
                muscles: Some(HINGE_MUSCLES),
                stretch: Some(&[(MuscleID::Hamstrings, Stimulus::PRIMARY)]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Sumo Deadlift",
                aliases: &["Sumo Deadlift"],
                muscles: Some(&[
                    (MuscleID::Glutes, Stimulus::PRIMARY),
                    (MuscleID::Quads, Stimulus::PRIMARY),
                    (MuscleID::Adductors, Stimulus::PRIMARY),
                    (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
                    (MuscleID::Hamstrings, Stimulus::SECONDARY),
                    (MuscleID::Traps, Stimulus::SECONDARY),
                    (MuscleID::Forearms, Stimulus::SECONDARY),
                ]),
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
                muscles: Some(HINGE_MUSCLES),
                stretch: Some(&[(MuscleID::Hamstrings, Stimulus::PRIMARY)]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Deficit Stiff-Legged Deadlift",
                equipment: Some(&[Equipment::Dumbbell]),
                muscles: Some(HINGE_MUSCLES),
                stretch: Some(&[(MuscleID::Hamstrings, Stimulus::PRIMARY)]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Romanian Deadlift",
                equipment: Some(&[Equipment::Dumbbell]),
                muscles: Some(HINGE_MUSCLES),
                stretch: Some(&[(MuscleID::Hamstrings, Stimulus::PRIMARY)]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Stiff-Legged Deadlift",
                equipment: Some(&[Equipment::Dumbbell]),
                muscles: Some(HINGE_MUSCLES),
                stretch: Some(&[(MuscleID::Hamstrings, Stimulus::PRIMARY)]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Kettlebell Single-Leg Deadlift",
                aliases: &["Single-Leg Romanian Deadlift"],
                laterality: Some(Laterality::Unilateral),
                equipment: Some(&[Equipment::Kettlebell]),
                muscles: Some(HINGE_MUSCLES),
                stretch: Some(&[(MuscleID::Hamstrings, Stimulus::PRIMARY)]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Trap Bar Deadlift",
                aliases: &["Hex Bar Deadlift"],
                equipment: Some(&[Equipment::TrapBar]),
                muscles: Some(&[
                    (MuscleID::Glutes, Stimulus::PRIMARY),
                    (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
                    (MuscleID::Quads, Stimulus::PRIMARY),
                    (MuscleID::Hamstrings, Stimulus::SECONDARY),
                    (MuscleID::Adductors, Stimulus::SECONDARY),
                    (MuscleID::Traps, Stimulus::SECONDARY),
                    (MuscleID::Forearms, Stimulus::SECONDARY),
                ]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Barbell Floor Press",
        aliases: &["Floor Press"],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (MuscleID::Pecs, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::PRIMARY),
            (MuscleID::Triceps, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Dumbbell Floor Press",
            equipment: Some(&[Equipment::Dumbbell]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Barbell Good Morning",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (MuscleID::Glutes, Stimulus::PRIMARY),
            (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
            (MuscleID::Adductors, Stimulus::SECONDARY),
        ],
        stretch: &[(MuscleID::Hamstrings, Stimulus::PRIMARY)],
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
        aliases: &["Hip Thrust"],
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (MuscleID::Glutes, Stimulus::PRIMARY),
            (MuscleID::Quads, Stimulus::SECONDARY),
            (MuscleID::Adductors, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Machine Hip Thrust",
            equipment: Some(&[Equipment::Machine]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Barbell Overhead Triceps Extension",
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[(MuscleID::Triceps, Stimulus::PRIMARY)],
        stretch: &[(MuscleID::Triceps, Stimulus::PRIMARY)],
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
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::Pecs, Stimulus::PRIMARY),
        ],
        stretch: &[
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::Pecs, Stimulus::SECONDARY),
        ],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Dumbbell Pullover",
            aliases: &["Pullover"],
            equipment: Some(&[Equipment::Dumbbell]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Barbell Rear Delt Row",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (MuscleID::RearDelts, Stimulus::PRIMARY),
            (MuscleID::Traps, Stimulus::PRIMARY),
            (MuscleID::Biceps, Stimulus::SECONDARY),
            (MuscleID::Lats, Stimulus::SECONDARY),
            (MuscleID::Forearms, Stimulus::SECONDARY),
        ],
        stretch: &[],
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
        name: "Barbell Row",
        aliases: &["Bent-Over Row"],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::Traps, Stimulus::PRIMARY),
            (MuscleID::RearDelts, Stimulus::PRIMARY),
            (MuscleID::Biceps, Stimulus::SECONDARY),
            (MuscleID::Forearms, Stimulus::SECONDARY),
            (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Dumbbell One-Arm Row",
            aliases: &["One-Arm Dumbbell Row"],
            laterality: Some(Laterality::Unilateral),
            equipment: Some(&[Equipment::Dumbbell]),
            muscles: Some(&[
                (MuscleID::Lats, Stimulus::PRIMARY),
                (MuscleID::Traps, Stimulus::PRIMARY),
                (MuscleID::RearDelts, Stimulus::PRIMARY),
                (MuscleID::Biceps, Stimulus::SECONDARY),
                (MuscleID::Forearms, Stimulus::SECONDARY),
            ]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Barbell Shoulder Press",
        aliases: &["Overhead Press", "Military Press", "OHP"],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (MuscleID::FrontDelts, Stimulus::PRIMARY),
            (MuscleID::SideDelts, Stimulus::SECONDARY),
            (MuscleID::Triceps, Stimulus::SECONDARY),
        ],
        stretch: &[],
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
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (MuscleID::Traps, Stimulus::PRIMARY),
            (MuscleID::Forearms, Stimulus::SECONDARY),
        ],
        stretch: &[],
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
        aliases: &["Skull Crusher", "Lying Triceps Extension"],
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[(MuscleID::Triceps, Stimulus::PRIMARY)],
        stretch: &[],
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
        name: "Barbell Upright Row",
        aliases: &["Upright Row"],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (MuscleID::SideDelts, Stimulus::PRIMARY),
            (MuscleID::Traps, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::SECONDARY),
            (MuscleID::Biceps, Stimulus::SECONDARY),
            (MuscleID::Forearms, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Cable Upright Row",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Upright Row",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Box Jump",
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Box],
        muscles: &[
            (MuscleID::Quads, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::PRIMARY),
            (MuscleID::Adductors, Stimulus::PRIMARY),
            (MuscleID::Calves, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Plyometrics,
        variants: &[
            ExerciseVariant {
                name: "Lunge Jump",
                aliases: &["Jumping Lunge"],
                equipment: Some(&[]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Squat Jump",
                aliases: &["Jump Squat"],
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
        name: "Butterfly Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[(MuscleID::Adductors, Stimulus::PRIMARY)],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Cable Crossover",
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Cable],
        muscles: &[
            (MuscleID::Pecs, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::SECONDARY),
        ],
        stretch: &[(MuscleID::Pecs, Stimulus::SECONDARY)],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Cable Pallof Press",
        aliases: &["Pallof Press"],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Cable],
        muscles: &[(MuscleID::Abs, Stimulus::PRIMARY)],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Band Pallof Press",
            equipment: Some(&[Equipment::ResistanceBand]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Cable Rope Face Pull",
        aliases: &["Face Pull"],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Cable],
        muscles: &[
            (MuscleID::RearDelts, Stimulus::PRIMARY),
            (MuscleID::SideDelts, Stimulus::SECONDARY),
            (MuscleID::Traps, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Cable Row",
        aliases: &["Seated Cable Row", "Seated Row"],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Cable],
        muscles: &[
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::Traps, Stimulus::PRIMARY),
            (MuscleID::RearDelts, Stimulus::PRIMARY),
            (MuscleID::Biceps, Stimulus::SECONDARY),
            (MuscleID::Forearms, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Cable Flexion Row",
            muscles: Some(&[
                (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
                (MuscleID::Lats, Stimulus::PRIMARY),
                (MuscleID::Traps, Stimulus::PRIMARY),
                (MuscleID::RearDelts, Stimulus::PRIMARY),
                (MuscleID::Biceps, Stimulus::SECONDARY),
                (MuscleID::Forearms, Stimulus::SECONDARY),
            ]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Cable Straight-Arm Pulldown",
        aliases: &["Straight-Arm Pulldown"],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Cable],
        muscles: &[
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::Triceps, Stimulus::SECONDARY),
        ],
        stretch: &[(MuscleID::Lats, Stimulus::SECONDARY)],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Cable Triceps Pushdown",
        aliases: &["Triceps Pushdown"],
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Cable],
        muscles: &[(MuscleID::Triceps, Stimulus::PRIMARY)],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Cable Rope Triceps Pushdown",
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Camel Pose",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::HipFlexors, Stimulus::PRIMARY),
            (MuscleID::Abs, Stimulus::PRIMARY),
            (MuscleID::Quads, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Cat-Cow",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
            (MuscleID::Abs, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Child's Pose",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Cobra Stretch",
        aliases: &["Cobra Pose"],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Abs, Stimulus::PRIMARY),
            (MuscleID::HipFlexors, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Copenhagen Plank",
        aliases: &["Copenhagen Side Plank"],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Adductors, Stimulus::PRIMARY),
            (MuscleID::Abs, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Cossack Squat",
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Glutes, Stimulus::PRIMARY),
            (MuscleID::Quads, Stimulus::PRIMARY),
            (MuscleID::Adductors, Stimulus::SECONDARY),
            (MuscleID::Hamstrings, Stimulus::SECONDARY),
        ],
        stretch: &[
            (MuscleID::Adductors, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::SECONDARY),
        ],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Couch Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::HipFlexors, Stimulus::PRIMARY),
            (MuscleID::Quads, Stimulus::PRIMARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Cross-Body Shoulder Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[(MuscleID::RearDelts, Stimulus::PRIMARY)],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Crunch",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(MuscleID::Abs, Stimulus::PRIMARY)],
        stretch: &[],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Bicycle Crunch",
                ..ExerciseVariant::default()
            },
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
            ExerciseVariant {
                name: "Reverse Crunch",
                muscles: Some(&[
                    (MuscleID::Abs, Stimulus::PRIMARY),
                    (MuscleID::HipFlexors, Stimulus::SECONDARY),
                ]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Dead Bug",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(MuscleID::Abs, Stimulus::PRIMARY)],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Dead Hang",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::PullUpBar],
        muscles: &[(MuscleID::Forearms, Stimulus::SECONDARY)],
        stretch: &[
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::Pecs, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Deep Squat Hold",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Adductors, Stimulus::PRIMARY),
            (MuscleID::Calves, Stimulus::SECONDARY),
            (MuscleID::Glutes, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Dip",
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::ParallelBars],
        muscles: &[
            (MuscleID::Pecs, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::PRIMARY),
            (MuscleID::Triceps, Stimulus::PRIMARY),
        ],
        stretch: &[
            (MuscleID::Pecs, Stimulus::SECONDARY),
            (MuscleID::FrontDelts, Stimulus::SECONDARY),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Bench Dip",
                equipment: Some(&[]),
                muscles: Some(&[
                    (MuscleID::Triceps, Stimulus::PRIMARY),
                    (MuscleID::Pecs, Stimulus::SECONDARY),
                    (MuscleID::FrontDelts, Stimulus::SECONDARY),
                ]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Machine Dip",
                equipment: Some(&[Equipment::Machine]),
                muscles: Some(&[
                    (MuscleID::Triceps, Stimulus::PRIMARY),
                    (MuscleID::Pecs, Stimulus::SECONDARY),
                    (MuscleID::FrontDelts, Stimulus::SECONDARY),
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
        name: "Doorway Chest Stretch",
        aliases: &["Doorway Pec Stretch"],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Pecs, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[ExerciseVariant {
            name: "One-Arm Doorway Chest Stretch",
            laterality: Some(Laterality::Unilateral),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Downward Dog",
        aliases: &["Downward-Facing Dog"],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
            (MuscleID::Calves, Stimulus::PRIMARY),
            (MuscleID::Lats, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Dumbbell Farmer's Walk",
        aliases: &["Farmer's Walk", "Farmer's Carry"],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[
            (MuscleID::Forearms, Stimulus::PRIMARY),
            (MuscleID::Traps, Stimulus::PRIMARY),
            (MuscleID::Abs, Stimulus::SECONDARY),
            (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Kettlebell Farmer's Walk",
                equipment: Some(&[Equipment::Kettlebell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Trap Bar Farmer's Walk",
                equipment: Some(&[Equipment::TrapBar]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Dumbbell Fly",
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[
            (MuscleID::Pecs, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::SECONDARY),
        ],
        stretch: &[(MuscleID::Pecs, Stimulus::PRIMARY)],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Cable Fly",
                equipment: Some(&[Equipment::Cable]),
                stretch: Some(&[(MuscleID::Pecs, Stimulus::SECONDARY)]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Machine Fly",
                aliases: &["Pec Deck"],
                equipment: Some(&[Equipment::Machine]),
                stretch: Some(&[(MuscleID::Pecs, Stimulus::SECONDARY)]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Dumbbell Front Raise",
        aliases: &["Front Raise"],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[
            (MuscleID::FrontDelts, Stimulus::PRIMARY),
            (MuscleID::SideDelts, Stimulus::SECONDARY),
            (MuscleID::Pecs, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Front Raise",
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Cable Front Raise",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Dumbbell Incline Row",
        aliases: &["Chest-Supported Row"],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[
            (MuscleID::Traps, Stimulus::PRIMARY),
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::RearDelts, Stimulus::PRIMARY),
            (MuscleID::Biceps, Stimulus::SECONDARY),
            (MuscleID::Forearms, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Dumbbell Lateral Raise",
        aliases: &["Lateral Raise"],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[
            (MuscleID::SideDelts, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::SECONDARY),
        ],
        stretch: &[],
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
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[
            (MuscleID::RearDelts, Stimulus::PRIMARY),
            (MuscleID::Traps, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Cable Reverse Fly",
                equipment: Some(&[Equipment::Cable]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Machine Reverse Fly",
                aliases: &["Reverse Pec Deck"],
                equipment: Some(&[Equipment::Machine]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Dumbbell Reverse Wrist Curl",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[(MuscleID::Forearms, Stimulus::PRIMARY)],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Cable Reverse Wrist Curl",
            equipment: Some(&[Equipment::Cable]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Dumbbell Wrist Curl",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[(MuscleID::Forearms, Stimulus::PRIMARY)],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Cable Wrist Curl",
            equipment: Some(&[Equipment::Cable]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Figure-Four Stretch",
        aliases: &["Figure 4 Stretch"],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::HipRotators, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Frog Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[(MuscleID::Adductors, Stimulus::PRIMARY)],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Front Split",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::HipFlexors, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
            (MuscleID::Quads, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "German Hang",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::GymnasticRings],
        muscles: &[],
        stretch: &[
            (MuscleID::Pecs, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::PRIMARY),
            (MuscleID::Biceps, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Glute Bridge",
        aliases: &["Hip Bridge"],
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Glutes, Stimulus::PRIMARY),
            (MuscleID::Quads, Stimulus::SECONDARY),
            (MuscleID::Adductors, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Single-Leg Glute Bridge",
            laterality: Some(Laterality::Unilateral),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Half Split",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
            (MuscleID::Calves, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Handstand Push Up",
        aliases: &["HSPU"],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::FrontDelts, Stimulus::PRIMARY),
            (MuscleID::SideDelts, Stimulus::SECONDARY),
            (MuscleID::Triceps, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Pike Push Up",
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Hanging Leg Raise",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::PullUpBar],
        muscles: &[
            (MuscleID::Abs, Stimulus::PRIMARY),
            (MuscleID::HipFlexors, Stimulus::PRIMARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Hanging Knee Raise",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Lying Leg Raise",
                equipment: Some(&[]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Hollow Body Hold",
        aliases: &["Hollow Hold"],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Abs, Stimulus::PRIMARY),
            (MuscleID::HipFlexors, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Horse Stance",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Quads, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::SECONDARY),
        ],
        stretch: &[(MuscleID::Adductors, Stimulus::PRIMARY)],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Inverted Row",
        aliases: &["Australian Pull Up", "Body Row"],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::Traps, Stimulus::PRIMARY),
            (MuscleID::RearDelts, Stimulus::PRIMARY),
            (MuscleID::Biceps, Stimulus::SECONDARY),
            (MuscleID::Forearms, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Ring Row",
            equipment: Some(&[Equipment::GymnasticRings]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Jefferson Curl",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Barbell],
        muscles: &[
            (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::SECONDARY),
        ],
        stretch: &[
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
            (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Kettlebell Swing",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Kettlebell],
        muscles: &[
            (MuscleID::Glutes, Stimulus::PRIMARY),
            (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::SECONDARY),
            (MuscleID::Adductors, Stimulus::SECONDARY),
            (MuscleID::Traps, Stimulus::SECONDARY),
            (MuscleID::Forearms, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Kneeling Hip Flexor Stretch",
        aliases: &["Half-Kneeling Hip Flexor Stretch"],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[(MuscleID::HipFlexors, Stimulus::PRIMARY)],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Kneeling Lat Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::Triceps, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "L-Sit",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::ParallelBars],
        muscles: &[
            (MuscleID::Abs, Stimulus::PRIMARY),
            (MuscleID::HipFlexors, Stimulus::PRIMARY),
            (MuscleID::Quads, Stimulus::SECONDARY),
            (MuscleID::Triceps, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Lat Pulldown",
        aliases: &["Lat Pull Down"],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::Traps, Stimulus::SECONDARY),
            (MuscleID::Biceps, Stimulus::SECONDARY),
            (MuscleID::RearDelts, Stimulus::SECONDARY),
            (MuscleID::Forearms, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Leg Extension",
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[(MuscleID::Quads, Stimulus::PRIMARY)],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Leg Press",
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (MuscleID::Quads, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::PRIMARY),
            (MuscleID::Adductors, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Leg Swing",
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(MuscleID::HipFlexors, Stimulus::SECONDARY)],
        stretch: &[
            (MuscleID::Hamstrings, Stimulus::SECONDARY),
            (MuscleID::HipFlexors, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[ExerciseVariant {
            name: "Lateral Leg Swing",
            muscles: Some(&[(MuscleID::Abductors, Stimulus::SECONDARY)]),
            stretch: Some(&[
                (MuscleID::Adductors, Stimulus::SECONDARY),
                (MuscleID::Abductors, Stimulus::SECONDARY),
            ]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Lizard Stretch",
        aliases: &["Lizard Pose"],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::HipFlexors, Stimulus::PRIMARY),
            (MuscleID::Adductors, Stimulus::SECONDARY),
            (MuscleID::Glutes, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Lunge",
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Quads, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::PRIMARY),
            (MuscleID::Adductors, Stimulus::PRIMARY),
        ],
        stretch: &[(MuscleID::HipFlexors, Stimulus::SECONDARY)],
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
                stretch: Some(&[
                    (MuscleID::Adductors, Stimulus::PRIMARY),
                    (MuscleID::Hamstrings, Stimulus::SECONDARY),
                ]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Walking Lunge",
                laterality: Some(Laterality::Bilateral),
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Reverse Lunge",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Side Lunge",
                aliases: &["Lateral Lunge"],
                stretch: Some(&[
                    (MuscleID::Adductors, Stimulus::PRIMARY),
                    (MuscleID::Hamstrings, Stimulus::SECONDARY),
                ]),
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
        aliases: &["Hack Squat"],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (MuscleID::Quads, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::SECONDARY),
            (MuscleID::Adductors, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Barbell Hack Squat",
            equipment: Some(&[Equipment::Barbell]),
            muscles: Some(&[
                (MuscleID::Quads, Stimulus::PRIMARY),
                (MuscleID::Glutes, Stimulus::SECONDARY),
                (MuscleID::Adductors, Stimulus::SECONDARY),
                (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
                (MuscleID::Traps, Stimulus::SECONDARY),
                (MuscleID::Forearms, Stimulus::SECONDARY),
                (MuscleID::Calves, Stimulus::SECONDARY),
            ]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Machine Hip Abduction",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (MuscleID::Abductors, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::SECONDARY),
        ],
        stretch: &[],
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
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[(MuscleID::Adductors, Stimulus::PRIMARY)],
        stretch: &[],
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
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[(MuscleID::Calves, Stimulus::PRIMARY)],
        stretch: &[(MuscleID::Calves, Stimulus::PRIMARY)],
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
                aliases: &["Seated Calf Raise"],
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Machine-Assisted Dip",
        aliases: &["Assisted Dip"],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Assisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (MuscleID::Pecs, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::PRIMARY),
            (MuscleID::Triceps, Stimulus::PRIMARY),
        ],
        stretch: &[
            (MuscleID::Pecs, Stimulus::SECONDARY),
            (MuscleID::FrontDelts, Stimulus::SECONDARY),
        ],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Band-Assisted Dip",
                equipment: Some(&[Equipment::ResistanceBand, Equipment::ParallelBars]),
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
        aliases: &["Assisted Pull Up"],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Assisted,
        equipment: &[Equipment::Machine],
        muscles: &[
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::Traps, Stimulus::SECONDARY),
            (MuscleID::RearDelts, Stimulus::SECONDARY),
            (MuscleID::Biceps, Stimulus::SECONDARY),
            (MuscleID::Forearms, Stimulus::SECONDARY),
        ],
        stretch: &[],
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
        name: "Neck Flexion Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Neck, Stimulus::PRIMARY),
            (MuscleID::Traps, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Neck Side Stretch",
        aliases: &["Lateral Neck Stretch"],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Neck, Stimulus::PRIMARY),
            (MuscleID::Traps, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Nordic Curl",
        aliases: &["Nordic Hamstring Curl"],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(MuscleID::Hamstrings, Stimulus::PRIMARY)],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Overhead Triceps Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Triceps, Stimulus::PRIMARY),
            (MuscleID::Lats, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Pancake Good Morning",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(MuscleID::ErectorSpinae, Stimulus::SECONDARY)],
        stretch: &[
            (MuscleID::Adductors, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
        ],
        category: Category::Flexibility,
        variants: &[
            ExerciseVariant {
                name: "Barbell Pancake Good Morning",
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Standing Straddle Good Morning",
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Straddle Romanian Deadlift",
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Straddle Romanian Deadlift",
                equipment: Some(&[Equipment::Dumbbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Standing Straddle Good Morning",
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Pancake Lift",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::HipFlexors, Stimulus::PRIMARY),
            (MuscleID::Abs, Stimulus::SECONDARY),
            (MuscleID::Abductors, Stimulus::SECONDARY),
        ],
        stretch: &[
            (MuscleID::Adductors, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
        ],
        category: Category::Flexibility,
        variants: &[ExerciseVariant {
            name: "Single-Leg Pancake Lift",
            laterality: Some(Laterality::Unilateral),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Pancake Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Adductors, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
            (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Pigeon Stretch",
        aliases: &["Pigeon Pose"],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::HipRotators, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::SECONDARY),
            (MuscleID::HipFlexors, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Pike Good Morning",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(MuscleID::ErectorSpinae, Stimulus::SECONDARY)],
        stretch: &[(MuscleID::Hamstrings, Stimulus::PRIMARY)],
        category: Category::Flexibility,
        variants: &[ExerciseVariant {
            name: "Barbell Pike Good Morning",
            equipment: Some(&[Equipment::Barbell]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Pike Lift",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::HipFlexors, Stimulus::PRIMARY),
            (MuscleID::Quads, Stimulus::SECONDARY),
            (MuscleID::Abs, Stimulus::SECONDARY),
        ],
        stretch: &[(MuscleID::Hamstrings, Stimulus::PRIMARY)],
        category: Category::Flexibility,
        variants: &[ExerciseVariant {
            name: "Single-Leg Pike Lift",
            laterality: Some(Laterality::Unilateral),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Pike Stretch",
        aliases: &["Seated Forward Fold"],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
            (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[ExerciseVariant {
            name: "Single-Leg Pike Stretch",
            laterality: Some(Laterality::Unilateral),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Plank",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(MuscleID::Abs, Stimulus::PRIMARY)],
        stretch: &[],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Side Plank",
            laterality: Some(Laterality::Unilateral),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Pull Up",
        aliases: &["Pullup"],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::PullUpBar],
        muscles: &[
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::Traps, Stimulus::SECONDARY),
            (MuscleID::RearDelts, Stimulus::SECONDARY),
            (MuscleID::Biceps, Stimulus::SECONDARY),
            (MuscleID::Forearms, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Chin Up",
                aliases: &["Chinup"],
                muscles: Some(&[
                    (MuscleID::Lats, Stimulus::PRIMARY),
                    (MuscleID::Biceps, Stimulus::PRIMARY),
                    (MuscleID::Traps, Stimulus::SECONDARY),
                    (MuscleID::RearDelts, Stimulus::SECONDARY),
                    (MuscleID::Forearms, Stimulus::SECONDARY),
                ]),
                ..ExerciseVariant::default()
            },
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
        name: "Puppy Pose",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Lats, Stimulus::PRIMARY),
            (MuscleID::Pecs, Stimulus::SECONDARY),
            (MuscleID::Triceps, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Push Up",
        aliases: &["Press Up", "Pushup"],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Pecs, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::PRIMARY),
            (MuscleID::Triceps, Stimulus::SECONDARY),
            (MuscleID::Abs, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Archer Push Up",
                laterality: Some(Laterality::Unilateral),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Decline Push Up",
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Diamond Push Up",
                muscles: Some(&[
                    (MuscleID::Pecs, Stimulus::PRIMARY),
                    (MuscleID::Triceps, Stimulus::PRIMARY),
                    (MuscleID::FrontDelts, Stimulus::SECONDARY),
                    (MuscleID::Abs, Stimulus::SECONDARY),
                ]),
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
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[(MuscleID::Quads, Stimulus::PRIMARY)],
        stretch: &[
            (MuscleID::Quads, Stimulus::PRIMARY),
            (MuscleID::HipFlexors, Stimulus::PRIMARY),
        ],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Reverse Plank",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Glutes, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::SECONDARY),
            (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Reverse Tabletop",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Glutes, Stimulus::SECONDARY),
            (MuscleID::Triceps, Stimulus::SECONDARY),
        ],
        stretch: &[
            (MuscleID::FrontDelts, Stimulus::PRIMARY),
            (MuscleID::Pecs, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Seated Leg Curl",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Machine],
        muscles: &[(MuscleID::Hamstrings, Stimulus::PRIMARY)],
        stretch: &[(MuscleID::Hamstrings, Stimulus::PRIMARY)],
        category: Category::Strength,
        variants: &[ExerciseVariant {
            name: "Lying Leg Curl",
            stretch: Some(&[]),
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Seated Spinal Twist",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::SECONDARY),
            (MuscleID::Abs, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Shoulder Dislocate",
        aliases: &["Shoulder Pass-Through"],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::ResistanceBand],
        muscles: &[],
        stretch: &[
            (MuscleID::Pecs, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::PRIMARY),
            (MuscleID::Lats, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Side Split",
        aliases: &["Middle Split", "Straddle Split"],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Adductors, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Sit Up",
        aliases: &["Situp"],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Abs, Stimulus::PRIMARY),
            (MuscleID::HipFlexors, Stimulus::PRIMARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Skin the Cat",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::GymnasticRings],
        muscles: &[
            (MuscleID::Lats, Stimulus::SECONDARY),
            (MuscleID::Abs, Stimulus::SECONDARY),
        ],
        stretch: &[
            (MuscleID::Pecs, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::PRIMARY),
            (MuscleID::Biceps, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Sleeper Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[(MuscleID::RearDelts, Stimulus::PRIMARY)],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Slider Hamstring Curl",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Sliders],
        muscles: &[
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::SECONDARY),
        ],
        stretch: &[],
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
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Quads, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::PRIMARY),
            (MuscleID::Adductors, Stimulus::PRIMARY),
        ],
        stretch: &[(MuscleID::HipFlexors, Stimulus::SECONDARY)],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Bulgarian Split Squat",
                equipment: Some(&[Equipment::Barbell]),
                stretch: Some(&[
                    (MuscleID::HipFlexors, Stimulus::PRIMARY),
                    (MuscleID::Quads, Stimulus::SECONDARY),
                ]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Split Squat",
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Bulgarian Split Squat",
                aliases: &["Rear-Foot-Elevated Split Squat", "RFESS"],
                stretch: Some(&[
                    (MuscleID::HipFlexors, Stimulus::PRIMARY),
                    (MuscleID::Quads, Stimulus::SECONDARY),
                ]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Bulgarian Split Squat",
                equipment: Some(&[Equipment::Dumbbell]),
                stretch: Some(&[
                    (MuscleID::HipFlexors, Stimulus::PRIMARY),
                    (MuscleID::Quads, Stimulus::SECONDARY),
                ]),
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
                stretch: Some(&[
                    (MuscleID::HipFlexors, Stimulus::PRIMARY),
                    (MuscleID::Quads, Stimulus::SECONDARY),
                ]),
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
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Quads, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::PRIMARY),
            (MuscleID::Adductors, Stimulus::PRIMARY),
            (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
            (MuscleID::Calves, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Front Squat",
                aliases: &["Front Squat"],
                equipment: Some(&[Equipment::Barbell]),
                muscles: Some(&[
                    (MuscleID::Quads, Stimulus::PRIMARY),
                    (MuscleID::Glutes, Stimulus::PRIMARY),
                    (MuscleID::Adductors, Stimulus::PRIMARY),
                    (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
                    (MuscleID::Calves, Stimulus::SECONDARY),
                    (MuscleID::Abs, Stimulus::SECONDARY),
                ]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Overhead Squat",
                aliases: &["Overhead Squat"],
                equipment: Some(&[Equipment::Barbell]),
                muscles: Some(&[
                    (MuscleID::Quads, Stimulus::PRIMARY),
                    (MuscleID::Glutes, Stimulus::PRIMARY),
                    (MuscleID::Adductors, Stimulus::PRIMARY),
                    (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
                    (MuscleID::Calves, Stimulus::SECONDARY),
                    (MuscleID::FrontDelts, Stimulus::SECONDARY),
                    (MuscleID::SideDelts, Stimulus::SECONDARY),
                    (MuscleID::Traps, Stimulus::SECONDARY),
                    (MuscleID::Abs, Stimulus::SECONDARY),
                ]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Squat",
                aliases: &["Back Squat"],
                equipment: Some(&[Equipment::Barbell]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Barbell Zercher Squat",
                aliases: &["Zercher Squat"],
                equipment: Some(&[Equipment::Barbell]),
                muscles: Some(&[
                    (MuscleID::Quads, Stimulus::PRIMARY),
                    (MuscleID::Glutes, Stimulus::PRIMARY),
                    (MuscleID::Adductors, Stimulus::PRIMARY),
                    (MuscleID::ErectorSpinae, Stimulus::PRIMARY),
                    (MuscleID::Calves, Stimulus::SECONDARY),
                    (MuscleID::Biceps, Stimulus::SECONDARY),
                    (MuscleID::Abs, Stimulus::SECONDARY),
                ]),
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
        name: "Standing Hamstring Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
            (MuscleID::Calves, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Standing Pike Stretch",
        aliases: &["Forward Fold"],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
            (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
            (MuscleID::Calves, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Standing Quad Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[(MuscleID::Quads, Stimulus::PRIMARY)],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Standing Straddle Hold",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Adductors, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
            (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Step Up",
        aliases: &["Box Step Up"],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Box],
        muscles: &[
            (MuscleID::Quads, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::PRIMARY),
            (MuscleID::Adductors, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[
            ExerciseVariant {
                name: "Barbell Step Up",
                equipment: Some(&[Equipment::Barbell, Equipment::Box]),
                ..ExerciseVariant::default()
            },
            ExerciseVariant {
                name: "Dumbbell Step Up",
                equipment: Some(&[Equipment::Dumbbell, Equipment::Box]),
                ..ExerciseVariant::default()
            },
        ],
    },
    BaseExercise {
        name: "Thread the Needle",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::RearDelts, Stimulus::PRIMARY),
            (MuscleID::Lats, Stimulus::SECONDARY),
            (MuscleID::Traps, Stimulus::SECONDARY),
            (MuscleID::ErectorSpinae, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Wall Biceps Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::Biceps, Stimulus::PRIMARY),
            (MuscleID::FrontDelts, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Wall Calf Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[(MuscleID::Calves, Stimulus::PRIMARY)],
        category: Category::Flexibility,
        variants: &[ExerciseVariant {
            name: "Bent-Knee Wall Calf Stretch",
            aliases: &["Knee-to-Wall Stretch"],
            ..ExerciseVariant::default()
        }],
    },
    BaseExercise {
        name: "Wall Sit",
        aliases: &["Wall Squat"],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[
            (MuscleID::Quads, Stimulus::PRIMARY),
            (MuscleID::Glutes, Stimulus::SECONDARY),
        ],
        stretch: &[],
        category: Category::Strength,
        variants: &[],
    },
    BaseExercise {
        name: "Weighted Pancake",
        aliases: &[],
        force: Force::Pull,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Bilateral,
        assistance: Assistance::Unassisted,
        equipment: &[Equipment::Dumbbell],
        muscles: &[(MuscleID::ErectorSpinae, Stimulus::SECONDARY)],
        stretch: &[
            (MuscleID::Adductors, Stimulus::PRIMARY),
            (MuscleID::Hamstrings, Stimulus::PRIMARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "World's Greatest Stretch",
        aliases: &[],
        force: Force::Push,
        mechanic: Mechanic::Compound,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[
            (MuscleID::HipFlexors, Stimulus::SECONDARY),
            (MuscleID::Hamstrings, Stimulus::SECONDARY),
            (MuscleID::Adductors, Stimulus::SECONDARY),
        ],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Wrist Extensor Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[(MuscleID::Forearms, Stimulus::PRIMARY)],
        category: Category::Flexibility,
        variants: &[],
    },
    BaseExercise {
        name: "Wrist Flexor Stretch",
        aliases: &[],
        force: Force::Static,
        mechanic: Mechanic::Isolation,
        laterality: Laterality::Unilateral,
        assistance: Assistance::Unassisted,
        equipment: &[],
        muscles: &[],
        stretch: &[(MuscleID::Forearms, Stimulus::PRIMARY)],
        category: Category::Flexibility,
        variants: &[],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashSet;

    use crate::exercise::normalized_name;

    #[test]
    fn test_exercise_variant() {
        assert_eq!(
            ExerciseVariant::default(),
            ExerciseVariant {
                name: "",
                aliases: &[],
                muscles: None,
                stretch: None,
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
    fn test_exercise() {
        assert!(exercise(&Name::new("Arm Circle").unwrap()).is_some());
        assert!(exercise(&Name::new("Unknown").unwrap()).is_none());
    }

    #[test]
    fn test_exercises() {
        for (name, exercise) in EXERCISES.iter() {
            assert_eq!(*name, exercise.name);

            // The mechanic of a stretch counts the joints held at end range, not its muscles.
            if exercise.category == Category::Flexibility {
                assert!(!exercise.stretch.is_empty(), "{name}");
            } else {
                assert!(
                    match exercise.mechanic {
                        Mechanic::Isolation =>
                            exercise
                                .muscles
                                .iter()
                                .filter(|(_, s)| *s == Stimulus::PRIMARY)
                                .collect::<Vec<_>>()
                                .len()
                                == 1,
                        Mechanic::Compound =>
                            !exercise
                                .muscles
                                .iter()
                                .filter(|(_, s)| *s == Stimulus::PRIMARY)
                                .collect::<Vec<_>>()
                                .is_empty()
                                && exercise.muscles.len() > 1,
                    },
                    "{name}"
                );
            }

            if name.as_ref().contains("One-") || name.as_ref().contains("Single-Leg") {
                assert!(exercise.laterality == Laterality::Unilateral);
            }

            assert!(match exercise.assistance {
                Assistance::Assisted => name.as_ref().contains("Assisted"),
                Assistance::Unassisted => !name.as_ref().contains("Assisted"),
            });
            if name.as_ref().contains("Barbell") {
                assert!(exercise.equipment.contains(&Equipment::Barbell));
            }
            if name.as_ref().contains("Box") {
                assert!(exercise.equipment.contains(&Equipment::Box));
            }
            if name.as_ref().contains("Cable") {
                assert!(exercise.equipment.contains(&Equipment::Cable));
            }
            if name.as_ref().contains("Dumbbell") {
                assert!(exercise.equipment.contains(&Equipment::Dumbbell));
            }
            if name.as_ref().contains("Ball") {
                assert!(exercise.equipment.contains(&Equipment::ExerciseBall));
            }
            if name.as_ref().contains("Ring") {
                assert!(exercise.equipment.contains(&Equipment::GymnasticRings));
            }
            if name.as_ref().contains("Kettlebell") {
                assert!(exercise.equipment.contains(&Equipment::Kettlebell));
            }
            if name.as_ref().contains("Machine") {
                assert!(exercise.equipment.contains(&Equipment::Machine));
            }
            if name.as_ref().contains("Band") {
                assert!(exercise.equipment.contains(&Equipment::ResistanceBand));
            }
            if name.as_ref().contains("Slider") {
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
    fn test_exercises_distinct_normalized_names_and_aliases() {
        let mut names = HashSet::new();

        for exercise in EXERCISES.values() {
            for name in std::iter::once(exercise.name.as_ref().as_str())
                .chain(exercise.aliases.iter().copied())
            {
                assert!(
                    names.insert(normalized_name(name)),
                    "name or alias \"{name}\" not distinct"
                );
            }
        }
    }

    #[test]
    fn test_exercise_variants_duplicate_muscles() {
        for exercise in EXERCISE_VARIANTS {
            for muscles in [exercise.muscles, exercise.stretch] {
                assert_distinct_muscles(muscles, exercise.name);
            }

            for variant in exercise.variants {
                for muscles in [variant.muscles, variant.stretch] {
                    assert_distinct_muscles(muscles.unwrap_or_default(), variant.name);
                }
            }
        }
    }

    fn assert_distinct_muscles(muscles: &[(MuscleID, Stimulus)], name: &str) {
        let distinct: HashSet<MuscleID> = muscles.iter().map(|(m, _)| m).copied().collect();
        assert_eq!(
            muscles.len(),
            distinct.len(),
            "duplicate muscle entries for \"{name}\""
        );
    }
}
