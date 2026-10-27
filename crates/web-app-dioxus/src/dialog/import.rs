//! Import of routines and the schedule from a text.
//!
//! The dialog first reads the text with [`domain::parse_text`] and shows its errors. A text
//! without errors is shown as a preview, in which the user reviews the changes of existing routines
//! and resolves routines whose name is already used and exercises that match none of their
//! exercises, before [`ImportService::import_text`] saves it.

use std::collections::BTreeMap;

use dioxus::prelude::*;

use valens_domain::{self as domain, ImportService};

use crate::{
    DOMAIN_SERVICE,
    cache::{Cache, CacheState},
    notification::notify,
    page,
    ui::{
        element::{Dialog, Error, Icon, LoadingDialog, SaveDialog, SectionCaption, Table},
        form::{ButtonSelect, ButtonSelectOption, InputField, TextAreaField},
    },
};

/// What an import may change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImportTarget {
    /// Routines only, rejecting a text with a schedule.
    Routines,
    /// Routines and the schedule.
    Schedule,
}

#[component]
pub fn ImportDialog(target: ImportTarget, on_close: EventHandler<()>) -> Element {
    let cache = consume_context::<Cache>();
    let mut input = use_signal(String::new);
    let mut errors = use_signal(Vec::<String>::new);
    let preview = use_signal(|| None::<Preview>);
    let is_loading = use_signal(|| false);

    let exercises = cache.exercises.read();
    let routines = cache.routines.read();
    let (exercises, routines) = match (&*exercises, &*routines) {
        (CacheState::Ready(exercises), CacheState::Ready(routines)) => (exercises, routines),
        (CacheState::Error(err), _) | (_, CacheState::Error(err)) => {
            return rsx! {
                Dialog {
                    on_close: move |_| on_close.call(()),
                    Error { message: "{err}" }
                }
            };
        }
        (CacheState::Loading, _) | (_, CacheState::Loading) => return rsx! { LoadingDialog {} },
    };

    if preview.read().is_some() {
        return view_preview(preview, is_loading, exercises, routines, on_close);
    }

    rsx! {
        SaveDialog {
            title: rsx! { "Import from text" },
            save_label: "Next",
            on_close: move |_| on_close.call(()),
            on_save: move |()| {
                let (CacheState::Ready(exercises), CacheState::Ready(routines)) =
                    (&*cache.exercises.peek(), &*cache.routines.peek())
                else {
                    return;
                };
                let mut preview = preview;
                match parse(&input.read(), target, routines) {
                    Ok(text) => preview.set(Some(Preview::new(text, exercises))),
                    Err(text_errors) => errors.set(text_errors),
                }
            },
            is_loading: false,
            disabled: input.read().trim().is_empty(),
            TextAreaField {
                label: "Text".to_string(),
                help: "Use the format of \"Show as text\".",
                value: input(),
                errors: errors(),
                error_testid: "import-error",
                has_changed: false,
                autofocus: true,
                rows: 12,
                "data-testid": "import-text",
                on_input: move |event: FormEvent| {
                    input.set(event.value());
                    errors.set(vec![]);
                },
            }
        }
    }
}

