use std::{collections::HashSet, slice::Iter};

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum Muscle {
    // Neck
    Neck = 1,
    // Chest
    Pecs = 11,
    // Back
    Traps = 21,
    Lats = 22,
    // Shoulders
    FrontDelts = 31,
    SideDelts = 32,
    RearDelts = 33,
    // Upper arms
    Biceps = 41,
    Triceps = 42,
    // Forearms
    Forearms = 51,
    // Waist
    Abs = 61,
    ErectorSpinae = 62,
    // Hips
    Glutes = 71,
    Abductors = 72,
    // Thighs
    Quads = 81,
    Hamstrings = 82,
    Adductors = 83,
    // Calves
    Calves = 91,
}

impl Muscle {
    pub fn iter() -> Iter<'static, Muscle> {
        static MUSCLES: [Muscle; 18] = [
            Muscle::Neck,
            Muscle::Pecs,
            Muscle::Traps,
            Muscle::Lats,
            Muscle::FrontDelts,
            Muscle::SideDelts,
            Muscle::RearDelts,
            Muscle::Biceps,
            Muscle::Triceps,
            Muscle::Forearms,
            Muscle::Abs,
            Muscle::ErectorSpinae,
            Muscle::Glutes,
            Muscle::Abductors,
            Muscle::Quads,
            Muscle::Hamstrings,
            Muscle::Adductors,
            Muscle::Calves,
        ];
        MUSCLES.iter()
    }

    pub fn id(muscle: Muscle) -> u8 {
        muscle as u8
    }

    pub fn from_repr(repr: u8) -> Option<Muscle> {
        match repr {
            x if x == Muscle::Neck as u8 => Some(Muscle::Neck),
            x if x == Muscle::Pecs as u8 => Some(Muscle::Pecs),
            x if x == Muscle::Traps as u8 => Some(Muscle::Traps),
            x if x == Muscle::Lats as u8 => Some(Muscle::Lats),
            x if x == Muscle::FrontDelts as u8 => Some(Muscle::FrontDelts),
            x if x == Muscle::SideDelts as u8 => Some(Muscle::SideDelts),
            x if x == Muscle::RearDelts as u8 => Some(Muscle::RearDelts),
            x if x == Muscle::Biceps as u8 => Some(Muscle::Biceps),
            x if x == Muscle::Triceps as u8 => Some(Muscle::Triceps),
            x if x == Muscle::Forearms as u8 => Some(Muscle::Forearms),
            x if x == Muscle::Abs as u8 => Some(Muscle::Abs),
            x if x == Muscle::ErectorSpinae as u8 => Some(Muscle::ErectorSpinae),
            x if x == Muscle::Glutes as u8 => Some(Muscle::Glutes),
            x if x == Muscle::Abductors as u8 => Some(Muscle::Abductors),
            x if x == Muscle::Quads as u8 => Some(Muscle::Quads),
            x if x == Muscle::Hamstrings as u8 => Some(Muscle::Hamstrings),
            x if x == Muscle::Adductors as u8 => Some(Muscle::Adductors),
            x if x == Muscle::Calves as u8 => Some(Muscle::Calves),
            _ => None,
        }
    }

    pub fn name(muscle: Muscle) -> &'static str {
        match muscle {
            Muscle::Neck => "Neck",
            Muscle::Pecs => "Pecs",
            Muscle::Traps => "Traps",
            Muscle::Lats => "Lats",
            Muscle::FrontDelts => "Front Delts",
            Muscle::SideDelts => "Side Delts",
            Muscle::RearDelts => "Rear Delts",
            Muscle::Biceps => "Biceps",
            Muscle::Triceps => "Triceps",
            Muscle::Forearms => "Forearms",
            Muscle::Abs => "Abs",
            Muscle::ErectorSpinae => "Erector Spinae",
            Muscle::Glutes => "Glutes",
            Muscle::Abductors => "Abductors",
            Muscle::Quads => "Quads",
            Muscle::Hamstrings => "Hamstrings",
            Muscle::Adductors => "Adductors",
            Muscle::Calves => "Calves",
        }
    }

    pub fn description(muscle: Muscle) -> &'static str {
        #[allow(clippy::match_same_arms)]
        match muscle {
            Muscle::Neck => "",
            Muscle::Pecs => "Chest",
            Muscle::Traps => "Upper back",
            Muscle::Lats => "Sides of back",
            Muscle::FrontDelts => "Anterior shoulders",
            Muscle::SideDelts => "Mid shoulders",
            Muscle::RearDelts => "Posterior shoulders",
            Muscle::Biceps => "Front of upper arms",
            Muscle::Triceps => "Back of upper arms",
            Muscle::Forearms => "",
            Muscle::Abs => "Belly",
            Muscle::ErectorSpinae => "Lower back and spine",
            Muscle::Glutes => "Buttocks",
            Muscle::Abductors => "Outside of hips",
            Muscle::Quads => "Front of thighs",
            Muscle::Hamstrings => "Back of thighs",
            Muscle::Adductors => "Inner thighs",
            Muscle::Calves => "Back of lower legs",
        }
    }
}

#[derive(Default, PartialEq)]
pub struct ExerciseFilter {
    pub muscles: HashSet<Muscle>,
}

impl ExerciseFilter {
    pub fn is_empty(&self) -> bool {
        self.muscles.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn test_muscle_id() {
        for muscle in Muscle::iter() {
            assert_eq!(Muscle::from_repr(Muscle::id(*muscle)).unwrap(), *muscle);
        }

        assert_eq!(Muscle::from_repr(u8::MAX), None);
    }

    #[test]
    fn test_muscle_name() {
        let mut names = HashSet::new();

        for muscle in Muscle::iter() {
            let name = Muscle::name(*muscle);

            assert!(!name.is_empty());
            assert!(!names.contains(name));

            names.insert(name);
        }
    }

    #[test]
    fn test_muscle_description() {
        let mut descriptions = HashSet::new();

        for muscle in Muscle::iter() {
            let description = Muscle::description(*muscle);

            assert!(description.is_empty() || !descriptions.contains(description));

            descriptions.insert(description);
        }
    }

    #[test]
    fn test_exercise_filter_is_empty() {
        assert!(ExerciseFilter {
            muscles: HashSet::new()
        }
        .is_empty());
    }
}
