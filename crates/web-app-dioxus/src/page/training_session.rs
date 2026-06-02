use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use dioxus::{prelude::*, web::WebEventExt};
use futures_util::StreamExt;
use gloo_timers::future::IntervalStream;
use indexmap::IndexMap;
use web_sys::wasm_bindgen::JsCast;

use valens_domain::{self as domain, TrainingSessionService};
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE, DROP_SET_CALCULATOR, METRONOME, ONE_REP_MAX_CALCULATOR, Route,
    cache::{Cache, CacheState},
    eh,
    notification::notify,
    ongoing_training_session::OngoingTrainingSession,
    page::{
        self,
        common::{OneRepMaxCalculatorState, SetsPerMuscle, Timer, TimerService},
    },
    settings::Settings,
    ui::{
        element::{
            ActivityBar, Block, CenteredBlock, Color, Dialog, ErrorMessage, FloatingActionButton,
            Icon, Loading, LoadingDialog, LoadingPage, MenuOption, NoConnection, OptionsMenu,
            SaveDialog, Title,
        },
        form::{Field, FieldValue, FieldValueState, InputField},
    },
    unsaved_changes::{UnsavedChangesDialog, use_unsaved_changes},
};

static IS_LOADING: GlobalSignal<bool> = Signal::global(|| false);

/// Renders a training session and drives its *ongoing* state.
///
/// At most one training session is ongoing at a time. It is tracked in the session-scoped
/// [`OngoingTrainingSession`] context. `progress` mirrors that state locally, while
/// `owns_progress` records whether this page currently drives it.
///
/// An ongoing training session is started when no other session is ongoing and either
/// - this session is opened while it still has no stored set, or
/// - the user activates a set through a `<<` button in edit mode (this also restarts a
///   session that already has stored sets).
///
/// An ongoing session previously stored for this id is resumed when the page is opened.
///
/// The ongoing session ends as soon as any of the following holds:
/// - the end of the training session is reached,
/// - every set has stored input, or
/// - the user presses the end button.
///
/// Ending the session returns the page to view mode.
///
/// While another session is ongoing, this page stays editable but shows no active focus
/// and ignores `<<` activations.
///
/// The inner component is rendered as a single keyed list entry so that navigating directly
/// between two training sessions remounts it, reinitializing all per-session state. A lone
/// keyed child would not be remounted on a key change, and the router reuses the route
/// component across parameter changes.
#[component]
pub fn TrainingSession(id: domain::TrainingSessionID) -> Element {
    rsx! {
        for current in [id] {
            TrainingSessionInner { key: "{current:?}", id: current }
        }
    }
}

