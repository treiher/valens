//! Text form of routines and the schedule.

use std::fmt::Write as _;

use crate::{
    Exercise, Routine, RoutineID, RoutinePart, Schedule, ScheduleSlot, Sides, Tempo, Weekday,
    training::values_to_string,
};

const SEPARATOR: &str = " \u{2014} ";
/// The separator and the forms of it typed without the em dash.
const SEPARATORS: [&str; 3] = [SEPARATOR, " \u{2013} ", " - "];
const ROUTINE: &str = "Routine";
const ROTATION: &str = "Rotation";
const SCHEDULE: &str = "Schedule";

impl Routine {
    /// Writes the routine as text, naming each exercise as in `exercises`.
    #[must_use]
    pub fn to_text(&self, exercises: &[Exercise]) -> String {
        routine_lines(self, exercises).join("\n")
    }
}

impl Schedule {
    /// Writes the schedule as text, followed by the text of each routine it uses, in the order of
    /// their first use.
    ///
    /// Days without a slot are left out. A routine missing from `routines` is written as a
    /// placeholder and has no text of its own.
    #[must_use]
    pub fn to_text(&self, routines: &[Routine], exercises: &[Exercise]) -> String {
        let mut used = vec![];
        let mut reference = |routine_id: RoutineID| {
            if !used.contains(&routine_id) {
                used.push(routine_id);
            }
            routine_reference(routine_id, routines)
        };

        let mut week = vec![];
        for weekday in Weekday::iter() {
            let Some(slots) = self.entries().get(weekday).filter(|s| !s.is_empty()) else {
                continue;
            };
            week.push(weekday.to_string());
            for slot in slots {
                match slot {
                    ScheduleSlot::Routine(routine_id) => {
                        week.push(format!("- {}", reference(*routine_id)));
                    }
                    ScheduleSlot::Rotation(rotation_id) => {
                        if let Some(rotation) = self.rotations().get(rotation_id) {
                            week.push(format!("- {ROTATION}{SEPARATOR}{}", rotation.name));
                        }
                    }
                }
            }
        }

        let mut rotations = self.rotations().values().collect::<Vec<_>>();
        rotations.sort_by(|a, b| a.name.cmp(&b.name));

        let mut blocks = vec![SCHEDULE.to_string()];
        if !week.is_empty() {
            blocks.push(week.join("\n"));
        }
        for rotation in rotations {
            let mut lines = vec![format!("{ROTATION}{SEPARATOR}{}", rotation.name)];
            for routine_id in rotation.routines() {
                lines.push(format!("- {}", reference(*routine_id)));
            }
            blocks.push(lines.join("\n"));
        }
        for routine_id in used {
            if let Some(routine) = routines.iter().find(|r| r.id == routine_id) {
                blocks.push(routine.to_text(exercises));
            }
        }
        blocks.join("\n\n")
    }
}

/// Names a routine in a slot or a rotation, prefixing a name that would otherwise be read as
/// naming a rotation or a routine by its prefix.
fn routine_reference(routine_id: RoutineID, routines: &[Routine]) -> String {
    match routines.iter().find(|r| r.id == routine_id) {
        Some(routine) if starts_with_kind(routine.name.as_ref()) => {
            format!("{ROUTINE}{SEPARATOR}{}", routine.name)
        }
        Some(routine) => routine.name.to_string(),
        None => format!("{ROUTINE}#{}", *routine_id),
    }
}

/// Whether `name` starts with `Routine` or `Rotation` followed by a separator.
fn starts_with_kind(name: &str) -> bool {
    [ROUTINE, ROTATION]
        .iter()
        .any(|kind| strip_kind(name, kind).is_some())
}

/// Returns the remainder of `line` after `kind` and a separator.
fn strip_kind<'a>(line: &'a str, kind: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(kind)?;
    SEPARATORS
        .iter()
        .find_map(|separator| rest.strip_prefix(separator))
}