fn view_preview(
    mut preview: Signal<Option<Preview>>,
    mut is_loading: Signal<bool>,
    exercises: &[domain::Exercise],
    routines: &[domain::Routine],
    on_close: EventHandler<()>,
) -> Element {
    let Some(current) = preview.read().clone() else {
        return rsx! {};
    };
    let changes = current.changes(routines, exercises);
    let copy_names = current.copy_names(routines, &changes);
    let choices = current.choices(&copy_names);

    let routine_rows = current
        .text
        .routines
        .iter()
        .zip(&changes)
        .map(|(routine, change)| {
            let name = routine.name.clone();
            let copy_name = copy_names.get(&routine.name);
            let existing = routines.iter().find(|r| r.name == routine.name);
            let (previous_sets, sets) = routine_summary(routine, existing, change);
            let status = match change {
                domain::RoutineChange::New => rsx! {
                    span { class: "tag is-success is-light", "data-testid": "import-routine-status", "New" }
                },
                domain::RoutineChange::Unchanged => rsx! {
                    span { class: "tag", "data-testid": "import-routine-status", "Unchanged" }
                },
                domain::RoutineChange::Changed(_) => rsx! {
                    ButtonSelect {
                        "data-testid": "import-routine-choice",
                        is_small: true,
                        options: vec![
                            ButtonSelectOption { text: "Replace".to_string(), icon: None, value: false },
                            ButtonSelectOption { text: "Create copy".to_string(), icon: None, value: true },
                        ],
                        selected: copy_name.is_some(),
                        on_click: {
                            let name = name.clone();
                            let routines = routines.to_vec();
                            move |(_, copy): (MouseEvent, bool)| {
                                if let Some(preview) = &mut *preview.write() {
                                    if copy {
                                        preview.create_copy(&name, &routines);
                                    } else {
                                        preview.copies.remove(&name);
                                    }
                                }
                            }
                        },
                    }
                },
            };
            vec![rsx! {
                div {
                    class: "is-flex is-flex-wrap-wrap is-justify-content-space-between is-align-items-center mb-2",
                    div {
                        class: "mr-2",
                        span { "data-testid": "import-routine-name", "{routine.name}" }
                        p {
                            class: "help is-flex is-align-items-center",
                            "data-testid": "import-routine-sets",
                            if let Some(previous_sets) = previous_sets {
                                "{previous_sets}"
                                Icon { name: "arrow-right", is_small: true, class: "mx-1" }
                            }
                            "{sets}"
                        }
                    }
                    {status}
                }
                if let Some(copy_name) = copy_name {
                    InputField {
                        label: "Name".to_string(),
                        "data-testid": "import-copy-name",
                        value: current.copies.get(&routine.name).cloned().unwrap_or_default(),
                        error: copy_name.clone().err(),
                        has_changed: false,
                        on_input: {
                            let name = name.clone();
                            move |event: FormEvent| {
                                if let Some(preview) = &mut *preview.write() {
                                    preview.copies.insert(name.clone(), event.value());
                                }
                            }
                        },
                    }
                }
                if let domain::RoutineChange::Changed(lines) = change {
                    {view_line_changes(lines)}
                }
            }]
        })
        .collect::<Vec<_>>();

    let exercise_choices = domain::ImportChoices {
        exercises: current.exercise_choices(),
        copies: BTreeMap::new(),
    };
    let exercise_rows = current
        .exercises
        .iter()
        .map(|(name, _)| {
            let choice = exercise_choices.exercise(name, exercises);
            let description = match &choice {
                domain::ExerciseChoice::Existing(id) => exercises
                    .iter()
                    .find(|e| e.id == *id)
                    .map_or_else(String::new, |e| format!("Use {}", e.name)),
                domain::ExerciseChoice::Catalog(catalog_name) => {
                    format!("Create from catalog: {catalog_name}")
                }
                domain::ExerciseChoice::New => "Create new exercise".to_string(),
            };
            let is_new = choice == domain::ExerciseChoice::New;
            let name = name.clone();
            vec![rsx! {
                div {
                    "data-testid": "import-exercise",
                    div {
                        class: "is-flex is-justify-content-space-between is-align-items-center",
                        span { "{name}" }
                    }
                    div {
                        class: "is-flex is-align-items-center is-justify-content-space-between",
                        span { class: "is-italic", "data-testid": "import-exercise-choice", "{description}" }
                        div {
                            class: "is-flex is-flex-wrap-nowrap",
                            if !is_new {
                                a {
                                    class: "has-text-link mx-2",
                                    title: "Reset",
                                    aria_label: "Reset",
                                    "data-testid": "import-exercise-reset",
                                    onclick: {
                                        let name = name.clone();
                                        move |_| choose_exercise(preview, &name, domain::ExerciseChoice::New)
                                    },
                                    Icon { name: "rotate-left", is_small: true }
                                }
                            }
                            a {
                                class: "has-text-link mx-2",
                                title: "Change",
                                aria_label: "Change",
                                "data-testid": "import-exercise-change",
                                onclick: move |_| {
                                    if let Some(preview) = &mut *preview.write() {
                                        preview.picking = Some(name.clone());
                                    }
                                },
                                Icon { name: "right-left", is_small: true }
                            }
                        }
                    }
                }
            }]
        })
        .collect::<Vec<_>>();

    let text = current.text.clone();
    let is_ready = choices.is_some();
    rsx! {
        SaveDialog {
            title: rsx! { "Import from text" },
            save_label: "Import",
            on_close: move |_| on_close.call(()),
            has_back: true,
            on_back: move |_| preview.set(None),
            on_save: move |()| {
                let text = text.clone();
                if let Some(choices) = choices.clone() {
                    let cache = consume_context::<Cache>();
                    is_loading.set(true);
                    // A task of the dialog would be cancelled when the dialog is closed, leaving
                    // the import incomplete and the cache outdated.
                    dioxus::dioxus_core::spawn_forever(async move {
                        let result = DOMAIN_SERVICE().import_text(&text, &choices).await;
                        // A dialog closed in the meantime must not close a dialog opened since.
                        let is_open = if let Ok(mut is_loading) = is_loading.try_write() {
                            *is_loading = false;
                            true
                        } else {
                            false
                        };
                        cache.refresh_exercises();
                        cache.refresh_routines();
                        cache.refresh_schedule();
                        match result {
                            Ok(()) if is_open => on_close.call(()),
                            Ok(()) => {}
                            Err(err) => notify(err.action(), &err),
                        }
                    });
                }
            },
            is_loading: is_loading(),
            disabled: !is_ready,
            if !routine_rows.is_empty() {
                SectionCaption { "Routines" }
                Table { body: routine_rows }
            }
            if !exercise_rows.is_empty() {
                SectionCaption { "Unknown exercises" }
                p { class: "help", "Choose an existing exercise or one from the catalog, or create a new one." }
                Table { body: exercise_rows }
            }
            if let Some(schedule) = &current.text.schedule {
                {view_schedule(schedule)}
            }
        }
        // Rendered outside of the form of the preview, as Dioxus skips the submit handler of a
        // nested form.
        if let Some(name) = current.picking {
            {view_exercise_picker(preview, name)}
        }
    }
}