#[component]
fn TrainingSessionInner(id: domain::TrainingSessionID) -> Element {
    let mut edit = use_signal(|| false);
    let mut progress = use_store(|| Progress::new(id));
    let mut owns_progress = use_signal(|| false);
    let mut resume_attempted = use_signal(|| false);

    let ongoing = consume_context::<OngoingTrainingSession>();
    let id_value = id.as_u128();
    let other_session_running = move || ongoing.in_progress_other_than(id_value);

    let mut end_session = move |element_count: usize| {
        edit.set(false);
        owns_progress.set(false);
        progress.write().set_element_idx(element_count);
        progress.timer_service().write().unset();
        spawn(async move {
            ongoing.clear().await;
        });
    };

    let cache = consume_context::<Cache>();
    let training_session = use_memo(move || {
        if let CacheState::Ready(training_sessions) = &*cache.training_sessions.read() {
            let training_session = training_sessions.iter().find(|e| e.id == id).cloned();
            if let Some(training_session) = &training_session
                && training_session.is_empty()
                && !progress.read().is_active()
            {
                edit.set(true);
                if ongoing.is_loaded() && !other_session_running() {
                    owns_progress.set(true);
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

    use_effect(move || {
        if !ongoing.is_loaded() || *resume_attempted.peek() {
            return;
        }
        resume_attempted.set(true);
        if let Some(ongoing) = ongoing
            .get()
            .filter(|o| o.training_session_id == id.as_u128())
        {
            progress.set(Progress::from(ongoing));
            edit.set(true);
            owns_progress.set(true);
        }
    });
    use_effect(move || {
        if !owns_progress() || !progress.read().is_active() {
            return;
        }
        let (len, all_sets_recorded) =
            training_session
                .read()
                .as_ref()
                .map_or((usize::MAX, false), |training_session| {
                    (
                        training_session.elements.len(),
                        training_session.all_sets_recorded(),
                    )
                });
        if progress.read().element_idx >= len || all_sets_recorded {
            end_session(len);
        } else {
            let value = web_app::OngoingTrainingSession::from((*progress.read()).clone());
            spawn(async move {
                ongoing.set(value).await;
            });
        }
    });

    let settings = use_context::<Settings>();
    use_effect(move || {
        progress
            .timer_service()
            .write()
            .set_beep_volume(settings.beep_volume());
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
        if let Some(training_session) = training_session()
            && let Some(element) = training_session.elements.get(element_idx)
        {
            let automatic_metronome = settings.automatic_metronome();
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
    });

    use_effect(move || {
        if !settings.notifications() {
            return;
        }

        if let (Some(training_session), CacheState::Ready(exercises)) =
            (training_session(), &*cache.exercises.read())
        {
            let element_idx = *progress.element_idx().read();
            let element = training_session.elements.get(element_idx);
            match element {
                Some(domain::TrainingSessionElement::Set { .. }) => {
                    let sections = training_session.compute_sections();
                    if let Some(section) = sections.get(training_session.section_idx(element_idx)) {
                        let exercise_ids = unique(section.exercise_ids());
                        let exercise_names = exercise_ids
                            .clone()
                            .into_iter()
                            .map(|id| {
                                let number =
                                    if let Some(number) = exercise_number(&id, &exercise_ids) {
                                        format!("{} ", circled_number(number))
                                    } else {
                                        String::new()
                                    };
                                let name = exercise_name(id, exercises);
                                format!("{number}{name}")
                            })
                            .collect::<Vec<_>>();
                        let title = exercise_names.join("\n");
                        web_app::replace_notifications(&title, None);
                    }
                }
                Some(domain::TrainingSessionElement::Rest { target_time, .. }) => {
                    web_app::replace_notifications("Rest", target_time.map(|t| format!("{t} s")));
                }
                None => {}
            }
        }
    });
    use_drop(move || {
        web_app::close_notifications();
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

    let section_elements: Signal<HashMap<usize, web_sys::Element>> = use_signal(HashMap::new);

    let current_section_idx = use_memo(move || -> Option<usize> {
        if !owns_progress() {
            return None;
        }
        let ts_ref = training_session.read();
        let ts = ts_ref.as_ref()?;
        let element_idx = progress.read().element_idx;
        if ts.elements.is_empty() || element_idx >= ts.elements.len() {
            return None;
        }
        Some(ts.section_idx_lookahead(element_idx))
    });

    // Resolve the active section's DOM element through a memo so the scroll effect below
    // re-fires only when the target changes, not on every unrelated section mount.
    let current_section_element = use_memo(move || -> Option<web_sys::Element> {
        let idx = current_section_idx()?;
        section_elements.read().get(&idx).cloned()
    });

    use_effect(move || {
        let Some(element) = current_section_element() else {
            return;
        };
        let options = web_sys::ScrollIntoViewOptions::new();
        options.set_behavior(web_sys::ScrollBehavior::Smooth);
        options.set_block(web_sys::ScrollLogicalPosition::Center);
        element.scroll_into_view_with_scroll_into_view_options(&options);
    });

    match (
        &*cache.training_sessions.read(),
        &*training_session.read(),
        &*cache.exercises.read(),
    ) {
        (
            CacheState::Ready(training_sessions),
            Some(training_session),
            CacheState::Ready(exercises),
        ) => {
            let elements_len = training_session.elements.len();
            let show_active_focus = owns_progress() && progress.read().element_idx < elements_len;
            let focus = SetFocus {
                progress,
                owns_progress,
                ongoing,
                show_active_focus,
            };
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
                    {view_form(field_values, progress, focus, edit_dialog, training_session, exercises, settings, cache, section_elements)},
                } else {
                    {view_list(training_session, exercises, settings)},
                    {view_muscles(training_session, exercises)}
                }
                Notes { notes, edit },
                {view_edit_dialog(edit_dialog, field_values, training_sessions, cache)}
                if let Some(ongoing) = ongoing.get().filter(|o| o.training_session_id == id.as_u128()) {
                    OngoingSessionBar {
                        start_time: ongoing.start_time,
                        on_end: move |()| end_session(elements_len),
                    }
                }
                FloatingActionButton {
                    icon: (if edit() { if has_changes() { "save" } else { "eye" } } else { "edit" }).to_string(),
                    on_click: eh!(mut edit, training_session; {
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
                UnsavedChangesDialog {}
            }
        }
        (CacheState::Ready(_), None, _) => rsx! {
            ErrorMessage { message: "Training session not found" }
        },
        (
            CacheState::Error(domain::ReadError::Storage(domain::StorageError::NoConnection)),
            _,
            _,
        ) => {
            rsx! { NoConnection {} }
        }
        (CacheState::Error(err), _, _) | (_, _, CacheState::Error(err)) => {
            rsx! { ErrorMessage { message: err } }
        }
        (CacheState::Loading, _, _) | (_, _, CacheState::Loading) => {
            rsx! { LoadingPage {} }
        }
    }
}

#[component]
fn OngoingSessionBar(
    start_time: chrono::DateTime<chrono::Utc>,
    on_end: EventHandler<()>,
) -> Element {
    let mut now = use_signal(chrono::Utc::now);
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        let mut interval = IntervalStream::new(1000);
        while interval.next().await.is_some() {
            now.set(chrono::Utc::now());
        }
    });
    let mut confirm = use_signal(|| false);

    let elapsed = (now() - start_time).num_seconds().max(0);
    let hours = elapsed / 3600;
    let minutes = (elapsed % 3600) / 60;
    let seconds = elapsed % 60;
    let elapsed_text = if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    };

    rsx! {
        ActivityBar {
            div {
                class: "is-flex is-align-items-center",
                Icon {
                    name: "dumbbell",
                    class: "has-text-info mr-3"
                }
                div {
                    class: "is-flex-grow-1 s-size-7 has-text-centered has-text-weight-bold",
                    span {
                        "{elapsed_text}"
                    }
                }
                a {
                    class: "has-text-info ml-3",
                    "data-testid": "activity-bar-end-session",
                    onclick: move |_| confirm.set(true),
                    Icon { name: "stop" }
                }
            }
        }
        if confirm() {
            Dialog {
                on_close: move |_| confirm.set(false),
                color: Color::Info,
                div {
                    class: "block",
                    "End the current training session?"
                }
                div {
                    class: "field is-grouped is-grouped-centered",
                    div {
                        class: "control",
                        onclick: move |_| confirm.set(false),
                        button {
                            class: "button is-light is-soft",
                            "data-testid": "activity-bar-end-session-cancel",
                            "Continue"
                        }
                    }
                    div {
                        class: "control",
                        onclick: move |_| {
                            confirm.set(false);
                            on_end.call(());
                        },
                        button {
                            class: "button is-info",
                            "data-testid": "activity-bar-end-session-confirm",
                            "End"
                        }
                    }
                }
            }
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

/// Per-set focus state of the training session form.
///
/// Bundles the state that decides how a set row reacts to interaction: whether this page owns
/// the ongoing session, whether a *different* session is ongoing, and whether the row matching
/// the current progress should be highlighted as the active focus.
#[derive(Clone, Copy)]
struct SetFocus {
    progress: Store<Progress>,
    owns_progress: Signal<bool>,
    ongoing: OngoingTrainingSession,
    show_active_focus: bool,
}

impl SetFocus {
    fn is_focused(self, element_idx: usize) -> bool {
        self.show_active_focus && self.progress.read().element_idx == element_idx
    }

    fn other_session_running(self) -> bool {
        self.ongoing
            .in_progress_other_than(self.progress.read().training_session_id.as_u128())
    }

    /// Marks this page as the owner of the ongoing session.
    ///
    /// Gaining ownership from a non-owning state begins a new run and resets the elapsed time.
    /// Re-focusing a row within an already-owned session keeps the original start time.
    fn take_ownership(mut self) {
        if !*self.owns_progress.peek() {
            self.progress.write().reset();
        }
        self.owns_progress.set(true);
    }
}

#[allow(clippy::too_many_arguments)]
fn view_form(
    mut field_values: Signal<HashMap<usize, SetFieldValues>>,
    mut progress: Store<Progress>,
    focus: SetFocus,
    mut edit_dialog: Signal<EditDialog>,
    training_session: &domain::TrainingSession,
    exercises: &[domain::Exercise],
    settings: Settings,
    cache: Cache,
    mut section_elements: Signal<HashMap<usize, web_sys::Element>>,
) -> Element {
    let mut element_idx: usize = 0;
    let sections = training_session.compute_sections();
    let progress_element_idx = progress.read().element_idx;
    let progress_section_idx = training_session.section_idx(progress_element_idx);
    let progress_section_idx_lookahead =
        training_session.section_idx_lookahead(progress_element_idx);
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
        let is_current_section = !focus.show_active_focus
            || section_idx == progress_section_idx
            || section_idx == progress_section_idx_lookahead;
        let exercise_ids = unique(section.exercise_ids());
        let exercise_ids_len = exercise_ids.len();
        let element_idx_for_options = element_idx;
        let exercise_names = exercise_ids.iter().enumerate().map(|(i, id)| {
            let name = exercise_name(*id, exercises);
            let number = exercise_number(id, &exercise_ids);
            let note = training_session.exercise_notes.get(id).cloned().unwrap_or_default();
            let note_is_empty = note.is_empty();
            let exercise_id = *id;
            rsx! {
                tr {
                    class: if is_current_section { "" } else { "is-semitransparent" },
                    td {
                        class: "has-text-centered has-text-weight-bold",
                        class: if i == 0 { "pt-2" },
                        class: if i == exercise_ids_len - 1 && note_is_empty { "pb-1" },
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
                            "data-testid": "item-options",
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
                if !note_is_empty {
                    tr {
                        class: if is_current_section { "" } else { "is-semitransparent" },
                        td {
                            class: "px-2",
                            class: if i == exercise_ids_len - 1 { "pb-1" },
                            colspan: 6,
                            div {
                                class: "is-clickable is-italic has-text-centered",
                                "data-testid": "exercise-note",
                                onclick: eh!(mut edit_dialog; training_session; {
                                    *edit_dialog.write() = EditDialog::ExerciseNote {
                                        training_session,
                                        exercise_id,
                                    };
                                }),
                                { note }
                            }
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

                    let set = if set_field_values.is_empty() && !set_field_values.changed() && target_reps.is_none() && target_time.is_some() && !focus.other_session_running() {
                        if let Some(target_time) = target_time {
                            rsx! {
                                tr {
                                    class: if is_current_section { "" } else { "is-semitransparent" },
                                    td {}
                                    td {
                                        class: "p-1",
                                        colspan: 4,
                                        div {
                                            class: "notification is-link has-text-centered px-6 py-1",
                                            class: if focus.is_focused(element_idx) { "is-size-1" },
                                            if focus.is_focused(element_idx) {
                                                Timer { timer: progress.timer_service() }
                                            } else {
                                                div {
                                                    onclick: move |_| {
                                                        if focus.other_session_running() {
                                                            return;
                                                        }
                                                        focus.take_ownership();
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
                                            class: if focus.is_focused(element_idx) { "is-link is-outlined" },
                                            disabled: focus.other_session_running(),
                                            onclick: eh!(mut training_session; target_time; {
                                                if focus.is_focused(element_idx) {
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
                                                    focus.take_ownership();
                                                    progress.write().set_element_idx(element_idx);
                                                    progress.timer_service().write().unset();
                                                }
                                            }),
                                            Icon { name: if focus.is_focused(element_idx) { "check" } else { "angles-left" } }
                                        }
                                    }
                                }
                                if is_current_section {
                                    {set_value_buttons(set_buttons, element_idx, field_values, settings)}
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
                                        on_input: move |event: FormEvent| {
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
                                    if settings.show_tut() {
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
                                            on_input: move |event: FormEvent| {
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
                                        on_input: move |event: FormEvent| {
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
                                    if settings.show_rpe() {
                                        InputField {
                                            left_icon: rsx! { "@" },
                                            inputmode: "numeric",
                                            size: 2,
                                            value: set_field_values.rpe.input.clone(),
                                            error: if let Err(err) = &set_field_values.rpe.validated { err.clone() },
                                            has_changed: set_field_values.rpe.changed(),
                                            has_text_right: true,
                                            on_input: move |event: FormEvent| {
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
                                    if set_field_values.valid() && !(set_field_values.is_empty() && !set_field_values.changed() && focus.is_focused(element_idx)) {
                                        button {
                                            class: "button is-small",
                                            class: if set_field_values.has_valid_changes() { "is-link is-outlined" } else if !set_field_values.is_empty() { "is-ghost" },
                                            "data-testid": "set-action",
                                            disabled: focus.other_session_running() && set_field_values.is_empty() && !set_field_values.changed(),
                                            onclick: eh!(mut training_session; field_values, set_field_values; {
                                                if set_field_values.is_empty() && !set_field_values.changed() {
                                                    focus.take_ownership();
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
                                {set_value_buttons(set_buttons, element_idx, field_values, settings)}
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
                            if focus.is_focused(element_idx) {
                                td {}
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
                                td {}
                                td {
                                    class: "p-1",
                                    colspan: 4,
                                    div {
                                        class: "notification p-0 is-size-7 has-background-auto-text-95 has-text-centered",
                                        onclick: move |_| {
                                            if focus.other_session_running() {
                                                return;
                                            }
                                            focus.take_ownership();
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
            tbody {
                class: if settings.scroll_snapping() { "section-snap" },
                onmounted: move |event| {
                    if let Some(element) = event.data().try_as_web_event() {
                        section_elements.write().insert(section_idx, element);
                    }
                },
                for name in exercise_names {
                    {name}
                }
                for set in sets {
                    {set}
                }
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

/// Renders the row of buttons that prefill a set with the target, previous-set,
/// or previous-session values.
fn set_value_buttons(
    set_buttons: IndexMap<domain::Set, Vec<String>>,
    element_idx: usize,
    field_values: Signal<HashMap<usize, SetFieldValues>>,
    settings: Settings,
) -> Element {
    rsx! {
        tr {
            td {}
            td {
                class: "p-1 has-text-centered",
                colspan: 4,
                for (set, icons) in set_buttons {
                    button {
                        class: "button is-small mr-2",
                        "data-testid": "set-value",
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
                        span { {set.to_string(settings.show_tut(), settings.show_rpe())} },
                    }
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
        return rsx! { Loading {} };
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
                                if let Some(n) = notes.write().as_mut() {
                                    n.input = event.value();
                                    n.validated = Ok(event.value());
                                }
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
    settings: Settings,
) -> Element {
    let sections = training_session.compute_sections();
    let rows = sections.iter().map(|section| {
        let exercise_ids = unique(section.exercise_ids());
        let exercise_ids_len = exercise_ids.len();
        let exercise_names = exercise_ids.clone().into_iter().enumerate().map(|(i, id)| {
            let name = exercise_name(id, exercises);
            let number = exercise_number(&id, &exercise_ids);
            let note = training_session.exercise_notes.get(&id).cloned().unwrap_or_default();
            let note_is_empty = note.is_empty();
            rsx! {
                tr {
                    td {
                        class: "has-text-centered has-text-weight-bold",
                        class: if i == 0 { "pt-2" },
                        class: if i == exercise_ids_len - 1 && note_is_empty { "pb-1" },
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
                if !note_is_empty {
                    tr {
                        td {
                            class: "px-2",
                            class: if i == exercise_ids_len - 1 { "pb-1" },
                            colspan: 5,
                            div {
                                class: "is-italic has-text-centered",
                                "data-testid": "exercise-note",
                                { note }
                            }
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
                                if reps.is_none() && (time.is_none() || !settings.show_tut()) && weight.is_none() && (rpe.is_none() || !settings.show_rpe()) {
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
                                            if settings.show_tut() && *time > domain::Time::default() {
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
                                            if settings.show_rpe() && *rpe > domain::RPE::ZERO {
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
    training_sessions: &[domain::TrainingSession],
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
                    LoadingDialog {}
                } else {
                    OptionsMenu {
                        options: vec![
                            rsx! {
                                MenuOption {
                                    icon: "file-lines".to_string(),
                                    text: "Show exercise notes".to_string(),
                                    "data-testid": "options-show-exercise-notes",
                                    on_click: eh!(mut edit_dialog; training_session, section_idx, exercise_idx; {
                                        let sections = training_session.compute_sections();
                                        let exercise_ids = unique(sections[section_idx].exercise_ids());
                                        let exercise_id = exercise_ids[exercise_idx];
                                        *edit_dialog.write() = EditDialog::ExerciseNote { training_session, exercise_id };
                                    })
                                },
                                {
                                    let sections = training_session.compute_sections();
                                    let exercise_ids = unique(sections[*section_idx].exercise_ids());
                                    let exercise_id = exercise_ids[*exercise_idx];
                                    let recent_best_set = domain::most_recent_best_set_for_one_rep_max(
                                        training_sessions,
                                        exercise_id,
                                    );
                                    rsx! {
                                        if let Some((reps, weight)) = recent_best_set {
                                            MenuOption {
                                                icon: "dumbbell".to_string(),
                                                text: "Show 1RM".to_string(),
                                                "data-testid": "options-1rm",
                                                on_click: eh!(mut edit_dialog; {
                                                    let mut state = OneRepMaxCalculatorState::new(reps.into(), f32::from(weight));
                                                    state.visible = true;
                                                    *ONE_REP_MAX_CALCULATOR.write() = state;
                                                    *edit_dialog.write() = EditDialog::None;
                                                })
                                            }
                                            MenuOption {
                                                icon: "arrow-down-wide-short".to_string(),
                                                text: "Show drop set".to_string(),
                                                "data-testid": "options-drop-set",
                                                on_click: eh!(mut edit_dialog; {
                                                    let mut state = DROP_SET_CALCULATOR.write();
                                                    state.start_weight = f32::from(weight);
                                                    state.visible = true;
                                                    *edit_dialog.write() = EditDialog::None;
                                                })
                                            }
                                        }
                                    }
                                },
                                MenuOption {
                                    icon: "plus".to_string(),
                                    text: "Add set".to_string(),
                                    on_click: eh!(mut training_session; element_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, field_values);
                                        training_session.add_set(element_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "plus".to_string(),
                                    text: "Add same exercise".to_string(),
                                    on_click: eh!(mut training_session; section_idx, exercise_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, field_values);
                                        training_session.add_same_exercise(section_idx, exercise_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "plus".to_string(),
                                    text: "Add other exercise".to_string(),
                                    on_click: eh!(mut edit_dialog; training_session, section_idx; {
                                        *edit_dialog.write() = EditDialog::AddExercise { training_session, section_idx };
                                    })
                                },
                                MenuOption {
                                    icon: "arrow-up".to_string(),
                                    text: "Move up".to_string(),
                                    on_click: eh!(mut training_session; section_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, field_values);
                                        training_session.move_section_up(section_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "arrow-down".to_string(),
                                    text: "Move down".to_string(),
                                    on_click: eh!(mut training_session; section_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, field_values);
                                        training_session.move_section_down(section_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "arrow-right-arrow-left".to_string(),
                                    text: "Replace exercise".to_string(),
                                    on_click: eh!(mut edit_dialog; training_session, section_idx, exercise_idx; {
                                        *edit_dialog.write() = EditDialog::ReplaceExercise { training_session, section_idx, exercise_idx };
                                    })
                                },
                                MenuOption {
                                    icon: "times".to_string(),
                                    text: "Remove set".to_string(),
                                    on_click: eh!(mut training_session; section_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, field_values);
                                        training_session.remove_set(section_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "times".to_string(),
                                    text: "Remove exercise".to_string(),
                                    on_click: eh!(mut training_session; section_idx, exercise_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, field_values);
                                        training_session.remove_exercise(section_idx, exercise_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                            },
                        ],
                        on_close: eh!(mut close_dialog; { close_dialog(); })
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
                    LoadingDialog {}
                } else {
                    Dialog {
                        on_close: eh!(mut close_dialog; { close_dialog(); }),
                        no_horizontal_padding: true,
                        page::exercises::ExerciseList {
                            add: false,
                            filter: String::new(),
                            on_exercise_click: {
                                let training_session = training_session.clone();
                                let section_idx = *section_idx;
                                move |(_, exercise_id)| {
                                    let mut training_session = training_session.clone();
                                    modify_training_session_elements(&mut training_session, field_values);
                                    training_session.add_exercise(section_idx, exercise_id);
                                    save(training_session, cache, close_dialog)
                                }
                            },
                            on_catalog_click: |_| {}
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
                    LoadingDialog {}
                } else {
                    Dialog {
                        title: rsx! { "Replace exercise" },
                        on_close: eh!(mut close_dialog; { close_dialog(); }),
                        no_horizontal_padding: true,
                        page::exercises::ExerciseList {
                            add: false,
                            filter: String::new(),
                            on_exercise_click: {
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
                            on_catalog_click: |_| {}
                        }
                    }
                }
            }
        }
        EditDialog::AppendExercise { training_session } => {
            rsx! {
                if IS_LOADING() {
                    LoadingDialog {}
                } else {
                    Dialog {
                        on_close: eh!(mut close_dialog; { close_dialog(); }),
                        no_horizontal_padding: true,
                        page::exercises::ExerciseList {
                            add: false,
                            filter: String::new(),
                            on_exercise_click: {
                                let training_session = training_session.clone();
                                move |(_, exercise_id)| {
                                    let mut training_session = training_session.clone();
                                    modify_training_session_elements(&mut training_session, field_values);
                                    training_session.append_exercise(exercise_id);
                                    save(training_session, cache, close_dialog)
                                }
                            },
                            on_catalog_click: |_| {}
                        }
                    }
                }
            }
        }
        EditDialog::ExerciseNote {
            training_session,
            exercise_id,
        } => {
            rsx! {
                ExerciseNoteDialog {
                    training_session: training_session.clone(),
                    exercise_id: *exercise_id,
                    on_save: move |ts| save(ts, cache, close_dialog),
                    on_close: eh!(mut close_dialog; { close_dialog(); }),
                }
            }
        }
    }
}

#[component]
fn ExerciseNoteDialog(
    training_session: domain::TrainingSession,
    exercise_id: domain::ExerciseID,
    on_save: EventHandler<domain::TrainingSession>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    let cache = consume_context::<Cache>();
    let note = training_session
        .exercise_notes
        .get(&exercise_id)
        .cloned()
        .unwrap_or_default();
    let mut note_input = use_signal(|| note.clone());
    let mut textarea_element = use_signal(|| None::<web_sys::HtmlTextAreaElement>);
    let changed = note_input.read().trim() != note.trim();
    let previous_notes: Vec<(chrono::NaiveDate, String, String)> =
        match (&*cache.training_sessions.read(), &*cache.routines.read()) {
            (CacheState::Ready(training_sessions), CacheState::Ready(routines)) => training_session
                .previous_exercise_notes(exercise_id, training_sessions)
                .into_iter()
                .map(|previous_note| {
                    (
                        previous_note.date,
                        routines
                            .iter()
                            .find(|routine| routine.id == previous_note.routine_id)
                            .map_or_else(|| "-".to_string(), |routine| routine.name.to_string()),
                        previous_note.note,
                    )
                })
                .collect(),
            _ => vec![],
        };
    rsx! {
        SaveDialog {
            title: rsx! { "Exercise notes" },
            on_close,
            on_save: eh!(mut training_session; exercise_id; {
                let note = note_input.read().trim().to_string();
                if note.is_empty() {
                    training_session.exercise_notes.remove(&exercise_id);
                } else {
                    training_session.exercise_notes.insert(exercise_id, note);
                }
                on_save.call(training_session);
            }),
            is_loading: IS_LOADING(),
            disabled: IS_LOADING() || !changed,
            div {
                class: "field",
                div {
                    class: "control",
                    textarea {
                        class: "textarea",
                        class: if changed { "is-info" },
                        oninput: move |event| {
                            *note_input.write() = event.value();
                        },
                        onmounted: move |event| async move {
                            let _ = event.set_focus(true).await;
                            if let Some(element) = event.data().try_as_web_event()
                                && let Some(textarea) = element.dyn_ref::<web_sys::HtmlTextAreaElement>()
                            {
                                if let Ok(len) = u32::try_from(textarea.value().encode_utf16().count()) {
                                    let _ = textarea.set_selection_range(len, len);
                                }
                                textarea_element.set(Some(textarea.clone()));
                            }
                        },
                        { note.clone() }
                    }
                }
            }
            if !previous_notes.is_empty() {
                for (date, routine_name, note) in previous_notes {
                    div {
                        div {
                            class: "block has-text-centered has-text-weight-bold mb-1",
                            "{date} {routine_name}"
                        }
                        div {
                            class: "is-relative is-italic has-text-centered mb-2",
                            "data-testid": "previous-exercise-note",
                            {note.clone()}
                            button {
                                class: "button is-overlay-top-right p-0 mr-2",
                                r#type: "button",
                                "data-testid": "exercise-note-reuse",
                                onclick: move |_| {
                                    note_input.write().clone_from(&note);
                                    if let Some(textarea) = textarea_element.read().as_ref() {
                                        textarea.set_value(&note);
                                    }
                                },
                                Icon { name: "reply".to_string() }
                            }
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
            && let Some(set_field_values) = field_values.get(&element_idx)
        {
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

async fn save(
    training_session: domain::TrainingSession,
    cache: Cache,
    mut close_dialog: impl FnMut(),
) {
    *IS_LOADING.write() = true;
    match DOMAIN_SERVICE()
        .modify_training_session(
            training_session.id,
            Some(training_session.notes),
            Some(training_session.elements),
            Some(training_session.exercise_notes),
        )
        .await
    {
        Ok(_) => {
            cache.refresh_training_sessions();
        }
        Err(err) => {
            notify("Failed to modify training session", &err);
        }
    }
    *IS_LOADING.write() = false;
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

    fn reset(&mut self) {
        self.start_time = chrono::Utc::now();
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
    ExerciseNote {
        training_session: domain::TrainingSession,
        exercise_id: domain::ExerciseID,
    },
}