fn routine_lines(routine: &Routine, exercises: &[Exercise]) -> Vec<String> {
    let mut lines = vec![format!("{ROUTINE}{SEPARATOR}{}", routine.name)];
    if !routine.notes.is_empty() {
        lines.push(String::new());
        lines.extend(routine.notes.split('\n').map(ToString::to_string));
    }
    for (i, section) in routine.sections.iter().enumerate() {
        lines.push(String::new());
        lines.extend(part_lines(section, &section_label(i), 0, exercises));
    }
    lines
}

/// The label of the top-level section at `index`: a letter, or its number from the 27th section
/// on.
fn section_label(index: usize) -> String {
    u8::try_from(index)
        .ok()
        .map(|n| char::from(b'A'.saturating_add(n)))
        .filter(char::is_ascii_uppercase)
        .map_or_else(|| format!("#{}", index + 1), |c| c.to_string())
}

fn part_label(parent: &str, n: usize) -> String {
    if parent.ends_with(|c: char| c.is_ascii_digit()) {
        format!("{parent}.{n}")
    } else {
        format!("{parent}{n}")
    }
}

fn part_lines(
    part: &RoutinePart,
    label: &str,
    depth: usize,
    exercises: &[Exercise],
) -> Vec<String> {
    let indent = "  ".repeat(depth);
    let mut lines = vec![];
    match part {
        RoutinePart::RoutineSection { rounds, parts } => {
            let r = u32::from(*rounds);
            let sets_label = if r == 1 { "set" } else { "sets" };
            lines.push(format!("{indent}[{label}] {r} {sets_label}"));
            let mut counter = 1;
            for part in parts {
                match part {
                    RoutinePart::RoutineSection { .. } => {
                        let label = part_label(label, counter);
                        counter += 1;
                        lines.extend(part_lines(part, &label, depth + 1, exercises));
                        lines.push(String::new());
                    }
                    RoutinePart::RoutineActivity { exercise_id, .. } if exercise_id.is_nil() => {
                        lines.extend(part_lines(part, "", depth + 1, exercises));
                    }
                    RoutinePart::RoutineActivity { .. } => {
                        let label = part_label(label, counter);
                        counter += 1;
                        lines.extend(part_lines(part, &label, depth + 1, exercises));
                    }
                }
            }
            if lines.last().is_some_and(String::is_empty) {
                lines.pop();
            }
        }
        RoutinePart::RoutineActivity {
            exercise_id,
            reps,
            tempo,
            weight,
            rpe,
            automatic,
            sides,
        } => {
            let line = if exercise_id.is_nil() {
                let mut line = format!("{indent}Rest");
                if let Some(tempo) = tempo.non_zero() {
                    let _ = write!(line, "{SEPARATOR}{tempo}");
                }
                if !automatic {
                    let _ = write!(line, "{SEPARATOR}manual");
                }
                line
            } else {
                let name = exercises.iter().find(|e| e.id == *exercise_id).map_or_else(
                    || format!("Exercise#{}", **exercise_id),
                    |e| e.name.to_string(),
                );
                let targets = values_to_string(
                    reps.non_zero(),
                    weight.non_zero(),
                    rpe.non_zero(),
                    &tempo_text(*tempo, reps.non_zero().is_some()),
                );
                let mut line = format!("{indent}{label}{SEPARATOR}{name}");
                if !targets.is_empty() {
                    let _ = write!(line, "{SEPARATOR}{targets}");
                }
                if *sides == Sides::PerSide {
                    let _ = write!(line, "{SEPARATOR}per side");
                }
                if *automatic {
                    let _ = write!(line, "{SEPARATOR}auto-start");
                }
                line
            };
            lines.push(line);
        }
    }
    lines
}