fn view_exercise_picker(mut preview: Signal<Option<Preview>>, name: domain::Name) -> Element {
    let mut close = move || {
        if let Some(preview) = &mut *preview.write() {
            preview.picking = None;
        }
    };
    rsx! {
        Dialog {
            title: rsx! { "Exercise for \u{201c}{name}\u{201d}" },
            on_close: move |_| close(),
            no_horizontal_padding: true,
            page::exercises::ExerciseList {
                add: false,
                filter: domain::ExerciseFilter {
                    name: domain::catalog_search_term(&name),
                    ..domain::ExerciseFilter::default()
                },
                on_exercise_click: {
                    let name = name.clone();
                    move |(_, id)| {
                        choose_exercise(preview, &name, domain::ExerciseChoice::Existing(id));
                        close();
                    }
                },
                on_catalog_click: move |(_, catalog_name): (MouseEvent, String)| {
                    if let Ok(catalog_name) = domain::Name::new(&catalog_name) {
                        choose_exercise(preview, &name, domain::ExerciseChoice::Catalog(catalog_name));
                    }
                    close();
                },
            }
        }
    }
}

fn choose_exercise(
    mut preview: Signal<Option<Preview>>,
    name: &domain::Name,
    choice: domain::ExerciseChoice,
) {
    if let Some(preview) = &mut *preview.write()
        && let Some((_, chosen)) = preview.exercises.iter_mut().find(|(n, _)| n == name)
    {
        *chosen = Some(choice);
    }
}

fn view_schedule(schedule: &domain::ParsedSchedule) -> Element {
    let slot_name = |slot: &domain::ParsedSlot| match slot {
        domain::ParsedSlot::Routine(reference) => rsx! { "{reference.name}" },
        domain::ParsedSlot::Rotation(reference) => rsx! {
            Icon { name: "rotate", is_small: true, class: "mr-2".to_string() }
            "{reference.name}"
        },
    };
    let week = domain::Weekday::iter()
        .map(|weekday| {
            let slots = schedule.entries.get(weekday).cloned().unwrap_or_default();
            vec![
                rsx! { "{weekday}" },
                if slots.is_empty() {
                    rsx! { span { class: "has-text-weak", "Rest day" } }
                } else {
                    rsx! {
                        for slot in slots {
                            div { "data-testid": "import-slot", {slot_name(&slot)} }
                        }
                    }
                },
            ]
        })
        .collect::<Vec<_>>();
    let rotations = schedule
        .rotations
        .iter()
        .map(|rotation| {
            vec![
                rsx! {
                    Icon { name: "rotate", is_small: true, class: "mr-2".to_string() }
                    "{rotation.name}"
                },
                rsx! {
                    for reference in &rotation.routines {
                        div { "{reference.name}" }
                    }
                },
            ]
        })
        .collect::<Vec<_>>();
    rsx! {
        SectionCaption { "Schedule" }
        p { class: "help", "Replaces the current schedule." }
        div { "data-testid": "import-week", Table { body: week } }
        if !rotations.is_empty() {
            SectionCaption { "Rotations" }
            Table { body: rotations }
        }
    }
}

