use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use dioxus::prelude::*;
use indexmap::IndexMap;

use valens_domain::{self as domain, SessionService, TrainingSessionService};
use valens_web_app::{self as web_app, OngoingTrainingSessionService, SettingsService};

use crate::{
    DOMAIN_SERVICE, METRONOME, NOTIFICATIONS, Route, WEB_APP_SERVICE,
    cache::{Cache, CacheState},
    eh, ensure_session,
    page::{
        self,
        common::{SetsPerMuscle, Timer, TimerService},
    },
    ui::{
        element::{
            Block, CenteredBlock, Dialog, ErrorMessage, FloatingActionButton, Icon, Loading,
            LoadingDialog, LoadingPage, MenuOption, NoConnection, OptionsMenu, Title,
        },
        form::{Field, FieldValue, FieldValueState, InputField},
    },
    unsaved_changes::{UnsavedChangesDialog, use_unsaved_changes},
};

static IS_LOADING: GlobalSignal<bool> = Signal::global(|| false);

#[component]
pub fn TrainingSession(id: domain::TrainingSessionID) -> Element {
    ensure_session!();

    let mut edit = use_signal(|| false);
    let mut progress = use_store(|| Progress::new(id));

    let cache = consume_context::<Cache>();
    let training_session = use_memo(move || {
        if let CacheState::Ready(training_sessions) = &*cache.training_sessions.read() {
            let training_session = training_sessions.iter().find(|e| e.id == id).cloned();
            if let Some(training_session) = &training_session {
                if training_session.is_empty() && !progress.read().is_active() {
                    edit.set(true);
                    progress.write().set_element_idx(0);
                }
            }
            training_session
        } else {
            None
        }
    });
    let routine = use_memo(move || {
        if let Some(training_session) = &*training_session.read() {
            if let CacheState::Ready(routines) = &*cache.routines.read() {
                routines
                    .iter()
                    .find(|e| e.id == training_session.routine_id)
                    .cloned()
            } else {
                None
            }
        } else {
            None
        }
    });

    use_future(move || async move {
        let ongoing_training_session = WEB_APP_SERVICE.read().get_ongoing_training_session().await;
        if let Ok(Some(ongoing_training_session)) = ongoing_training_session {
            if ongoing_training_session.training_session_id == id.as_u128() {
                progress.set(Progress::from(ongoing_training_session));
                edit.set(true);
            }
        }
    });
    use_effect(move || {
        if progress.read().is_active() {
            spawn(async move {
                let _ = WEB_APP_SERVICE
                    .read()
                    .set_ongoing_training_session(
                        if progress.read().element_idx
                            < training_session
                                .read()
                                .as_ref()
                                .map_or(usize::MAX, |training_session| {
                                    training_session.elements.len()
                                })
                        {
                            Some(web_app::OngoingTrainingSession::from(
                                (*progress.read()).clone(),
                            ))
                        } else {
                            None
                        },
                    )
                    .await;
            });
        }
    });

    let settings = use_resource(|| async { WEB_APP_SERVICE.read().get_settings().await });
    use_effect(move || {
        if let Some(Ok(settings)) = settings.read().as_ref() {
            progress
                .timer_service()
                .write()
                .set_beep_volume(settings.beep_volume);
        }
    });

    let mut field_values = use_signal(HashMap::new);
    use_memo(move || {
        if let Some(training_session) = training_session() {
            field_values.set(
                training_session
                    .elements
                    .iter()
                    .cloned()
                    .enumerate()
                    .filter_map(|(idx, element)| {
                        if let domain::TrainingSessionElement::Set {
                            reps,
                            time,
                            weight,
                            rpe,
                            ..
                        } = element
                        {
                            Some((
                                idx,
                                SetFieldValues {
                                    reps: FieldValue::new_with_empty_default(
                                        reps.unwrap_or_default(),
                                    ),
                                    time: FieldValue::new_with_empty_default(
                                        time.unwrap_or_default(),
                                    ),
                                    weight: FieldValue::new_with_empty_default(
                                        weight.unwrap_or_default(),
                                    ),
                                    rpe: FieldValue::new_with_empty_default(
                                        rpe.unwrap_or_default(),
                                    ),
                                },
                            ))
                        } else {
                            None
                        }
                    })
                    .collect(),
            );
        }
    });

    let mut has_unsaved_changes = use_unsaved_changes();
    use_effect(move || {
        has_unsaved_changes.set(field_values.read().iter().any(|(_, v)| v.changed()));
    });

    use_effect(move || {
        let element_idx = progress.read().element_idx;
        if let Some(training_session) = training_session() {
            if let Some(element) = training_session.elements.get(element_idx) {
                let automatic_metronome = settings
                    .read()
                    .as_ref()
                    .and_then(|s| s.as_ref().ok())
                    .is_some_and(|s| s.automatic_metronome);
                match element {
                    domain::TrainingSessionElement::Set {
                        target_reps,
                        target_time,
                        automatic,
                        ..
                    } => {
                        if automatic_metronome {
                            METRONOME.write().pause();
                        }
                        if let Some(target_time) = target_time {
                            if automatic_metronome && target_reps.is_some() {
                                METRONOME.with_mut(|metronome| {
                                    metronome.set_interval((*target_time).into());
                                    metronome.set_stressed_beat(1);
                                    metronome.start();
                                });
                            }
                            if progress.timer_service().read().is_set() {
                                if progress.timer_service().read().seconds() <= 0 {
                                    progress.write().set_element_idx(element_idx + 1);
                                    progress.timer_service().write().unset();
                                    if let Some(set_field_values) =
                                        field_values.write().get_mut(&element_idx)
                                    {
                                        set_field_values.time.validated = Ok(*target_time);
                                    }
                                    spawn(async move {
                                        let mut training_session = training_session.clone();
                                        modify_training_session_elements(
                                            &mut training_session,
                                            field_values,
                                        );
                                        save(training_session, cache, || {}).await;
                                    });
                                }
                            } else {
                                progress
                                    .timer_service()
                                    .write()
                                    .set(i64::from(*target_time));
                                if *automatic {
                                    progress.timer_service().write().start();
                                }
                            }
                        }
                    }
                    domain::TrainingSessionElement::Rest {
                        target_time,
                        automatic,
                    } => {
                        if automatic_metronome {
                            METRONOME.write().pause();
                        }
                        if let Some(target_time) = target_time {
                            if progress.timer_service().read().is_set() {
                                if *automatic && progress.timer_service().read().seconds() <= 0 {
                                    progress.write().set_element_idx(element_idx + 1);
                                    progress.timer_service().write().unset();
                                }
                            } else {
                                progress
                                    .timer_service()
                                    .write()
                                    .set(i64::from(*target_time));
                                progress.timer_service().write().start();
                            }
                        } else if *automatic {
                            progress.write().set_element_idx(element_idx + 1);
                        }
                    }
                }
            }
        }
    });

    let edit_dialog = use_signal(|| EditDialog::None);

    let mut notes =
        use_signal(move || training_session().map(|ts| FieldValue::new(ts.notes.clone())));
    use_effect(move || {
        if let Some(training_session) = training_session.read().as_ref() {
            notes.with_mut(|notes| {
                if notes.is_none()
                    || notes
                        .as_ref()
                        .is_some_and(|notes| notes.input == training_session.notes)
                {
                    *notes = Some(FieldValue::new(training_session.notes.clone()));
                }
            });
        }
    });

    let has_changes = use_memo(move || {
        notes.read().as_ref().is_some_and(FieldValueState::changed)
            || field_values
                .read()
                .iter()
                .any(|(_, f)| f.has_valid_changes())
    });

    match (
        &*cache.training_sessions.read(),
        &*training_session.read(),
        &*cache.exercises.read(),
        &*settings.read(),
    ) {
        (
            CacheState::Ready(_),
            Some(training_session),
            CacheState::Ready(exercises),
            Some(Ok(settings)),
        ) => {
            rsx! {
                Title { "{training_session.date}" }
                if let Some(routine) = &*routine.read() {
                    Block {
                        Title {
                            class: "has-text-link",
                            Link {
                                to: Route::Routine { id: routine.id },
                                "{routine.name}"
                            }
                        }
                    }
                }
                if edit() {
                    {view_form(field_values, progress, edit_dialog, training_session, exercises, *settings, cache)},
                } else {
                    {view_list(training_session, exercises, *settings)},
                    {view_muscles(training_session, exercises)}
                }
                Notes { notes, edit },
                {view_edit_dialog(edit_dialog, field_values, cache)}
                FloatingActionButton {
                    icon: (if edit() { if has_changes() { "save" } else { "eye" } } else { "edit" }).to_string(),
                    onclick: eh!(mut edit, training_session; {
                        if edit() && has_changes() {
                            modify_training_session_elements(&mut training_session, field_values);
                            training_session.notes = notes.read().as_ref().unwrap().validated.clone().unwrap();
                            spawn(async move {
                                save(training_session.clone(), cache, || {}).await;
                            });
                        } else {
                            edit.toggle();
                            if edit() && !progress.read().is_active() {
                                progress.write().set_element_idx(0);
                            }
                        }
                    }),
                    is_loading: IS_LOADING(),
                }
                UnsavedChangesDialog { }
            }
        }
        (CacheState::Ready(_), None, _, _) => rsx! {
            ErrorMessage { message: "Training session not found" }
        },
        (
            CacheState::Error(domain::ReadError::Storage(domain::StorageError::NoConnection)),
            _,
            _,
            _,
        ) => {
            rsx! { NoConnection {  } {} }
        }
        (CacheState::Error(err), _, _, _) | (_, _, CacheState::Error(err), _) => {
            rsx! { ErrorMessage { message: err } }
        }
        (_, _, _, Some(Err(err))) => {
            rsx! { ErrorMessage { message: err } }
        }
        (CacheState::Loading, _, _, _) | (_, _, CacheState::Loading, _) | (_, _, _, None) => {
            rsx! { LoadingPage {} }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct SetFieldValues {
    reps: FieldValue<domain::Reps>,
    time: FieldValue<domain::Time>,
    weight: FieldValue<domain::Weight>,
    rpe: FieldValue<domain::RPE>,
}

impl SetFieldValues {
    fn valid(&self) -> bool {
        self.reps.valid() && self.time.valid() && self.weight.valid() && self.rpe.valid()
    }

    fn changed(&self) -> bool {
        self.reps.changed() || self.time.changed() || self.weight.changed() || self.rpe.changed()
    }

    fn has_valid_changes(&self) -> bool {
        FieldValue::has_valid_changes(&[&self.reps, &self.time, &self.weight, &self.rpe])
    }

    fn is_empty(&self) -> bool {
        self.reps.input.is_empty()
            && self.time.input.is_empty()
            && self.weight.input.is_empty()
            && self.rpe.input.is_empty()
    }
}

fn view_form(
    mut field_values: Signal<HashMap<usize, SetFieldValues>>,
    mut progress: Store<Progress>,
    mut edit_dialog: Signal<EditDialog>,
    training_session: &domain::TrainingSession,
    exercises: &[domain::Exercise],
    settings: web_app::Settings,
    cache: Cache,
) -> Element {
    let mut element_idx: usize = 0;
    let sections = training_session.compute_sections();
    let training_sessions_cache = &*cache.training_sessions.read();
    let sets_by_exercise = DOMAIN_SERVICE().get_sets_by_exercise(training_session);
    let previous_session_sets_by_exercise = {
        if let CacheState::Ready(training_sessions) = training_sessions_cache {
            DOMAIN_SERVICE()
                .get_previous_session_sets_by_exercise(training_session, training_sessions)
        } else {
            HashMap::new()
        }
    };
    let mut set_index_for_exercise: HashMap<domain::ExerciseID, usize> = HashMap::new();
    let rows = sections.iter().enumerate().map(|(section_idx, section)| {
        let is_current_section = progress.read().element_idx >= element_idx.checked_sub(1).unwrap_or_default() && progress.read().element_idx < element_idx + section.elements().len();
        let exercise_ids = unique(section.exercise_ids());
        let exercise_ids_len = exercise_ids.len();
        let element_idx_for_options = element_idx;
        let exercise_names = exercise_ids.iter().enumerate().map(|(i, id)| {
            let name = exercise_name(*id, exercises);
            let number = exercise_number(id, &exercise_ids);
            rsx! {
                tr {
                    class: if is_current_section { "" } else { "is-semitransparent" },
                    td {
                        class: "has-text-centered has-text-weight-bold",
                        class: if i == 0 { "pt-2" },
                        class: if i == exercise_ids_len - 1 { "pb-1" },
                        colspan: 6,
                        if let Some(number) = number {
                            span{
                                class: "px-1",
                                "{circled_number(number)}"
                            }
                        }
                        Link {
                            class: "px-1",
                            to: Route::Exercise { id: *id },
                            "{name}"
                        }
                        a {
                            class: "px-1 is-link",
                            onclick: eh!(training_session, element_idx_for_options; {
                                *edit_dialog.write() = EditDialog::Options {
                                    training_session: training_session.clone(),
                                    section_idx,
                                    element_idx: element_idx_for_options,
                                    exercise_idx: i,
                                }
                            }),
                            Icon { name: "ellipsis-vertical" }
                        }
                    }
                }
            }
        });

        let exercise_counts = section.exercise_counts();

        let sets = section.elements().iter().map(|element| {
            let set = match element {
                domain::TrainingSessionElement::Set { exercise_id, target_reps, target_time, target_weight, target_rpe, .. } => {
                    let set_index = *set_index_for_exercise.entry(*exercise_id).or_default();
                    let set_field_values = &field_values.read()[&element_idx];

                    let mut set_buttons: IndexMap<domain::Set, Vec<String>> = IndexMap::new();
                    if is_current_section && set_field_values.is_empty() || set_field_values.changed() {
                        if target_reps.is_some() || target_time.is_some() || target_weight.is_some() || target_rpe.is_some() {
                            set_buttons.entry(domain::Set {
                                reps: target_reps.unwrap_or_default(),
                                time: target_time.unwrap_or_default(),
                                weight: target_weight.unwrap_or_default(),
                                rpe: target_rpe.unwrap_or_default(),
                            }).or_default().push("bullseye".to_string());
                        }
                        let previous_set = set_index.checked_sub(*exercise_counts.get(exercise_id).unwrap_or(&1)).and_then(|previous_set_index| sets_by_exercise.get(exercise_id).and_then(|set| set.get(previous_set_index).map(|e| (**e).clone())));
                        if let Some(domain::TrainingSessionElement::Set { reps, time, weight, rpe, .. }) = previous_set {
                            set_buttons.entry(domain::Set {
                                reps: reps.unwrap_or_default(),
                                time: time.unwrap_or_default(),
                                weight: weight.unwrap_or_default(),
                                rpe: rpe.unwrap_or_default(),
                            }).or_default().push("arrow-turn-down".to_string());
                        }
                        let previous_session_set = previous_session_sets_by_exercise.get(exercise_id).and_then(|set| set.get(set_index).map(|e| (**e).clone()));
                        if let Some(domain::TrainingSessionElement::Set { reps, time, weight, rpe, .. }) = previous_session_set {
                            set_buttons.entry(domain::Set {
                                reps: reps.unwrap_or_default(),
                                time: time.unwrap_or_default(),
                                weight: weight.unwrap_or_default(),
                                rpe: rpe.unwrap_or_default(),
                            }).or_default().push("calendar-minus".to_string());
                        }
                    }

                    let set = if set_field_values.is_empty() && !set_field_values.changed() && target_reps.is_none() && target_time.is_some() {
                        if let Some(target_time) = target_time {
                            rsx! {
                                tr {
                                    class: if is_current_section { "" } else { "is-semitransparent" },
                                    td { }
                                    td {
                                        class: "p-1",
                                        colspan: 4,
                                        div {
                                            class: "notification is-link has-text-centered px-6 py-1",
                                            class: if progress.read().element_idx == element_idx { "is-size-1" },
                                            if progress.read().element_idx == element_idx {
                                                Timer { timer: progress.timer_service() }
                                            } else {
                                                div {
                                                    onclick: move |_| {
                                                        progress.write().set_element_idx(element_idx);
                                                        progress.timer_service().write().unset();
                                                    },
                                                    "{target_time} s"
                                                }
                                            }
                                        }
                                    }
                                    td {
                                        class: "p-1",
                                        style: "vertical-align: middle",
                                        button {
                                            class: "button is-small",
                                            class: if progress.read().element_idx == element_idx { "is-link is-outlined" },
                                            onclick: eh!(mut training_session; target_time; {
                                                if progress.read().element_idx == element_idx {
                                                    progress.write().set_element_idx(element_idx + 1);
                                                    progress.timer_service().write().unset();
                                                    if let Some(set_field_values) = field_values.write().get_mut(&element_idx) {
                                                        set_field_values.time.validated = Ok(target_time);
                                                    }
                                                    modify_training_session_elements(&mut training_session, field_values);
                                                    spawn(async move {
                                                        save(training_session.clone(), cache, || {}).await;
                                                    });
                                                } else {
                                                    progress.write().set_element_idx(element_idx);
                                                    progress.timer_service().write().unset();
                                                }
                                            }),
                                            Icon { name: if progress.read().element_idx == element_idx { "check" } else { "angles-left" } }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    } else {
                        let number = exercise_number(exercise_id, &exercise_ids);
                        rsx! {
                            tr {
                                class: if is_current_section { "" } else { "is-semitransparent" },
                                td {
                                    class: "p-1",
                                    style: "vertical-align: middle",
                                    if let Some(number) = number {
                                        "{circled_number(number)}"
                                    }
                                }
                                td {
                                    class: "p-1 has-text-right",
                                    InputField {
                                        right_icon: rsx! { "✕" },
                                        r#type: "number",
                                        min: "0",
                                        max: "999",
                                        step: 1,
                                        size: 2,
                                        value: set_field_values.reps.input.clone(),
                                        error: if let Err(err) = &set_field_values.reps.validated { err.clone() },
                                        has_changed: set_field_values.reps.changed(),
                                        has_text_right: true,
                                        oninput: move |event: FormEvent| {
                                            async move {
                                                if let Some(set_field_values) = field_values.write().get_mut(&element_idx) {
                                                    set_field_values.reps.input = event.value();
                                                    set_field_values.reps.validated = if event.value().is_empty() {
                                                        Ok(domain::Reps::default())
                                                    } else {
                                                        domain::Reps::try_from(event.value().as_ref()).map_err(|err| err.to_string())
                                                    };
                                                }
                                            }
                                        },
                                    }
                                }
                                td {
                                    class: "p-1 has-text-right",
                                    if settings.show_tut {
                                        InputField {
                                            right_icon: rsx! { "s" },
                                            r#type: "number",
                                            min: "0",
                                            max: "999",
                                            step: 1,
                                            size: 2,
                                            value: set_field_values.time.input.clone(),
                                            error: if let Err(err) = &set_field_values.time.validated { err.clone() },
                                            has_changed: set_field_values.time.changed(),
                                            has_text_right: true,
                                            oninput: move |event: FormEvent| {
                                                async move {
                                                    if let Some(set_field_values) = field_values.write().get_mut(&element_idx) {
                                                        set_field_values.time.input = event.value();
                                                        set_field_values.time.validated = if event.value().is_empty() {
                                                            Ok(domain::Time::default())
                                                        } else {
                                                            domain::Time::try_from(event.value().as_ref()).map_err(|err| err.to_string())
                                                        };
                                                    }
                                                }
                                            },
                                        }
                                    }
                                }
                                td {
                                    class: "p-1 has-text-right",
                                    InputField {
                                        right_icon: rsx! { "kg" },
                                        inputmode: "numeric",
                                        size: 3,
                                        value: set_field_values.weight.input.clone(),
                                        error: if let Err(err) = &set_field_values.weight.validated { err.clone() },
                                        has_changed: set_field_values.weight.changed(),
                                        has_text_right: true,
                                        oninput: move |event: FormEvent| {
                                            async move {
                                                if let Some(set_field_values) = field_values.write().get_mut(&element_idx) {
                                                    set_field_values.weight.input = event.value();
                                                    set_field_values.weight.validated = if event.value().is_empty() {
                                                        Ok(domain::Weight::default())
                                                    } else {
                                                        domain::Weight::try_from(event.value().as_ref()).map_err(|err| err.to_string())
                                                    };
                                                }
                                            }
                                        },
                                    }
                                }
                                td {
                                    class: "p-1",
                                    if settings.show_rpe {
                                        InputField {
                                            left_icon: rsx! { "@" },
                                            inputmode: "numeric",
                                            size: 2,
                                            value: set_field_values.rpe.input.clone(),
                                            error: if let Err(err) = &set_field_values.rpe.validated { err.clone() },
                                            has_changed: set_field_values.rpe.changed(),
                                            has_text_right: true,
                                            oninput: move |event: FormEvent| {
                                                async move {
                                                    if let Some(set_field_values) = field_values.write().get_mut(&element_idx) {
                                                        set_field_values.rpe.input = event.value();
                                                        set_field_values.rpe.validated = if event.value().is_empty() {
                                                            Ok(domain::RPE::default())
                                                        } else {
                                                            domain::RPE::try_from(event.value().as_ref()).map_err(|err| err.to_string())
                                                        };
                                                    }
                                                }
                                            },
                                        }
                                    }
                                }
                                td {
                                    class: "p-1",
                                    style: "vertical-align: middle",
                                    if set_field_values.valid() && !(set_field_values.is_empty() && !set_field_values.changed() && progress.read().element_idx == element_idx) {
                                        button {
                                            class: "button is-small",
                                            class: if set_field_values.has_valid_changes() { "is-link is-outlined" } else if !set_field_values.is_empty() { "is-ghost" },
                                            onclick: eh!(mut training_session; field_values, set_field_values; {
                                                if set_field_values.is_empty() && !set_field_values.changed() {
                                                    progress.write().set_element_idx(element_idx);
                                                } else {
                                                    progress.write().set_element_idx(element_idx + 1);
                                                    progress.timer_service().write().unset();
                                                    modify_training_session_elements(&mut training_session, field_values);
                                                    spawn(async move {
                                                        save(training_session.clone(), cache, || {}).await;
                                                    });
                                                }
                                            }),
                                            Icon { name: if set_field_values.is_empty() && !set_field_values.changed() { "angles-left" } else { "check" } }
                                        }
                                    }
                                }
                            }
                            if is_current_section {
                                tr {
                                    td { }
                                    td {
                                        class: "p-1 has-text-centered",
                                        colspan: 4,
                                        for (set, icons) in set_buttons {
                                            button {
                                                class: "button is-small mr-2",
                                                onclick: eh!(mut field_values; set; {
                                                    if let Some(set_field_values) = field_values.write().get_mut(&element_idx) {
                                                        let reps = set.reps;
                                                        set_field_values.reps.input = if reps == domain::Reps::default() { String::new() } else { reps.to_string() };
                                                        set_field_values.reps.validated = Ok(reps);
                                                        let time = set.time;
                                                        set_field_values.time.input = if time == domain::Time::default() { String::new() } else { time.to_string() };
                                                        set_field_values.time.validated = Ok(time);
                                                        let weight = set.weight;
                                                        set_field_values.weight.input = if weight == domain::Weight::default() { String::new() } else { weight.to_string() };
                                                        set_field_values.weight.validated = Ok(weight);
                                                        let rpe = set.rpe;
                                                        set_field_values.rpe.input = if rpe == domain::RPE::default() { String::new() } else { rpe.to_string() };
                                                        set_field_values.rpe.validated = Ok(rpe);
                                                    }
                                                }),
                                                Icon { name: icons[0].clone(), is_small: true },
                                                span { {set.to_string(settings.show_tut, settings.show_rpe)} },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    };
                    set_index_for_exercise.entry(*exercise_id).and_modify(|i| *i += 1);
                    set
                }
                domain::TrainingSessionElement::Rest { target_time, .. } => {
                    rsx! {
                        tr {
                            class: if is_current_section { "" } else { "is-semitransparent" },
                            if progress.read().element_idx == element_idx {
                                td { }
                                td {
                                    class: "p-1",
                                    colspan: 4,
                                    div {
                                        class: "notification is-success is-size-1 has-text-centered p-1",
                                        if target_time.is_some() {
                                            Timer { timer: progress.timer_service() }
                                        } else {
                                            "Rest"
                                        }
                                    }
                                }
                                td {
                                    class: "p-1",
                                    style: "vertical-align: middle",
                                    button {
                                        class: "button is-small",
                                        onclick: move |_| {
                                            progress.write().set_element_idx(element_idx + 1);
                                            progress.timer_service().write().unset();
                                        },
                                        Icon { name: "check" }
                                    }
                                }
                            } else {
                                td { }
                                td {
                                    class: "p-1",
                                    colspan: 4,
                                    div {
                                        class: "notification p-0 is-size-7 has-background-auto-text-95 has-text-centered",
                                        onclick: move |_| {
                                            progress.write().set_element_idx(element_idx);
                                            progress.timer_service().write().unset();
                                        },
                                        if let Some(target_time) = target_time {
                                            "{target_time} s"
                                        } else {
                                            "Rest"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            };
            element_idx += 1;
            set
        });
        rsx! {
            for name in exercise_names {
                {name}
            }
            for set in sets {
                {set}
            }
        }
    });

    rsx! {
        Block {
            table {
                class: "mx-auto",
                for row in rows {
                    {row}
                }
            }
        }
        Block {
            div {
                class: "has-text-centered",
                button {
                    class: "button is-small is-white-soft",
                    onclick: eh!(mut edit_dialog; training_session; {
                        *edit_dialog.write() = EditDialog::AppendExercise { training_session };
                    }),
                    Icon { name: "plus" }
                }
            }
        }
    }
}

#[component]
fn Notes(notes: Signal<Option<FieldValue<String>>>, edit: ReadSignal<bool>) -> Element {
    let Some((changed, input, orig)) = notes
        .read()
        .as_ref()
        .map(|n| (n.changed(), n.input.clone(), n.orig.clone()))
    else {
        return rsx! { Loading { } };
    };

    if edit() {
        rsx! {
            CenteredBlock {
                class: "px-2",
                Title { "Notes" },
                Field {
                    label: "",
                    textarea {
                        class: "textarea",
                        class: if changed { "is-info" },
                        oninput: {
                            move |event| {
                                notes.with_mut(|n| if let Some(n) = n.as_mut() {
                                    n.input = event.value();
                                    n.validated = Ok(event.value());
                                });
                            }
                        },
                        { input },
                    }
                }
            }
        }
    } else {
        rsx! {
            if !orig.is_empty() {
                CenteredBlock {
                    Title { "Notes" },
                    p { { orig } }
                }
            }
        }
    }
}

fn view_list(
    training_session: &domain::TrainingSession,
    exercises: &[domain::Exercise],
    settings: web_app::Settings,
) -> Element {
    let sections = training_session.compute_sections();
    let rows = sections.iter().map(|section| {
        let exercise_ids = unique(section.exercise_ids());
        let exercise_ids_len = exercise_ids.len();
        let exercise_names = exercise_ids.clone().into_iter().enumerate().map(|(i, id)| {
            let name = exercise_name(id, exercises);
            let number = exercise_number(&id, &exercise_ids);
            rsx! {
                tr {
                    td {
                        class: "has-text-centered has-text-weight-bold",
                        class: if i == 0 { "pt-2" },
                        class: if i == exercise_ids_len - 1 { "pb-1" },
                        colspan: 5,
                        if let Some(number) = number {
                            span{
                                class: "px-1",
                                "{circled_number(number)}"
                            }
                        }
                        Link {
                            class: "px-1",
                            to: Route::Exercise { id },
                            "{name}"
                        }
                    }
                }
            }
        });

        let sets = section.elements().iter().map(|element| {
            rsx! {
                match element {
                    domain::TrainingSessionElement::Set { exercise_id, reps, time, weight, rpe, .. } => {
                        let number = exercise_number(exercise_id, &exercise_ids);
                        rsx! {
                            tr {
                                if reps.is_none() && (time.is_none() || !settings.show_tut) && weight.is_none() && (rpe.is_none() || !settings.show_rpe) {
                                    td {
                                        class: "px-2 has-text-centered",
                                        colspan: 5,
                                        if let Some(number) = number {
                                            span {
                                                class: "pr-2",
                                                "{circled_number(number)} "
                                            }
                                        }
                                        span {
                                            class: if number.is_some() { "pr-5" },
                                            "–"
                                        }
                                    }
                                } else {
                                    td {
                                        class: "px-2 has-text-centered",
                                        if let Some(number) = number {
                                            "{circled_number(number)}"
                                        }
                                    }
                                    td {
                                        class: "px-2 has-text-right",
                                        if let Some(reps) = reps {
                                            if *reps > domain::Reps::default() {
                                                "{reps} ×"
                                            }
                                        }
                                    }
                                    td {
                                        class: "px-2 has-text-right",
                                        if let Some(time) = time {
                                            if settings.show_tut && *time > domain::Time::default() {
                                                "{time} s"
                                            }
                                        }
                                    }
                                    td {
                                        class: "px-2 has-text-right",
                                        if let Some(weight) = weight {
                                            if *weight > domain::Weight::default() {
                                                "{weight} kg"
                                            }
                                        }
                                    }
                                    td {
                                        class: "px-2",
                                        if let Some(rpe) = rpe {
                                            if settings.show_rpe && *rpe > domain::RPE::ZERO {
                                                " @ {rpe}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    domain::TrainingSessionElement::Rest { .. } => {
                        rsx! {
                            tr {
                                td {
                                    class: "p-1",
                                }
                            }
                        }
                    }
                }
            }
        });
        rsx! {
            for name in exercise_names {
                {name}
            }
            for set in sets {
                {set}
            }
        }
    });

    rsx! {
        Block {
            table {
                class: "mx-auto",
                for row in rows {
                    {row}
                }
            }
        }
    }
}

fn view_muscles(
    training_session: &domain::TrainingSession,
    exercises: &[domain::Exercise],
) -> Element {
    let stimulus_per_muscle = training_session.stimulus_per_muscle(exercises);
    if stimulus_per_muscle.is_empty() {
        rsx! {}
    } else {
        rsx! {
            CenteredBlock {
                Title { "Hard sets per muscle" },
                SetsPerMuscle { stimulus_per_muscle: stimulus_per_muscle.clone() }
            }
        }
    }
}

fn view_edit_dialog(
    mut edit_dialog: Signal<EditDialog>,
    field_values: Signal<HashMap<usize, SetFieldValues>>,
    cache: Cache,
) -> Element {
    let close_dialog = move || {
        *edit_dialog.write() = EditDialog::None;
    };

    match &*edit_dialog.read() {
        EditDialog::None => rsx! {},
        EditDialog::Options {
            training_session,
            section_idx,
            element_idx,
            exercise_idx,
        } => {
            rsx! {
                if IS_LOADING() {
                    LoadingDialog { }
                } else {
                    OptionsMenu {
                        options: vec![
                            rsx! {
                                MenuOption {
                                    icon: "plus".to_string(),
                                    text: "Add set".to_string(),
                                    onclick: eh!(mut training_session; element_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, field_values);
                                        training_session.add_set(element_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "plus".to_string(),
                                    text: "Add same exercise".to_string(),
                                    onclick: eh!(mut training_session; section_idx, exercise_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, field_values);
                                        training_session.add_same_exercise(section_idx, exercise_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "plus".to_string(),
                                    text: "Add other exercise".to_string(),
                                    onclick: eh!(mut edit_dialog; training_session, section_idx; {
                                        *edit_dialog.write() = EditDialog::AddExercise { training_session, section_idx };
                                    })
                                },
                                MenuOption {
                                    icon: "arrow-up".to_string(),
                                    text: "Move up".to_string(),
                                    onclick: eh!(mut training_session; section_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, field_values);
                                        training_session.move_section_up(section_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "arrow-down".to_string(),
                                    text: "Move down".to_string(),
                                    onclick: eh!(mut training_session; section_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, field_values);
                                        training_session.move_section_down(section_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "arrow-right-arrow-left".to_string(),
                                    text: "Replace exercise".to_string(),
                                    onclick: eh!(mut edit_dialog; training_session, section_idx, exercise_idx; {
                                        *edit_dialog.write() = EditDialog::ReplaceExercise { training_session, section_idx, exercise_idx };
                                    })
                                },
                                MenuOption {
                                    icon: "times".to_string(),
                                    text: "Remove set".to_string(),
                                    onclick: eh!(mut training_session; section_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, field_values);
                                        training_session.remove_set(section_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "times".to_string(),
                                    text: "Remove exercise".to_string(),
                                    onclick: eh!(mut training_session; section_idx, exercise_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, field_values);
                                        training_session.remove_exercise(section_idx, exercise_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                            },
                        ],
                        close_event: eh!(mut close_dialog; { close_dialog(); })
                    }
                }
            }
        }
        EditDialog::AddExercise {
            training_session,
            section_idx,
        } => {
            rsx! {
                if IS_LOADING() {
                    LoadingDialog { }
                } else {
                    Dialog {
                        title: rsx! { "Add exercise" },
                        close_event: eh!(mut close_dialog; { close_dialog(); }),
                        page::exercises::ExerciseList {
                            add: false,
                            filter: String::new(),
                            change_route: false,
                            exercise_onclick: {
                                let training_session = training_session.clone();
                                let section_idx = *section_idx;
                                move |(_, exercise_id)| {
                                    let mut training_session = training_session.clone();
                                    modify_training_session_elements(&mut training_session, field_values);
                                    training_session.add_exercise(section_idx, exercise_id);
                                    save(training_session, cache, close_dialog)
                                }
                            },
                            catalog_onclick: |_| {}
                        }
                    }
                }
            }
        }
        EditDialog::ReplaceExercise {
            training_session,
            section_idx,
            exercise_idx,
        } => {
            rsx! {
                if IS_LOADING() {
                    LoadingDialog { }
                } else {
                    Dialog {
                        close_event: eh!(mut close_dialog; { close_dialog(); }),
                        message_body_class: "px-0",
                        page::exercises::ExerciseList {
                            add: false,
                            filter: String::new(),
                            change_route: false,
                            exercise_onclick: {
                                let training_session = training_session.clone();
                                let section_idx = *section_idx;
                                let exercise_idx = *exercise_idx;
                                move |(_, exercise_id)| {
                                    let mut training_session = training_session.clone();
                                    modify_training_session_elements(&mut training_session, field_values);
                                    training_session.replace_exercise(section_idx, exercise_idx, exercise_id);
                                    save(training_session, cache, close_dialog)
                                }
                            },
                            catalog_onclick: |_| {}
                        }
                    }
                }
            }
        }
        EditDialog::AppendExercise { training_session } => {
            rsx! {
                if IS_LOADING() {
                    LoadingDialog { }
                } else {
                    Dialog {
                        title: rsx! { "Append exercise" },
                        close_event: eh!(mut close_dialog; { close_dialog(); }),
                        page::exercises::ExerciseList {
                            add: false,
                            filter: String::new(),
                            change_route: false,
                            exercise_onclick: {
                                let training_session = training_session.clone();
                                move |(_, exercise_id)| {
                                    let mut training_session = training_session.clone();
                                    modify_training_session_elements(&mut training_session, field_values);
                                    training_session.append_exercise(exercise_id);
                                    save(training_session, cache, close_dialog)
                                }
                            },
                            catalog_onclick: |_| {}
                        }
                    }
                }
            }
        }
    }
}

fn unique<T: Copy + Eq + Hash>(mut vec: Vec<T>) -> Vec<T> {
    let mut seen = HashSet::new();
    vec.retain(|id| seen.insert(*id));
    vec
}

fn exercise_name(exercise_id: domain::ExerciseID, exercises: &[domain::Exercise]) -> String {
    exercises
        .iter()
        .find(|exercise| exercise.id == exercise_id)
        .map(|exercise| exercise.name.to_string())
        .unwrap_or(format!("Exercise#{}", exercise_id.as_u128()))
}

fn exercise_number(
    exercise_id: &domain::ExerciseID,
    exercise_ids: &[domain::ExerciseID],
) -> Option<u32> {
    if exercise_ids.len() > 1 {
        exercise_ids
            .iter()
            .position(|id| id == exercise_id)
            .and_then(|v| u32::try_from(v).ok())
    } else {
        None
    }
}

fn circled_number(number: u32) -> char {
    std::char::from_u32(0x2460 + number).unwrap_or_default()
}

fn modify_training_session_elements(
    training_session: &mut domain::TrainingSession,
    field_values: Signal<HashMap<usize, SetFieldValues>>,
) {
    for (element_idx, element) in &mut training_session.elements.iter_mut().enumerate() {
        if let domain::TrainingSessionElement::Set {
            reps,
            time,
            weight,
            rpe,
            ..
        } = element
        {
            if let Some(set_field_values) = field_values.get(&element_idx) {
                *reps = set_field_values
                    .reps
                    .validated
                    .clone()
                    .ok()
                    .filter(|reps| *reps > domain::Reps::default());
                *time = set_field_values
                    .time
                    .validated
                    .clone()
                    .ok()
                    .filter(|time| *time > domain::Time::default());
                *weight = set_field_values
                    .weight
                    .validated
                    .clone()
                    .ok()
                    .filter(|weight| *weight > domain::Weight::default());
                *rpe = set_field_values
                    .rpe
                    .validated
                    .clone()
                    .ok()
                    .filter(|rpe| *rpe > domain::RPE::default());
            }
        }
    }
}

async fn save(
    training_session: domain::TrainingSession,
    cache: Cache,
    mut close_dialog: impl FnMut(),
) {
    IS_LOADING.with_mut(|is_loading| *is_loading = true);
    match DOMAIN_SERVICE()
        .modify_training_session(
            training_session.id,
            Some(training_session.notes),
            Some(training_session.elements),
        )
        .await
    {
        Ok(_) => {
            cache.refresh_training_sessions();
        }
        Err(err) => {
            NOTIFICATIONS
                .write()
                .push(format!("Failed to modify training session: {err}"));
        }
    };
    IS_LOADING.with_mut(|is_loading| *is_loading = false);
    close_dialog();
}

#[derive(Store, Clone)]
struct Progress {
    training_session_id: domain::TrainingSessionID,
    start_time: chrono::DateTime<chrono::Utc>,
    element_idx: usize,
    element_start_time: chrono::DateTime<chrono::Utc>,
    timer_service: TimerService,
}

impl Progress {
    fn new(training_session_id: domain::TrainingSessionID) -> Self {
        Self {
            training_session_id,
            start_time: chrono::Utc::now(),
            element_idx: usize::MAX,
            element_start_time: chrono::Utc::now(),
            timer_service: TimerService::default(),
        }
    }

    fn is_active(&self) -> bool {
        self.element_idx != usize::MAX
    }

    fn set_element_idx(&mut self, element_idx: usize) {
        if self.element_idx == usize::MAX {
            self.start_time = chrono::Utc::now();
        }
        self.element_idx = element_idx;
        self.element_start_time = chrono::Utc::now();
    }
}

impl From<web_app::OngoingTrainingSession> for Progress {
    fn from(value: web_app::OngoingTrainingSession) -> Self {
        Self {
            training_session_id: value.training_session_id.into(),
            start_time: value.start_time,
            element_idx: value.element_idx,
            element_start_time: value.element_start_time,
            timer_service: TimerService::from(value.timer_state),
        }
    }
}

impl From<Progress> for web_app::OngoingTrainingSession {
    fn from(value: Progress) -> Self {
        web_app::OngoingTrainingSession {
            training_session_id: value.training_session_id.as_u128(),
            start_time: value.start_time,
            element_idx: value.element_idx,
            element_start_time: value.element_start_time,
            timer_state: value.timer_service.into(),
        }
    }
}

#[derive(Clone)]
pub enum EditDialog {
    None,
    Options {
        training_session: domain::TrainingSession,
        section_idx: usize,
        element_idx: usize,
        exercise_idx: usize,
    },
    AddExercise {
        training_session: domain::TrainingSession,
        section_idx: usize,
    },
    ReplaceExercise {
        training_session: domain::TrainingSession,
        section_idx: usize,
        exercise_idx: usize,
    },
    AppendExercise {
        training_session: domain::TrainingSession,
    },
}