/// Writes a tempo following reps with its unit per repetition, as a bare time could be read as
/// the duration of the set.
fn tempo_text(tempo: Tempo, follows_reps: bool) -> String {
    if follows_reps && tempo.non_zero().is_some() {
        let phases = tempo
            .phases()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\u{00b7}");
        format!("{phases} s/rep")
    } else {
        tempo.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use crate::{
        ExerciseID, ExerciseMuscle, MuscleID, Name, RPE, Reps, Rotation, RotationID, Rounds,
        Stimulus, Weight,
    };

    use super::*;

    #[test]
    fn test_routine_to_text() {
        assert_eq!(
            routine(1, "A", "B", example_sections()).to_text(&exercises()),
            "Routine \u{2014} A\n\nB\n\n[A] 2 sets\n  A1 \u{2014} A \u{2014} 10 \u{00d7} 30 kg @ 10 (2 s/rep)\n  Rest \u{2014} 60 s\n\n[B] 2 sets\n  B1 \u{2014} Exercise#00000000-0000-0000-0000-000000000002 \u{2014} 10\n  Rest \u{2014} 30 s"
        );
    }

    #[test]
    fn test_routine_to_text_without_notes() {
        assert_eq!(
            routine(0, "N", "", vec![section(1, vec![activity(1, 5)])]).to_text(&exercises()),
            "Routine \u{2014} N\n\n[A] 1 set\n  A1 \u{2014} A \u{2014} 5"
        );
    }

    #[rstest]
    #[case::single_phase_after_reps(8, &[2], 0.0, "8 (2 s/rep)")]
    #[case::several_phases_after_reps(8, &[3, 1, 1, 0], 80.0, "8 \u{00d7} 80 kg (3\u{00b7}1\u{00b7}1\u{00b7}0 s/rep)")]
    #[case::single_phase_after_weight(0, &[60], 40.0, "40 kg (60 s)")]
    #[case::hold(0, &[30], 0.0, "30 s")]
    #[case::several_phases_without_reps(0, &[5, 10], 0.0, "5\u{00b7}10")]
    fn test_routine_to_text_tempo(
        #[case] reps: u32,
        #[case] phases: &[u32],
        #[case] weight: f32,
        #[case] expected: &str,
    ) {
        let routine = routine(
            0,
            "N",
            "",
            vec![section(
                1,
                vec![RoutinePart::RoutineActivity {
                    exercise_id: 1.into(),
                    reps: Reps::new(reps).unwrap(),
                    tempo: Tempo::new(phases).unwrap(),
                    weight: Weight::new(weight).unwrap(),
                    rpe: RPE::ZERO,
                    automatic: false,
                    sides: Sides::Combined,
                }],
            )],
        );

        assert_eq!(
            routine.to_text(&exercises()).lines().last(),
            Some(format!("  A1 \u{2014} A \u{2014} {expected}").as_str())
        );
    }

    #[test]
    fn test_routine_to_text_includes_rpe() {
        let routine = routine(
            0,
            "N",
            "",
            vec![section(
                1,
                vec![RoutinePart::RoutineActivity {
                    exercise_id: 1.into(),
                    reps: Reps::default(),
                    tempo: Tempo::new(&[60]).unwrap(),
                    weight: Weight::default(),
                    rpe: RPE::EIGHT,
                    automatic: false,
                    sides: Sides::Combined,
                }],
            )],
        );

        assert_eq!(
            routine.to_text(&exercises()).lines().last(),
            Some("  A1 \u{2014} A \u{2014} 60 s @ 8")
        );
    }

    #[test]
    fn test_routine_to_text_time_only_and_rest_without_time() {
        let routine = routine(
            0,
            "T",
            "",
            vec![section(
                2,
                vec![
                    RoutinePart::RoutineActivity {
                        exercise_id: 1.into(),
                        reps: Reps::default(),
                        tempo: Tempo::new(&[30]).unwrap(),
                        weight: Weight::default(),
                        rpe: RPE::ZERO,
                        automatic: false,
                        sides: Sides::Combined,
                    },
                    rest(&[], true),
                ],
            )],
        );

        assert_eq!(
            routine.to_text(&exercises()),
            "Routine \u{2014} T\n\n[A] 2 sets\n  A1 \u{2014} A \u{2014} 30 s\n  Rest"
        );
    }

    #[test]
    fn test_routine_to_text_nested() {
        let routine = routine(
            0,
            "N",
            "",
            vec![section(
                2,
                vec![
                    activity(1, 5),
                    section(3, vec![activity(1, 8), rest(&[30], true)]),
                    rest(&[90], true),
                ],
            )],
        );

        assert_eq!(
            routine.to_text(&exercises()),
            "Routine \u{2014} N\n\n[A] 2 sets\n  A1 \u{2014} A \u{2014} 5\n  [A2] 3 sets\n    A2.1 \u{2014} A \u{2014} 8\n    Rest \u{2014} 30 s\n\n  Rest \u{2014} 90 s"
        );
    }

    #[test]
    fn test_routine_to_text_without_values() {
        assert_eq!(
            routine(0, "X", "", vec![section(1, vec![activity(1, 0)])]).to_text(&exercises()),
            "Routine \u{2014} X\n\n[A] 1 set\n  A1 \u{2014} A"
        );
    }

    #[rstest]
    #[case::per_side_without_values(0, Sides::PerSide, false, "A \u{2014} per side")]
    #[case::per_side_with_values(10, Sides::PerSide, false, "A \u{2014} 10 \u{2014} per side")]
    #[case::auto_start(10, Sides::Combined, true, "A \u{2014} 10 \u{2014} auto-start")]
    #[case::per_side_and_auto_start(
        0,
        Sides::PerSide,
        true,
        "A \u{2014} per side \u{2014} auto-start"
    )]
    fn test_routine_to_text_suffixes_of_activity(
        #[case] reps: u32,
        #[case] sides: Sides,
        #[case] automatic: bool,
        #[case] expected: &str,
    ) {
        let routine = routine(
            0,
            "N",
            "",
            vec![section(
                1,
                vec![RoutinePart::RoutineActivity {
                    exercise_id: 1.into(),
                    reps: Reps::new(reps).unwrap(),
                    tempo: Tempo::default(),
                    weight: Weight::default(),
                    rpe: RPE::ZERO,
                    automatic,
                    sides,
                }],
            )],
        );

        assert_eq!(
            routine.to_text(&exercises()).lines().last(),
            Some(format!("  A1 \u{2014} {expected}").as_str())
        );
    }

    #[rstest]
    #[case::automatic_with_time(&[60], true, "Rest \u{2014} 60 s")]
    #[case::manual_with_time(&[60], false, "Rest \u{2014} 60 s \u{2014} manual")]
    #[case::manual_without_time(&[], false, "Rest \u{2014} manual")]
    fn test_routine_to_text_suffix_of_rest(
        #[case] phases: &[u32],
        #[case] automatic: bool,
        #[case] expected: &str,
    ) {
        let routine = routine(0, "N", "", vec![section(1, vec![rest(phases, automatic)])]);

        assert_eq!(
            routine.to_text(&exercises()).lines().last(),
            Some(format!("  {expected}").as_str())
        );
    }

    #[test]
    fn test_routine_to_text_many_sections() {
        let routine = routine(0, "N", "", (0..27).map(|_| section(1, vec![])).collect());

        let text = routine.to_text(&[]);

        assert!(text.contains("\n[Z] 1 set"), "{text}");
        assert!(text.contains("\n[#27] 1 set"), "{text}");
    }

    #[test]
    fn test_routine_to_text_numbers_sibling_sections() {
        let routine = routine(
            0,
            "N",
            "",
            vec![section(
                1,
                vec![
                    section(1, vec![activity(1, 1)]),
                    section(1, vec![activity(1, 2)]),
                ],
            )],
        );

        let text = routine.to_text(&exercises());

        assert!(text.contains("[A1] 1 set"), "{text}");
        assert!(text.contains("[A2] 1 set"), "{text}");
    }

    #[test]
    fn test_routine_to_text_does_not_number_rests() {
        let routine = routine(
            0,
            "N",
            "",
            vec![section(
                1,
                vec![activity(1, 1), rest(&[60], true), activity(1, 2)],
            )],
        );

        let text = routine.to_text(&exercises());

        assert!(text.contains("A1 \u{2014} A \u{2014} 1"), "{text}");
        assert!(text.contains("A2 \u{2014} A \u{2014} 2"), "{text}");
    }

    #[test]
    fn test_schedule_to_text() {
        let schedule = schedule(
            vec![(1, "Upper/Lower", vec![2, 3])],
            vec![
                (Weekday::Monday, vec![ScheduleSlot::Routine(1.into())]),
                (Weekday::Wednesday, vec![ScheduleSlot::Rotation(1.into())]),
                (
                    Weekday::Friday,
                    vec![
                        ScheduleSlot::Routine(4.into()),
                        ScheduleSlot::Routine(5.into()),
                    ],
                ),
            ],
        );
        let routines = vec![
            routine(1, "Push Day", "", vec![]),
            routine(2, "Upper Day", "", vec![]),
            routine(3, "Lower Day", "", vec![]),
            routine(4, "Monday", "", vec![]),
            routine(5, "Mobility", "", vec![section(1, vec![activity(1, 5)])]),
        ];

        assert_eq!(
            schedule.to_text(&routines, &exercises()),
            "Schedule\n\n\
             Monday\n- Push Day\nWednesday\n- Rotation \u{2014} Upper/Lower\nFriday\n- Monday\n- Mobility\n\n\
             Rotation \u{2014} Upper/Lower\n- Upper Day\n- Lower Day\n\n\
             Routine \u{2014} Push Day\n\n\
             Routine \u{2014} Monday\n\n\
             Routine \u{2014} Mobility\n\n[A] 1 set\n  A1 \u{2014} A \u{2014} 5\n\n\
             Routine \u{2014} Upper Day\n\n\
             Routine \u{2014} Lower Day"
        );
    }

    #[test]
    fn test_schedule_to_text_empty() {
        assert_eq!(Schedule::default().to_text(&[], &[]), "Schedule");
    }

    #[test]
    fn test_schedule_to_text_rotations_ordered_by_name() {
        let schedule = schedule(vec![(1, "B", vec![]), (2, "A", vec![])], vec![]);

        assert_eq!(
            schedule.to_text(&[], &[]),
            "Schedule\n\nRotation \u{2014} A\n\nRotation \u{2014} B"
        );
    }

    #[test]
    fn test_schedule_to_text_slot_sharing_a_name_with_a_rotation() {
        let schedule = schedule(
            vec![(1, "A", vec![1])],
            vec![(
                Weekday::Monday,
                vec![
                    ScheduleSlot::Routine(1.into()),
                    ScheduleSlot::Rotation(1.into()),
                ],
            )],
        );

        assert_eq!(
            schedule.to_text(&[routine(1, "A", "", vec![])], &[]),
            "Schedule\n\nMonday\n- A\n- Rotation \u{2014} A\n\nRotation \u{2014} A\n- A\n\nRoutine \u{2014} A"
        );
    }

    #[rstest]
    #[case::rotation_em_dash("Rotation \u{2014} X")]
    #[case::rotation_hyphen("Rotation - X")]
    #[case::rotation_en_dash("Rotation \u{2013} X")]
    #[case::routine_em_dash("Routine \u{2014} X")]
    #[case::routine_hyphen("Routine - X")]
    #[case::routine_en_dash("Routine \u{2013} X")]
    fn test_schedule_to_text_prefixes_a_name_starting_with_a_kind(#[case] name: &str) {
        let schedule = schedule(
            vec![(1, "R", vec![1])],
            vec![(Weekday::Monday, vec![ScheduleSlot::Routine(1.into())])],
        );

        let text = schedule.to_text(&[routine(1, name, "", vec![])], &[]);

        assert_eq!(
            text.lines()
                .filter(|line| line.starts_with("- R"))
                .collect::<Vec<_>>(),
            vec![format!("- Routine \u{2014} {name}"); 2]
        );
    }

    #[rstest]
    #[case::without_separator("Routine X")]
    #[case::without_spaces("Rotation-X")]
    fn test_schedule_to_text_does_not_prefix_other_names(#[case] name: &str) {
        let schedule = schedule(
            vec![],
            vec![(Weekday::Monday, vec![ScheduleSlot::Routine(1.into())])],
        );

        let text = schedule.to_text(&[routine(1, name, "", vec![])], &[]);

        assert!(text.contains(&format!("\n- {name}\n")), "{text}");
    }

    #[test]
    fn test_schedule_to_text_slot_of_unknown_routine() {
        let schedule = schedule(
            vec![(1, "R", vec![2])],
            vec![(Weekday::Monday, vec![ScheduleSlot::Routine(1.into())])],
        );

        assert_eq!(
            schedule.to_text(&[], &[]),
            "Schedule\n\nMonday\n- Routine#00000000-0000-0000-0000-000000000001\n\nRotation \u{2014} R\n- Routine#00000000-0000-0000-0000-000000000002"
        );
    }

    fn routine(id: u128, name: &str, notes: &str, sections: Vec<RoutinePart>) -> Routine {
        Routine {
            id: id.into(),
            name: Name::new(name).unwrap(),
            notes: notes.to_string(),
            archived: false,
            sections,
        }
    }

    fn example_sections() -> Vec<RoutinePart> {
        vec![
            section(
                2,
                vec![
                    RoutinePart::RoutineActivity {
                        exercise_id: 1.into(),
                        reps: Reps::new(10).unwrap(),
                        tempo: Tempo::new(&[2]).unwrap(),
                        weight: Weight::new(30.0).unwrap(),
                        rpe: RPE::TEN,
                        automatic: false,
                        sides: Sides::Combined,
                    },
                    rest(&[60], true),
                ],
            ),
            section(
                2,
                vec![
                    RoutinePart::RoutineActivity {
                        exercise_id: 2.into(),
                        reps: Reps::new(10).unwrap(),
                        tempo: Tempo::default(),
                        weight: Weight::default(),
                        rpe: RPE::ZERO,
                        automatic: false,
                        sides: Sides::Combined,
                    },
                    rest(&[30], true),
                ],
            ),
        ]
    }

    fn section(rounds: u32, parts: Vec<RoutinePart>) -> RoutinePart {
        RoutinePart::RoutineSection {
            rounds: Rounds::new(rounds).unwrap(),
            parts,
        }
    }

    fn activity(exercise_id: u128, reps: u32) -> RoutinePart {
        RoutinePart::RoutineActivity {
            exercise_id: exercise_id.into(),
            reps: Reps::new(reps).unwrap(),
            tempo: Tempo::default(),
            weight: Weight::default(),
            rpe: RPE::ZERO,
            automatic: false,
            sides: Sides::Combined,
        }
    }

    fn rest(phases: &[u32], automatic: bool) -> RoutinePart {
        RoutinePart::RoutineActivity {
            exercise_id: ExerciseID::nil(),
            reps: Reps::default(),
            tempo: Tempo::new(phases).unwrap(),
            weight: Weight::default(),
            rpe: RPE::ZERO,
            automatic,
            sides: Sides::Combined,
        }
    }

    fn schedule(
        rotations: Vec<(u128, &str, Vec<u128>)>,
        entries: Vec<(Weekday, Vec<ScheduleSlot>)>,
    ) -> Schedule {
        Schedule::new(
            rotations
                .into_iter()
                .map(|(id, name, routines)| {
                    (
                        RotationID::from(id),
                        Rotation::new(
                            Name::new(name).unwrap(),
                            routines.into_iter().map(RoutineID::from).collect(),
                        )
                        .unwrap(),
                    )
                })
                .collect(),
            entries.into_iter().collect::<BTreeMap<_, _>>(),
        )
        .unwrap()
    }

    fn exercises() -> Vec<Exercise> {
        vec![Exercise {
            id: 1.into(),
            name: Name::new("A").unwrap(),
            notes: String::new(),
            muscles: vec![ExerciseMuscle {
                muscle_id: MuscleID::Pecs,
                stimulus: Stimulus::PRIMARY,
                stretch: Stimulus::NONE,
            }],
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }]
    }
}