/// Reads a text, rejecting a schedule if only routines are imported, and otherwise routines of the
/// schedule that neither the text nor the user has.
fn parse(
    input: &str,
    target: ImportTarget,
    routines: &[domain::Routine],
) -> Result<domain::ParsedText, Vec<String>> {
    let text = domain::parse_text(input)
        .map_err(|errors| errors.iter().map(ToString::to_string).collect::<Vec<_>>())?;
    let errors = match (&text.schedule, target) {
        (Some(schedule), ImportTarget::Routines) => vec![format!(
            "line {}: schedule can only be imported on the schedule page",
            schedule.line
        )],
        _ => text
            .unknown_routines(routines)
            .iter()
            .map(ToString::to_string)
            .collect(),
    };
    if errors.is_empty() {
        Ok(text)
    } else {
        Err(errors)
    }
}

fn view_line_changes(lines: &[domain::LineChange]) -> Element {
    let added = lines
        .iter()
        .filter(|line| matches!(line, domain::LineChange::Added(_)))
        .count();
    let removed = lines
        .iter()
        .filter(|line| matches!(line, domain::LineChange::Removed(_)))
        .count();
    rsx! {
        details {
            "data-testid": "import-routine-changes",
            summary {
                class: "is-size-7 has-text-link",
                "data-testid": "import-routine-changes-toggle",
                "Show changes (+{added} \u{2212}{removed})"
            }
            div {
                class: "is-family-monospace is-size-7 mt-1",
                style: "white-space: pre-wrap",
                for line in lines {
                    match line {
                        domain::LineChange::Kept(line) => rsx! { div { class: "has-text-weak", "  {line}" } },
                        domain::LineChange::Removed(line) => rsx! {
                            div { class: "has-text-danger", "data-testid": "import-removed-line", "- {line}" }
                        },
                        domain::LineChange::Added(line) => rsx! {
                            div { class: "has-text-success", "data-testid": "import-added-line", "+ {line}" }
                        },
                    }
                }
            }
        }
    }
}

/// The number of sets of the existing routine if a change alters it, and that of the imported
/// routine with its unit.
fn routine_summary(
    routine: &domain::ParsedRoutine,
    existing: Option<&domain::Routine>,
    change: &domain::RoutineChange,
) -> (Option<u32>, String) {
    let sets = domain::Routine {
        id: domain::RoutineID::nil(),
        name: routine.name.clone(),
        notes: String::new(),
        archived: false,
        sections: routine.sections(&|_| domain::ExerciseID::from(1)),
    }
    .num_sets();
    let unit = if sets == 1 { "set" } else { "sets" };
    let previous_sets = match (existing, change) {
        (Some(existing), domain::RoutineChange::Changed(_)) if existing.num_sets() != sets => {
            Some(existing.num_sets())
        }
        _ => None,
    };
    (previous_sets, format!("{sets} {unit}"))
}

/// The state of the preview of a read text.
#[derive(Clone, Debug, PartialEq)]
struct Preview {
    text: domain::ParsedText,
    /// The input of the name of each routine saved as a copy.
    copies: BTreeMap<domain::Name, String>,
    /// Each unknown exercise name with the choice for it, or `None` to resolve it as the import
    /// does without a choice.
    exercises: Vec<(domain::Name, Option<domain::ExerciseChoice>)>,
    /// The unknown exercise name for which an exercise is being chosen.
    picking: Option<domain::Name>,
}

impl Preview {
    fn new(text: domain::ParsedText, exercises: &[domain::Exercise]) -> Self {
        let exercises = text
            .unknown_exercises(exercises)
            .into_iter()
            .map(|name| (name, None))
            .collect();
        Self {
            text,
            copies: BTreeMap::new(),
            exercises,
            picking: None,
        }
    }

    /// The change of each routine of the text.
    fn changes(
        &self,
        routines: &[domain::Routine],
        exercises: &[domain::Exercise],
    ) -> Vec<domain::RoutineChange> {
        let choices = domain::ImportChoices {
            exercises: self.exercise_choices(),
            copies: BTreeMap::new(),
        };
        self.text
            .routines
            .iter()
            .map(|routine| routine.change(routines, exercises, &choices))
            .collect()
    }

    /// Saves `routine` as a copy under a free name.
    fn create_copy(&mut self, routine: &domain::Name, routines: &[domain::Routine]) {
        let others = self
            .copies
            .iter()
            .filter_map(|(routine, input)| {
                domain::Name::new(input)
                    .ok()
                    .map(|name| (routine.clone(), name))
            })
            .collect();
        let name = domain::free_copy_name(routine, &self.text, routines, &others);
        self.copies.insert(routine.clone(), name.to_string());
    }

    /// The validated name of each copy of a changed routine.
    fn copy_names(
        &self,
        routines: &[domain::Routine],
        changes: &[domain::RoutineChange],
    ) -> BTreeMap<domain::Name, Result<domain::Name, String>> {
        let copies = self
            .text
            .routines
            .iter()
            .zip(changes)
            .filter(|(_, change)| matches!(change, domain::RoutineChange::Changed(_)))
            .filter_map(|(routine, _)| {
                self.copies
                    .get(&routine.name)
                    .map(|input| (routine.name.clone(), input))
            })
            .collect::<BTreeMap<_, _>>();
        let others = copies
            .iter()
            .filter_map(|(routine, input)| {
                domain::Name::new(input)
                    .ok()
                    .map(|name| (routine.clone(), name))
            })
            .collect::<BTreeMap<_, _>>();
        copies
            .iter()
            .map(|(routine, input)| {
                (
                    routine.clone(),
                    domain::validate_copy_name(input, routine, &self.text, routines, &others)
                        .map_err(|err| err.to_string()),
                )
            })
            .collect()
    }

    /// The choices of the preview, or `None` while the name of a copy is invalid.
    fn choices(
        &self,
        copy_names: &BTreeMap<domain::Name, Result<domain::Name, String>>,
    ) -> Option<domain::ImportChoices> {
        let copies = copy_names
            .iter()
            .map(|(routine, name)| name.clone().ok().map(|name| (routine.clone(), name)))
            .collect::<Option<BTreeMap<_, _>>>()?;
        Some(domain::ImportChoices {
            exercises: self.exercise_choices(),
            copies,
        })
    }

    fn exercise_choices(&self) -> BTreeMap<domain::Name, domain::ExerciseChoice> {
        self.exercises
            .iter()
            .filter_map(|(name, choice)| choice.clone().map(|choice| (name.clone(), choice)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{
        TestCache, all_attributes_of, all_text_of, contains, render, selected_text_of,
    };

    use super::*;

    #[test]
    fn test_parse_reports_errors_with_line_numbers() {
        assert_eq!(
            parse(
                "Routine — X\n\n[A] 1 set\n  A1 — A — 8 (30 s)",
                ImportTarget::Routines,
                &[]
            ),
            Err(vec![
                "line 4: time following reps must be followed by \"s/rep\"".to_string()
            ])
        );
    }

    #[test]
    fn test_parse_rejects_schedule_when_importing_routines() {
        let text = "Schedule\n\nMonday\n- A\n\nRoutine — A";

        assert_eq!(
            parse(text, ImportTarget::Routines, &[]),
            Err(vec![
                "line 1: schedule can only be imported on the schedule page".to_string()
            ])
        );
        assert!(parse(text, ImportTarget::Schedule, &[]).is_ok());
    }

    #[test]
    fn test_parse_rejects_unknown_routine_of_schedule() {
        assert_eq!(
            parse(
                "Schedule\n\nMonday\n- A\n- B",
                ImportTarget::Schedule,
                &[routine(1, "A", "")]
            ),
            Err(vec!["line 5: unknown routine \"B\"".to_string()])
        );
    }

    #[test]
    fn test_a_changed_routine_is_replaced_by_default() {
        let html = render_preview("Routine — A\n\nNotes\n\nRoutine — B", &[], BTreeMap::new());

        assert_eq!(all_text_of(&html, "import-routine-name"), vec!["A", "B"]);
        assert_eq!(selected_text_of(&html, "import-routine-choice"), "Replace");
        assert_eq!(all_text_of(&html, "import-routine-status"), vec!["New"]);
        assert!(!contains(&html, "import-copy-name"));
        assert!(!is_save_disabled(&html));
    }

    #[test]
    fn test_the_set_count_of_the_existing_routine_is_shown_if_it_changes() {
        let html = render_preview(
            "Routine — A\n\n[A] 1 set\n  A1 — X — 5\n\nRoutine — D\n\nOther notes\n\nRoutine — B",
            &[],
            BTreeMap::new(),
        );

        assert_eq!(
            all_text_of(&html, "import-routine-sets"),
            vec!["01 set", "0 sets", "0 sets"]
        );
        assert_eq!(html.matches("icon-arrow-right").count(), 1);
    }

    #[test]
    fn test_the_changes_of_a_changed_routine_are_shown() {
        let html = render_preview("Routine — D\n\nNew notes", &[], BTreeMap::new());

        assert_eq!(
            all_text_of(&html, "import-routine-changes-toggle"),
            vec!["Show changes (+1 \u{2212}1)"]
        );
        assert_eq!(all_text_of(&html, "import-removed-line"), vec!["- Notes"]);
        assert_eq!(all_text_of(&html, "import-added-line"), vec!["+ New notes"]);
    }

    #[test]
    fn test_an_unchanged_routine_offers_no_choice() {
        let html = render_preview(
            "Routine — A",
            &[],
            BTreeMap::from([(domain::Name::new("A").unwrap(), "A".to_string())]),
        );

        assert_eq!(
            all_text_of(&html, "import-routine-status"),
            vec!["Unchanged"]
        );
        assert!(!contains(&html, "import-routine-choice"));
        assert!(!contains(&html, "import-routine-changes"));
        assert!(!contains(&html, "import-copy-name"));
        assert!(!is_save_disabled(&html));
    }

    #[rstest::rstest]
    #[case::unknown("Mystery", None, "Create new exercise", false)]
    #[case::catalog_match("Bench Press", None, "Create from catalog: Barbell Bench Press", true)]
    #[case::chosen_exercise(
        "Mystery",
        Some(domain::ExerciseChoice::Existing(1.into())),
        "Use Squat",
        true
    )]
    #[case::chosen_catalog_exercise(
        "Mystery",
        Some(domain::ExerciseChoice::Catalog(domain::Name::new("Dip").unwrap())),
        "Create from catalog: Dip",
        true
    )]
    #[case::reset_catalog_match(
        "Bench Press",
        Some(domain::ExerciseChoice::New),
        "Create new exercise",
        false
    )]
    fn test_the_choice_for_an_unknown_exercise_is_shown(
        #[case] name: &'static str,
        #[case] choice: Option<domain::ExerciseChoice>,
        #[case] description: &str,
        #[case] can_reset: bool,
    ) {
        let html = render_preview_with(
            format!("Routine — B\n\n[A] 1 set\n  A1 — {name} — 5\n  A2 — Squat — 5"),
            &["Squat"],
            BTreeMap::new(),
            choice
                .map(|choice| (domain::Name::new(name).unwrap(), choice))
                .into_iter()
                .collect(),
        );

        assert_eq!(
            all_text_of(&html, "import-exercise-choice"),
            vec![description]
        );
        assert_eq!(contains(&html, "import-exercise-reset"), can_reset);
        assert!(!is_save_disabled(&html));
    }

    #[rstest::rstest]
    #[case::used_by_existing_routine("A")]
    #[case::used_by_new_routine("B")]
    #[case::used_by_other_copy("C")]
    fn test_import_is_disabled_while_copy_name_is_used(#[case] copy_name: &str) {
        let html = render_preview(
            "Routine — A\n\nx\n\nRoutine — B\n\nRoutine — D",
            &[],
            BTreeMap::from([
                (domain::Name::new("A").unwrap(), copy_name.to_string()),
                (domain::Name::new("D").unwrap(), "C".to_string()),
            ]),
        );

        assert_eq!(
            selected_text_of(&html, "import-routine-choice"),
            "Create copy"
        );
        assert!(is_save_disabled(&html));
    }

    #[test]
    fn test_import_is_enabled_with_unused_copy_name() {
        let html = render_preview(
            "Routine — A\n\nx",
            &[],
            BTreeMap::from([(domain::Name::new("A").unwrap(), "A2".to_string())]),
        );

        assert!(contains(&html, "import-copy-name"));
        assert!(!is_save_disabled(&html));
    }

    #[test]
    fn test_the_week_of_a_schedule_is_previewed() {
        let html = render_preview(
            "Schedule\n\nMonday\n- A\n- Rotation — R\n\nRotation — R\n- A",
            &[],
            BTreeMap::new(),
        );

        assert_eq!(all_text_of(&html, "import-slot"), vec!["A", "R"]);
    }

    #[test]
    fn test_no_week_is_previewed_without_schedule() {
        let html = render_preview("Routine — A", &[], BTreeMap::new());

        assert!(!contains(&html, "import-week"));
    }

    #[test]
    fn test_the_exercise_picker_names_the_exercise_of_the_text() {
        let html = render(|| {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Squat")])
                .with_training_sessions(vec![])
                .provide();
            rsx! { PickerDialog {} }
        });

        assert!(
            html.contains("Exercise for \u{201c}Mystery\u{201d}"),
            "{html}"
        );
        assert_eq!(all_text_of(&html, "exercise-item"), vec!["Squat"]);
    }

    #[component]
    fn PickerDialog() -> Element {
        let preview = use_signal(|| None::<Preview>);
        view_exercise_picker(preview, domain::Name::new("Mystery").unwrap())
    }

    fn render_preview(
        text: &'static str,
        exercise_names: &'static [&'static str],
        copies: BTreeMap<domain::Name, String>,
    ) -> String {
        render_preview_with(text.to_string(), exercise_names, copies, BTreeMap::new())
    }

    fn render_preview_with(
        text: String,
        exercise_names: &'static [&'static str],
        copies: BTreeMap<domain::Name, String>,
        chosen: BTreeMap<domain::Name, domain::ExerciseChoice>,
    ) -> String {
        render(move || {
            let exercises = exercise_names
                .iter()
                .zip(1..)
                .map(|(name, id)| exercise(id, name))
                .collect::<Vec<_>>();
            TestCache::default()
                .with_exercises(exercises)
                .with_routines(vec![routine(1, "A", ""), routine(2, "D", "Notes")])
                .provide();
            rsx! { PreviewDialog { text: text.clone(), copies: copies.clone(), chosen: chosen.clone() } }
        })
    }

    #[component]
    fn PreviewDialog(
        text: String,
        copies: BTreeMap<domain::Name, String>,
        chosen: BTreeMap<domain::Name, domain::ExerciseChoice>,
    ) -> Element {
        let cache = consume_context::<Cache>();
        let (CacheState::Ready(exercises), CacheState::Ready(routines)) =
            (&*cache.exercises.read(), &*cache.routines.read())
        else {
            return rsx! {};
        };
        let preview = use_signal(|| {
            let mut preview = Preview {
                copies: copies.clone(),
                ..Preview::new(domain::parse_text(&text).unwrap(), exercises)
            };
            for (name, choice) in &mut preview.exercises {
                *choice = chosen.get(name).cloned();
            }
            Some(preview)
        });
        let is_loading = use_signal(|| false);
        view_preview(
            preview,
            is_loading,
            exercises,
            routines,
            EventHandler::new(|()| {}),
        )
    }

    fn is_save_disabled(html: &str) -> bool {
        all_attributes_of(html, "dialog-save", "disabled") == vec!["true"]
    }

    fn routine(id: u128, name: &str, notes: &str) -> domain::Routine {
        domain::Routine {
            id: id.into(),
            name: domain::Name::new(name).unwrap(),
            notes: notes.to_string(),
            archived: false,
            sections: vec![],
        }
    }

    fn exercise(id: u128, name: &str) -> domain::Exercise {
        domain::Exercise {
            id: id.into(),
            name: domain::Name::new(name).unwrap(),
            notes: String::new(),
            muscles: vec![],
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }
    }
}
