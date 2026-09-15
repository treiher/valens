use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::Hash,
    rc::Rc,
};

use dioxus::{prelude::*, web::WebEventExt};
use futures_util::StreamExt;
use gloo_timers::future::{IntervalStream, TimeoutFuture};
use indexmap::IndexMap;

use valens_domain::{self as domain, TrainingSessionService};
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE, DROP_SET_CALCULATOR, ONE_REP_MAX_CALCULATOR, Route,
    audio::{METRONOME_START_DELAY, MetronomeService, TICK_INTERVAL_MS, Timer, TimerService},
    cache::{Cache, CacheState},
    dialog::{drop_set::DropSetCalculator, one_rep_max::OneRepMaxCalculatorState},
    eh,
    loading::LoadingFlag,
    muscle::SetsPerMuscle,
    notification::notify,
    ongoing_training_session::OngoingTrainingSession,
    page,
    settings::Settings,
    ui::{
        element::{
            ActivityBar, Block, CenteredBlock, Color, Dialog, ErrorPage, FloatingActionButton,
            Icon, Loading, LoadingDialog, LoadingPage, MenuOption, OptionsMenu, PhaseBar,
            SaveDialog, Title,
        },
        form::{FieldValue, FieldValueState, InputField, TextAreaField},
    },
    unsaved_changes::{UnsavedChangesDialog, use_unsaved_changes},
    wake_lock::{self, WakeLock},
};

static IS_LOADING: GlobalSignal<bool> = Signal::global(|| false);

/// Number of earlier training sessions whose sets can be shown for an exercise.
const RECENT_SESSIONS: usize = 3;

/// The sets of an exercise in earlier training sessions, from the most recent to the oldest.
type SessionSets = Vec<(chrono::NaiveDate, Vec<domain::Set>)>;

const COLUMN_HEADER_CLASS: &str =
    "p-1 has-text-centered is-size-7 has-text-grey has-text-weight-normal";

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
    // The elements the drop set calculator fills, `None` while it is closed.
    let mut drop_set_target: Signal<Option<Vec<usize>>> = use_signal(|| None);

    let ongoing = consume_context::<OngoingTrainingSession>();
    let id_value = id.as_u128();
    let other_session_running = move || ongoing.in_progress_other_than(id_value);

    let mut end_session = move |element_count: usize| {
        edit.set(false);
        owns_progress.set(false);
        progress.write().set_element_idx(element_count);
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
    // Kept apart from the form, which is rendered anew with every tick of the timer.
    let recent_session_sets_by_exercise = use_memo(move || {
        let (CacheState::Ready(training_sessions), Some(training_session)) =
            (&*cache.training_sessions.read(), &*training_session.read())
        else {
            return HashMap::new();
        };
        DOMAIN_SERVICE()
            .get_recent_session_sets_by_exercise(
                training_session,
                training_sessions,
                RECENT_SESSIONS,
            )
            .into_iter()
            .map(|(exercise_id, sessions)| {
                let sessions = sessions
                    .into_iter()
                    .map(|(date, sets)| {
                        (
                            date,
                            sets.into_iter()
                                .filter_map(domain::TrainingSessionElement::set)
                                .collect(),
                        )
                    })
                    .collect::<SessionSets>();
                (exercise_id, sessions)
            })
            .collect::<HashMap<_, _>>()
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
    let mut metronome = use_store(MetronomeService::new);
    let phase_clock = PhaseClock {
        bar: use_signal(|| None),
        elapsed: use_signal(|| 0.),
        countdown_starts: use_signal(|| 0),
        countdown_start_pending: use_signal(|| false),
    };
    // The bar clock belongs to the element it was started on and does not outlive it.
    use_effect(move || {
        let element_idx = *progress.element_idx().read();
        let mut bar = phase_clock.bar;
        if bar.peek().is_some_and(|bar| bar.element_idx != element_idx) {
            bar.set(None);
        }
    });
    use_effect(move || {
        progress
            .timer_service()
            .write()
            .set_beep_volume(settings.beep_volume());
        metronome.write().set_beep_volume(settings.beep_volume());
    });

    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        let mut interval = IntervalStream::new(TICK_INTERVAL_MS);
        while interval.next().await.is_some() {
            let mut timer = progress.timer_service();
            timer.peek().sync();
            // Writing on every tick would re-run the effects reading the timer ten times a second.
            if timer.peek().needs_update() {
                timer.write().update();
            }
            if metronome.peek().is_active() {
                metronome.write().update();
            }
            // Only the segmented bar moves between two seconds, so it alone reads this.
            let mut elapsed = phase_clock.elapsed;
            let value = phase_clock.seconds(*progress.element_idx().peek(), &timer.peek());
            if (*elapsed.peek() - value).abs() > f64::EPSILON {
                elapsed.set(value);
            }
        }
    });

    let mut field_values = use_signal(HashMap::new);
    let mut expanded_history: Signal<HashSet<usize>> = use_signal(HashSet::new);
    use_memo(move || {
        if let Some(training_session) = training_session() {
            expanded_history.set(HashSet::new());
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
                                    reps: FieldValue::new_with_empty_default(reps),
                                    time: FieldValue::new_with_empty_default(time),
                                    weight: FieldValue::new_with_empty_default(weight),
                                    rpe: FieldValue::new_with_empty_default(rpe),
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

    let current_values_are_unperformed = use_memo(move || {
        field_values
            .read()
            .get(&*progress.element_idx().read())
            .is_none_or(SetFieldValues::is_unperformed)
    });

    // The beeps and the displays of a set are two readings of one clock.
    let clock_is_running = use_memo(move || {
        let element_idx = *progress.element_idx().read();
        phase_clock.bar_is_running(element_idx) || progress.timer_service().read().is_active()
    });
    use_effect(move || {
        let element_idx = *progress.element_idx().read();
        let target = training_session
            .read()
            .as_ref()
            .and_then(|training_session| training_session.elements.get(element_idx).cloned())
            .and_then(|element| match element {
                domain::TrainingSessionElement::Set {
                    target_reps,
                    target_tempo,
                    automatic,
                    ..
                } if target_reps.non_zero().is_some() || target_tempo.phases().len() > 1 => Some((
                    target_tempo.non_zero()?,
                    countdown_seconds(target_reps, target_tempo, automatic),
                )),
                _ => None,
            })
            .filter(|_| !other_session_running());
        match target {
            Some((tempo, countdown)) if clock_is_running() => {
                metronome.write().set_tempo(tempo);
                let elapsed = phase_clock
                    .seconds(element_idx, &progress.timer_service().peek())
                    .max(0.);
                // The tempo of a counted-down set ends with it, so that the beat opening the
                // repetition that follows does not sound.
                let duration = countdown
                    .filter(|_| !phase_clock.has_bar_clock(element_idx))
                    .map(f64::from);
                // The countdown and the tempo run on one clock, so a cue of the countdown can
                // share the moment of a beat.
                let beeped_tempo = duration.map(|_| tempo);
                if progress.timer_service().peek().tempo() != beeped_tempo {
                    progress.timer_service().write().set_tempo(beeped_tempo);
                }
                metronome.write().start(elapsed, duration);
            }
            _ => {
                if progress.timer_service().peek().tempo().is_some() {
                    progress.timer_service().write().set_tempo(None);
                }
                if metronome.peek().is_active() {
                    metronome.write().pause();
                }
            }
        }
    });

    use_effect(move || {
        let element_idx = progress.read().element_idx;
        // Only a set counted down automatically waits for its delayed start.
        let set_start_pending = move |pending: bool| {
            let mut start_pending = phase_clock.countdown_start_pending;
            if *start_pending.peek() != pending {
                start_pending.set(pending);
            }
        };
        if let Some(training_session) = training_session()
            && let Some(element) = training_session.elements.get(element_idx)
        {
            match element {
                domain::TrainingSessionElement::Set {
                    target_reps,
                    target_tempo,
                    target_weight,
                    automatic,
                    ..
                } => {
                    let Some(total) = countdown_seconds(*target_reps, *target_tempo, *automatic)
                    else {
                        set_start_pending(false);
                        return;
                    };
                    if !current_values_are_unperformed() {
                        set_start_pending(false);
                        // Filling a set by hand takes it out of its countdown.
                        if progress.timer_service().peek().is_set() {
                            progress.timer_service().write().unset();
                        }
                        return;
                    }
                    if progress.timer_service().read().is_set() {
                        if progress.timer_service().read().seconds() <= 0 {
                            progress.write().set_element_idx(element_idx + 1);
                            if let Some(set_field_values) =
                                field_values.write().get_mut(&element_idx)
                            {
                                set_field_values.fill(
                                    *target_reps,
                                    target_tempo.seconds_per_rep(),
                                    *target_weight,
                                );
                            }
                            spawn(async move {
                                let mut training_session = training_session.clone();
                                modify_training_session_elements(
                                    &mut training_session,
                                    &field_values.read(),
                                );
                                save(training_session, cache, || {}).await;
                            });
                        }
                    } else {
                        // A set guided by its countdown is not guided by a bar clock.
                        let mut bar = phase_clock.bar;
                        bar.set(None);
                        progress.timer_service().write().set(i64::from(total));
                        set_start_pending(*automatic);
                        if *automatic {
                            // The countdown starts with the first beep, and a tap within that
                            // lead-in decides it instead.
                            let starts = *phase_clock.countdown_starts.peek();
                            let mut start_pending = phase_clock.countdown_start_pending;
                            spawn(async move {
                                TimeoutFuture::new(lead_in_ms()).await;
                                if *progress.element_idx().peek() == element_idx
                                    && *phase_clock.countdown_starts.peek() == starts
                                    && progress.timer_service().peek().is_set()
                                {
                                    start_pending.set(false);
                                    progress.timer_service().write().start();
                                }
                            });
                        }
                    }
                }
                domain::TrainingSessionElement::Rest {
                    target_time,
                    automatic,
                } => {
                    set_start_pending(false);
                    if let Some(target_time) = target_time.non_zero() {
                        if progress.timer_service().read().is_set() {
                            if *automatic && progress.timer_service().read().seconds() <= 0 {
                                progress.write().set_element_idx(element_idx + 1);
                            }
                        } else {
                            progress.timer_service().write().set(i64::from(target_time));
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
                                        format!("{} ", exercise_marker(number))
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
                    web_app::replace_notifications(
                        "Rest",
                        target_time.non_zero().map(|t| format!("{t} s")),
                    );
                }
                None => {}
            }
        }
    });
    use_drop(move || {
        web_app::close_notifications();
    });

    let edit_dialog = use_signal(|| EditDialog::None);
    let exercise_dialog = use_signal(|| page::exercises::ExerciseDialog::None);
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

    let element_elements: Signal<HashMap<usize, web_sys::Element>> = use_signal(HashMap::new);

    // Resolve the DOM element to keep in view through a memo so the scroll effect below
    // re-fires only when the target changes, not on every unrelated mount.
    let target_element = use_memo(move || -> Option<web_sys::Element> {
        if !owns_progress() {
            return None;
        }
        let ts_ref = training_session.read();
        let ts = ts_ref.as_ref()?;
        let element_idx = progress.read().element_idx;
        if ts.elements.is_empty() || element_idx >= ts.elements.len() {
            return None;
        }
        element_elements.read().get(&element_idx).cloned()
    });

    use_effect(move || {
        let Some(element) = target_element() else {
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
                    {view_form(field_values, progress, focus, edit_dialog, exercise_dialog, training_session, &recent_session_sets_by_exercise.read(), exercises, settings, cache, element_elements, expanded_history, phase_clock)},
                } else {
                    {view_list(training_session, exercises)},
                    {view_muscles(training_session, exercises)}
                }
                Notes { notes, edit },
                {view_edit_dialog(edit_dialog, exercise_dialog, field_values, drop_set_target, progress, training_sessions, cache)}
                if drop_set_target.read().is_some() {
                    DropSetCalculator {
                        on_close: move |_| { drop_set_target.set(None); },
                        on_fill: move |weights: Vec<domain::Weight>| {
                            if let Some(elements) = drop_set_target.take() {
                                fill_weights(&mut field_values.write(), &elements, &weights);
                            }
                        },
                    }
                }
                {page::exercises::view_dialog(exercise_dialog, None)}
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
                            modify_training_session_elements(&mut training_session, &field_values.read());
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
            ErrorPage { message: "Training session not found" }
        },
        (CacheState::Error(err), _, _) | (_, _, CacheState::Error(err)) => {
            rsx! { ErrorPage { message: "{err}" } }
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
    /// Fills the values a set is recorded with when its countdown runs out or it is confirmed.
    ///
    /// The rating is what the set felt like and is therefore left to the user.
    fn fill(&mut self, reps: domain::Reps, time: domain::Time, weight: domain::Weight) {
        fill_field(&mut self.reps, reps);
        fill_field(&mut self.time, time);
        fill_field(&mut self.weight, weight);
    }

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

    /// Whether the set carries neither recorded nor entered values.
    fn is_unperformed(&self) -> bool {
        self.is_empty() && !self.changed()
    }
}

/// Writes a value into a field as if the user had entered and saved it.
fn fill_field<T: Default + PartialEq + ToString>(field: &mut FieldValue<T>, value: T) {
    let input = if value == T::default() {
        String::new()
    } else {
        value.to_string()
    };
    field.input.clone_from(&input);
    field.orig = input;
    field.validated = Ok(value);
}

/// The clock a set is guided by, and the state its bar and its countdown are read from.
#[derive(Clone, Copy, PartialEq)]
struct PhaseClock {
    /// The clock of a set performed with its input fields, which has no countdown of its own.
    bar: Signal<Option<BarClock>>,
    /// The seconds elapsed on the clock of the current element, written on every tick.
    elapsed: Signal<f64>,
    /// Counts the countdowns started, so that a delayed start can tell whether it still applies.
    countdown_starts: Signal<usize>,
    /// Whether the countdown of the current element is waiting for its delayed automatic start.
    countdown_start_pending: Signal<bool>,
}

impl PhaseClock {
    /// The seconds elapsed on the clock of `element_idx`.
    fn seconds(&self, element_idx: usize, timer: &TimerService) -> f64 {
        match *self.bar.peek() {
            Some(bar) if bar.element_idx == element_idx => bar.seconds(),
            _ => timer.elapsed_exact(),
        }
    }

    /// Whether the clock of `element_idx` is its bar rather than its countdown.
    fn has_bar_clock(&self, element_idx: usize) -> bool {
        matches!(*self.bar.peek(), Some(bar) if bar.element_idx == element_idx)
    }

    fn bar_is_running(&self, element_idx: usize) -> bool {
        self.bar
            .read()
            .is_some_and(|bar| bar.element_idx == element_idx && bar.is_running())
    }

    /// Starts the bar clock of `element_idx`, or holds it where it stands.
    fn toggle_bar(&mut self, element_idx: usize) {
        let bar = match *self.bar.peek() {
            Some(bar) if bar.element_idx == element_idx => bar.toggled(),
            _ => BarClock::started(element_idx),
        };
        self.bar.set(Some(bar));
    }
}

/// The clock of a set performed against its bar, which the user starts and holds by tapping it.
#[derive(Clone, Copy)]
struct BarClock {
    element_idx: usize,
    elapsed: f64,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl BarClock {
    fn started(element_idx: usize) -> Self {
        Self {
            element_idx,
            elapsed: 0.,
            started_at: Some(chrono::Utc::now()),
        }
    }

    fn is_running(&self) -> bool {
        self.started_at.is_some()
    }

    fn seconds(&self) -> f64 {
        self.elapsed
            + self.started_at.map_or(0., |started_at| {
                #[allow(clippy::cast_precision_loss)]
                let seconds = chrono::Utc::now()
                    .signed_duration_since(started_at)
                    .num_milliseconds() as f64
                    / 1000.;
                seconds
            })
    }

    fn toggled(self) -> Self {
        if self.is_running() {
            Self {
                elapsed: self.seconds(),
                started_at: None,
                ..self
            }
        } else {
            Self {
                started_at: Some(chrono::Utc::now()),
                ..self
            }
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn lead_in_ms() -> u32 {
    (METRONOME_START_DELAY * 1000.) as u32
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
    exercise_dialog: Signal<page::exercises::ExerciseDialog>,
    training_session: &domain::TrainingSession,
    recent_session_sets_by_exercise: &HashMap<domain::ExerciseID, SessionSets>,
    exercises: &[domain::Exercise],
    settings: Settings,
    cache: Cache,
    mut element_elements: Signal<HashMap<usize, web_sys::Element>>,
    expanded_history: Signal<HashSet<usize>>,
    phase_clock: PhaseClock,
) -> Element {
    let sections = training_session.compute_sections();
    let section_element_offsets = sections
        .iter()
        .scan(0, |offset, section| {
            let first_element_idx = *offset;
            *offset += section.elements().len();
            Some(first_element_idx)
        })
        .collect::<Vec<_>>();
    let progress_element_idx = progress.read().element_idx;
    let progress_section_idx = training_session.section_idx(progress_element_idx);
    let progress_section_idx_lookahead =
        training_session.section_idx_lookahead(progress_element_idx);
    let sets_by_exercise = DOMAIN_SERVICE().get_sets_by_exercise(training_session);
    let set_indices = training_session.set_indices();
    let shows_time = settings.show_tut() || training_session.has_time();
    let shows_rpe = settings.show_rpe() || training_session.has_rpe();
    let rows = sections.iter().enumerate().map(|(section_idx, section)| {
        let is_current_section = !focus.show_active_focus
            || section_idx == progress_section_idx
            || section_idx == progress_section_idx_lookahead;
        let exercise_ids = unique(section.exercise_ids());
        let exercise_ids_len = exercise_ids.len();
        let first_element_idx = section_element_offsets[section_idx];
        let exercise_names = exercise_ids.iter().enumerate().map(|(i, id)| {
            let name = exercise_name(*id, exercises);
            let number = exercise_number(id, &exercise_ids);
            let notes = exercises
                .iter()
                .find(|e| e.id == *id)
                .map(|e| e.notes.clone())
                .unwrap_or_default();
            let notes_are_empty = notes.is_empty();
            let note = training_session.exercise_notes.get(id).cloned().unwrap_or_default();
            let note_is_empty = note.is_empty();
            let exercise_id = *id;
            let is_last_exercise = i == exercise_ids_len - 1;
            rsx! {
                tr {
                    class: if is_current_section { "" } else { "is-semitransparent" },
                    td {
                        class: "has-text-centered has-text-weight-bold",
                        class: if i == 0 { "pt-2" },
                        class: if is_last_exercise && notes_are_empty && note_is_empty { "pb-1" },
                        colspan: 6,
                        if let Some(number) = number {
                            span{
                                class: "px-1",
                                "{exercise_marker(number)}"
                            }
                        }
                        Link {
                            class: "px-1",
                            "data-testid": "exercise-name",
                            to: Route::Exercise { id: *id },
                            "{name}"
                        }
                        a {
                            class: "px-1 is-link",
                            "data-testid": "item-options",
                            onclick: eh!(training_session, first_element_idx, exercise_id; {
                                *edit_dialog.write() = EditDialog::Options {
                                    training_session: training_session.clone(),
                                    section_idx,
                                    element_idx: first_element_idx,
                                    exercise_id,
                                }
                            }),
                            Icon { name: "ellipsis-vertical" }
                        }
                    }
                }
                if !notes_are_empty {
                    tr {
                        class: if is_current_section { "" } else { "is-semitransparent" },
                        td {
                            class: "px-2",
                            class: if is_last_exercise && note_is_empty { "pb-1" },
                            colspan: 6,
                            div {
                                class: "is-clickable is-italic is-size-7 has-text-grey has-text-centered is-preserving-line-breaks",
                                "data-testid": "exercise-notes",
                                onclick: eh!(mut exercise_dialog; {
                                    if let CacheState::Ready(exercises) = &*cache.exercises.read()
                                        && let Some(exercise) = exercises.iter().find(|e| e.id == exercise_id).cloned()
                                    {
                                        *exercise_dialog.write() = page::exercises::ExerciseDialog::EditNotes {
                                            exercise,
                                        };
                                    }
                                }),
                                { notes.clone() }
                            }
                        }
                    }
                }
                if !note_is_empty {
                    tr {
                        class: if is_current_section { "" } else { "is-semitransparent" },
                        td {
                            class: "px-2",
                            class: if is_last_exercise { "pb-1" },
                            colspan: 6,
                            div {
                                class: "is-clickable is-italic has-text-centered",
                                "data-testid": "session-exercise-notes",
                                onclick: eh!(mut edit_dialog; training_session; {
                                    *edit_dialog.write() = EditDialog::SessionExerciseNotes {
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

        let shows_column_header = {
            let field_values = field_values.read();
            section.elements().iter().enumerate().any(|(i, element)| {
                let domain::TrainingSessionElement::Set {
                    target_reps,
                    target_tempo,
                    automatic,
                    ..
                } = element
                else {
                    return false;
                };
                field_values
                    .get(&(first_element_idx + i))
                    .is_some_and(|set_field_values| {
                        countdown_total(
                            *target_reps,
                            *target_tempo,
                            *automatic,
                            set_field_values,
                            focus,
                        )
                        .is_none()
                    })
            })
        };

        let sets = section.elements().iter().enumerate().map(|(i, element)| {
            let element_idx = first_element_idx + i;
            let set = match element {
                domain::TrainingSessionElement::Set { exercise_id, target_reps, target_tempo, target_weight, target_rpe, automatic, .. } => {
                    let set_index = set_indices[&element_idx];
                    let set_field_values = &field_values.read()[&element_idx];

                    let show_set_buttons = is_current_section && (set_field_values.is_empty() || set_field_values.changed());

                    let history: &[_] = if show_set_buttons {
                        recent_session_sets_by_exercise
                            .get(exercise_id)
                            .map_or(&[], Vec::as_slice)
                    } else {
                        &[]
                    };

                    let mut set_buttons: IndexMap<domain::Set, SetButton> = IndexMap::new();
                    if show_set_buttons {
                        if target_reps.non_zero().is_some() || target_tempo.non_zero().is_some() || target_weight.non_zero().is_some() || target_rpe.non_zero().is_some() {
                            let button = set_buttons.entry(domain::Set {
                                reps: *target_reps,
                                time: target_tempo.seconds_per_rep(),
                                weight: *target_weight,
                                rpe: *target_rpe,
                            }).or_default();
                            button.icons.push("bullseye".to_string());
                            button.label = Some(element.target_to_string(shows_rpe));
                        }
                        let previous_set = set_index.checked_sub(*exercise_counts.get(exercise_id).unwrap_or(&1)).and_then(|previous_set_index| sets_by_exercise.get(exercise_id).and_then(|set| set.get(previous_set_index).and_then(|e| e.set())));
                        if let Some(set) = previous_set {
                            set_buttons.entry(set).or_default().icons.push("arrow-turn-down".to_string());
                        }
                        if let Some(domain::Set { reps, time, weight, rpe }) = history.first().and_then(|(_, sets)| sets.get(set_index)).cloned() {
                            set_buttons.entry(domain::Set { reps, time, weight, rpe }).or_default().icons.push("calendar-minus".to_string());
                        }
                    }

                    let number = exercise_number(exercise_id, &exercise_ids);
                    let recorded = (*target_reps, target_tempo.seconds_per_rep(), *target_weight);

                    match countdown_total(*target_reps, *target_tempo, *automatic, set_field_values, focus) {
                        None => rsx! {
                            tr {
                                class: if is_current_section { "" } else { "is-semitransparent" },
                                td {
                                    class: "p-1",
                                    style: "vertical-align: middle",
                                    "data-testid": "set-number",
                                    if let Some(number) = number {
                                        "{exercise_marker(number)}"
                                    }
                                }
                                td {
                                    class: "p-1 has-text-centered",
                                    InputField {
                                        inputmode: "numeric",
                                        "aria-label": "Reps",
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
                                    class: "p-1 has-text-centered",
                                    if shows_time {
                                        InputField {
                                            inputmode: "numeric",
                                            "aria-label": "Time (s)",
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
                                    class: "p-1 has-text-centered",
                                    InputField {
                                        inputmode: "decimal",
                                        "aria-label": "Weight (kg)",
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
                                    class: "p-1 has-text-centered",
                                    if shows_rpe {
                                        InputField {
                                            inputmode: "decimal",
                                            "aria-label": "RPE",
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
                                                    modify_training_session_elements(&mut training_session, &field_values.read());
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
                            if focus.is_focused(element_idx) && target_tempo.non_zero().is_some() {
                                tr {
                                    td {}
                                    td {
                                        class: "px-1",
                                        colspan: 4,
                                        SetTempoBar {
                                            element_idx,
                                            target_tempo: *target_tempo,
                                            phase_clock,
                                        }
                                    }
                                    td {}
                                }
                            }
                            if is_current_section {
                                {set_value_buttons(set_buttons, history, set_index, element_idx, field_values, expanded_history, shows_time, shows_rpe)}
                            }
                        },
                        Some(total) => rsx! {
                            tr {
                                class: if is_current_section { "" } else { "is-semitransparent" },
                                td {
                                    class: "p-1",
                                    style: "vertical-align: middle",
                                    "data-testid": "set-number",
                                    if let Some(number) = number {
                                        "{exercise_marker(number)}"
                                    }
                                }
                                td {
                                    class: "p-1",
                                    colspan: 4,
                                    div {
                                        class: "notification is-link has-text-centered px-6 pt-1",
                                        // The bottom padding of the focused block is the space above its phase bar
                                        class: if focus.is_focused(element_idx) { "is-size-1 pb-3 has-phase-bar" } else { "pb-1" },
                                        if focus.is_focused(element_idx) {
                                            SetCountdown {
                                                target_reps: *target_reps,
                                                target_tempo: *target_tempo,
                                                total,
                                                progress,
                                                phase_clock,
                                            }
                                        } else {
                                            div {
                                                onclick: move |_| {
                                                    if focus.other_session_running() {
                                                        return;
                                                    }
                                                    focus.take_ownership();
                                                    progress.write().set_element_idx(element_idx);
                                                },
                                                "{total} s"
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
                                        onclick: eh!(mut training_session; recorded; {
                                            if focus.is_focused(element_idx) {
                                                progress.write().set_element_idx(element_idx + 1);
                                                if let Some(set_field_values) = field_values.write().get_mut(&element_idx) {
                                                    let (reps, time, weight) = recorded;
                                                    set_field_values.fill(reps, time, weight);
                                                }
                                                modify_training_session_elements(&mut training_session, &field_values.read());
                                                spawn(async move {
                                                    save(training_session.clone(), cache, || {}).await;
                                                });
                                            } else {
                                                focus.take_ownership();
                                                progress.write().set_element_idx(element_idx);
                                            }
                                        }),
                                        Icon { name: if focus.is_focused(element_idx) { "check" } else { "angles-left" } }
                                    }
                                }
                            }
                            if is_current_section {
                                {set_value_buttons(set_buttons, history, set_index, element_idx, field_values, expanded_history, shows_time, shows_rpe)}
                            }
                        },
                    }
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
                                    if let Some(target_time) = target_time.non_zero() {
                                        div {
                                            // The bottom padding is the space above the phase bar
                                            class: "notification is-success is-size-1 has-text-centered px-1 pt-1 pb-3 has-phase-bar",
                                            Timer { timer: progress.timer_service() }
                                            PhaseBar {
                                                class: "phase-bar-pinned",
                                                phases: vec![u32::from(target_time)],
                                                position: Some((0, f64::from(u32::from(target_time)) - *phase_clock.elapsed.read())),
                                            }
                                        }
                                    } else {
                                        div {
                                            class: "notification is-success is-size-1 has-text-centered p-1",
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
                                        },
                                        if let Some(target_time) = target_time.non_zero() {
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
            (element_idx, set)
        });
        rsx! {
            tbody {
                for name in exercise_names {
                    {name}
                }
                if shows_column_header {
                    tr {
                        class: if is_current_section { "" } else { "is-semitransparent" },
                        "data-testid": "column-header",
                        td {}
                        th { class: COLUMN_HEADER_CLASS, scope: "col", "Reps" }
                        th {
                            class: COLUMN_HEADER_CLASS,
                            scope: "col",
                            if shows_time { "Time (s)" }
                        }
                        th { class: COLUMN_HEADER_CLASS, scope: "col", "Weight (kg)" }
                        th {
                            class: COLUMN_HEADER_CLASS,
                            scope: "col",
                            if shows_rpe { "RPE" }
                        }
                        td {}
                    }
                }
            }
            for (element_idx, set) in sets {
                tbody {
                    class: if settings.scroll_snapping() { "element-snap" },
                    class: if settings.scroll_snapping() && focus.is_focused(element_idx) { "element-snap-current" },
                    onmounted: move |event| {
                        if let Some(element) = event.data().try_as_web_event() {
                            element_elements.write().insert(element_idx, element);
                        }
                    },
                    {set}
                }
            }
        }
    });

    rsx! {
        Block {
            table {
                "data-testid": "session",
                class: "mx-auto has-equal-input-widths",
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

/// The seconds a set is counted down for, or `None` when it is rendered with its input fields.
fn countdown_total(
    target_reps: domain::Reps,
    target_tempo: domain::Tempo,
    automatic: bool,
    set_field_values: &SetFieldValues,
    focus: SetFocus,
) -> Option<u32> {
    if !set_field_values.is_unperformed() || focus.other_session_running() {
        return None;
    }
    countdown_seconds(target_reps, target_tempo, automatic)
}

/// The seconds a countdown of a set runs for, whatever the set already carries.
///
/// A set that prescribes repetitions is counted down only when it starts automatically.
fn countdown_seconds(
    target_reps: domain::Reps,
    target_tempo: domain::Tempo,
    automatic: bool,
) -> Option<u32> {
    let seconds_per_rep = u32::from(target_tempo.seconds_per_rep().non_zero()?);
    match target_reps.non_zero() {
        Some(reps) => automatic.then(|| u32::from(reps) * seconds_per_rep),
        None => Some(seconds_per_rep),
    }
}

/// The countdown of a set, showing the phase it is in and how far it has got.
#[component]
fn SetCountdown(
    target_reps: domain::Reps,
    target_tempo: domain::Tempo,
    total: u32,
    progress: Store<Progress>,
    phase_clock: PhaseClock,
) -> Element {
    let timer = progress.timer_service();
    // A countdown about to start automatically is not waiting for a tap.
    let is_running = timer.read().is_active() || *phase_clock.countdown_start_pending.read();
    // The countdown is set after the set has become current, so the total stands in until then.
    let remaining = if timer.read().is_set() {
        timer.read().seconds()
    } else {
        i64::from(total)
    };
    // The moment the countdown reaches zero already belongs to the next repetition, whose first
    // phase would replace the one that just ended.
    #[allow(clippy::cast_precision_loss)]
    let elapsed = (i64::from(total) - remaining.max(1)).max(0) as f64;
    let phase = target_tempo.phase_at(elapsed);
    let repetition = phase.map_or(0, |(repetition, _, _)| repetition) + 1;

    rsx! {
        div {
            "data-testid": "countdown",
            class: if is_running { "" } else { "is-blinking" },
            onclick: move |_| {
                let mut countdown_starts = phase_clock.countdown_starts;
                countdown_starts += 1;
                let mut start_pending = phase_clock.countdown_start_pending;
                start_pending.set(false);
                progress.timer_service().write().start_pause();
            },
            div { "{phase.map_or(remaining, |(_, _, remaining)| remaining.ceil() as i64)} s" }
            if let Some(reps) = target_reps.non_zero() {
                div { class: "is-size-6", "data-testid": "countdown-detail", "{repetition}/{reps}" }
            }
        }
        PhaseBar {
            class: "phase-bar-pinned",
            phases: target_tempo.phases().iter().copied().map(u32::from).collect::<Vec<_>>(),
            position: target_tempo
                .phase_at((*phase_clock.elapsed.read()).max(0.))
                .map(|(_, index, remaining)| (index, remaining)),
        }
    }
}

/// The tempo bar of a set performed with its input fields, which the user starts and holds by
/// tapping.
#[component]
fn SetTempoBar(
    element_idx: usize,
    target_tempo: domain::Tempo,
    phase_clock: PhaseClock,
) -> Element {
    let is_running = phase_clock.bar_is_running(element_idx);
    let has_started = phase_clock
        .bar
        .read()
        .is_some_and(|bar| bar.element_idx == element_idx);
    let wake_lock = use_hook(|| Rc::new(RefCell::new(None::<Rc<WakeLock>>)));
    *wake_lock.borrow_mut() = is_running.then(wake_lock::hold);

    rsx! {
        div {
            "data-testid": "set-tempo-bar",
            // Vertical padding, so the thin bar can be hit with a finger
            class: "py-2",
            class: if has_started && !is_running { "is-blinking" },
            onclick: move |_| {
                let mut phase_clock = phase_clock;
                phase_clock.toggle_bar(element_idx);
            },
            PhaseBar {
                phases: target_tempo.phases().iter().copied().map(u32::from).collect::<Vec<_>>(),
                position: has_started
                    .then(|| target_tempo.phase_at((*phase_clock.elapsed.read()).max(0.)))
                    .flatten()
                    .map(|(_, index, remaining)| (index, remaining)),
            }
        }
    }
}

/// Renders the row of buttons that prefill a set with the target, previous-set,
/// or previous-session values, and the sets of the recent sessions of the exercise.
///
/// The sets of the recent sessions are shown when the caret at the end of the row is activated.
/// Sets that do not share the position of the set are de-emphasized.
#[allow(clippy::too_many_arguments)]
fn set_value_buttons(
    set_buttons: IndexMap<domain::Set, SetButton>,
    history: &[(chrono::NaiveDate, Vec<domain::Set>)],
    set_index: usize,
    element_idx: usize,
    field_values: Signal<HashMap<usize, SetFieldValues>>,
    expanded_history: Signal<HashSet<usize>>,
    shows_time: bool,
    shows_rpe: bool,
) -> Element {
    let is_expanded = expanded_history.read().contains(&element_idx);
    let show_history = !history.is_empty();
    let last_session_index = history.len().saturating_sub(1);

    rsx! {
        tr {
            td {}
            td {
                class: "p-1",
                colspan: 4,
                div {
                    class: "is-flex is-flex-wrap-wrap is-justify-content-center is-flex-gap-row-gap-1",
                    for (set, button) in set_buttons {
                        {set_value_button(&set, Some(button.icons[0].clone()), button.label, false, element_idx, field_values, shows_time, shows_rpe)}
                    }
                    if show_history {
                        {history_caret(is_expanded, element_idx, expanded_history)}
                    }
                }
            }
        }
        if is_expanded && show_history {
            tr {
                td {}
                td {
                    class: "p-1 has-text-centered",
                    colspan: 4,
                    for (session_index, (date, sets)) in history.iter().enumerate() {
                        div {
                            class: if session_index < last_session_index { "mb-2" },
                            "data-testid": "set-history-session",
                            div { class: "is-size-7", "{date}" }
                            div {
                                class: "is-flex is-flex-wrap-wrap is-justify-content-center is-flex-gap-row-gap-1",
                                for (index, set) in sets.iter().cloned().enumerate() {
                                    {set_value_button(&set, None, None, sets.len() > set_index && index != set_index, element_idx, field_values, shows_time, shows_rpe)}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The icons of a button prefilling a set and the label it carries in place of its values.
#[derive(Default)]
struct SetButton {
    icons: Vec<String>,
    label: Option<String>,
}

/// Renders a button that prefills the set of `element_idx` with `set`.
#[allow(clippy::too_many_arguments)]
fn set_value_button(
    set: &domain::Set,
    icon: Option<String>,
    label: Option<String>,
    is_dimmed: bool,
    element_idx: usize,
    field_values: Signal<HashMap<usize, SetFieldValues>>,
    shows_time: bool,
    shows_rpe: bool,
) -> Element {
    let has_no_icon = icon.is_none();
    let label = label.unwrap_or_else(|| set.to_string(shows_time, shows_rpe));
    let label = if label.is_empty() && has_no_icon {
        "–".to_string()
    } else {
        label
    };
    let set = set.clone();
    rsx! {
        button {
            class: "button is-small mr-1",
            class: if is_dimmed { "is-semitransparent" },
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
            if let Some(icon) = icon {
                Icon { name: icon, is_small: true }
            }
            span { {label} },
        }
    }
}

/// Renders the caret that shows or hides the sets of the recent sessions of an exercise.
fn history_caret(
    is_expanded: bool,
    element_idx: usize,
    mut expanded_history: Signal<HashSet<usize>>,
) -> Element {
    rsx! {
        button {
            class: "button is-small",
            "data-testid": "set-history",
            onclick: move |_| {
                let mut expanded = expanded_history.write();
                if !expanded.remove(&element_idx) {
                    expanded.insert(element_idx);
                }
            },
            Icon { name: if is_expanded { "chevron-up" } else { "chevron-down" }, is_small: true }
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
                TextAreaField {
                    value: input,
                    has_changed: changed,
                    "data-testid": "session-notes",
                    on_input: move |event: FormEvent| {
                        if let Some(n) = notes.write().as_mut() {
                            n.input = event.value();
                            n.validated = Ok(event.value());
                        }
                    },
                }
            }
        }
    } else {
        rsx! {
            if !orig.is_empty() {
                CenteredBlock {
                    Title { "Notes" },
                    p {
                        class: "is-preserving-line-breaks",
                        "data-testid": "session-notes-text",
                        { orig }
                    }
                }
            }
        }
    }
}

fn view_list(
    training_session: &domain::TrainingSession,
    exercises: &[domain::Exercise],
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
                                "{exercise_marker(number)}"
                            }
                        }
                        Link {
                            class: "px-1",
                            "data-testid": "exercise-name",
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
                                "data-testid": "session-exercise-notes",
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
                                if *reps == domain::Reps::default() && *time == domain::Time::default() && *weight == domain::Weight::default() && *rpe == domain::RPE::ZERO {
                                    td {
                                        class: "px-2 has-text-centered",
                                        colspan: 5,
                                        if let Some(number) = number {
                                            span {
                                                class: "pr-2",
                                                "{exercise_marker(number)} "
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
                                            "{exercise_marker(number)}"
                                        }
                                    }
                                    td {
                                        class: "px-2 has-text-right",
                                        if *reps > domain::Reps::default() {
                                            "{reps} ×"
                                        }
                                    }
                                    td {
                                        class: "px-2 has-text-right",
                                        if *time > domain::Time::default() {
                                            "{time} s"
                                        }
                                    }
                                    td {
                                        class: "px-2 has-text-right",
                                        if *weight > domain::Weight::default() {
                                            "{weight} kg"
                                        }
                                    }
                                    td {
                                        class: "px-2",
                                        if *rpe > domain::RPE::ZERO {
                                            " @ {rpe}"
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
                "data-testid": "session",
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

#[allow(clippy::too_many_arguments)]
fn view_edit_dialog(
    mut edit_dialog: Signal<EditDialog>,
    exercise_dialog: Signal<page::exercises::ExerciseDialog>,
    field_values: Signal<HashMap<usize, SetFieldValues>>,
    drop_set_target: Signal<Option<Vec<usize>>>,
    progress: Store<Progress>,
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
            exercise_id,
        } => {
            let exercise_id = *exercise_id;
            rsx! {
                if IS_LOADING() {
                    LoadingDialog {}
                } else {
                    OptionsMenu {
                        options: vec![
                            rsx! {
                                {
                                    let exercise = if let CacheState::Ready(exercises) = &*cache.exercises.read() {
                                        exercises.iter().find(|e| e.id == exercise_id).cloned()
                                    } else {
                                        None
                                    };
                                    rsx! {
                                        if let Some(exercise) = exercise {
                                            MenuOption {
                                                icon: "note-sticky".to_string(),
                                                text: "Edit exercise notes".to_string(),
                                                "data-testid": "options-edit-exercise-notes",
                                                on_click: eh!(mut edit_dialog, exercise_dialog; exercise; {
                                                    *edit_dialog.write() = EditDialog::None;
                                                    *exercise_dialog.write() = page::exercises::ExerciseDialog::EditNotes { exercise };
                                                })
                                            }
                                        }
                                    }
                                },
                                MenuOption {
                                    icon: "file-lines".to_string(),
                                    text: "Show notes for this session".to_string(),
                                    "data-testid": "options-show-session-notes",
                                    on_click: eh!(mut edit_dialog; training_session, exercise_id; {
                                        *edit_dialog.write() = EditDialog::SessionExerciseNotes { training_session, exercise_id };
                                    })
                                },
                                {
                                    let recent_best_set = domain::most_recent_best_set_for_one_rep_max(
                                        training_sessions,
                                        exercise_id,
                                    );
                                    let elements = training_session.run_element_indices(
                                        *section_idx,
                                        exercise_id,
                                        progress.read().element_idx,
                                    );
                                    rsx! {
                                        if let Some((reps, weight)) = recent_best_set {
                                            MenuOption {
                                                icon: "dumbbell".to_string(),
                                                text: "Calculate 1RM".to_string(),
                                                "data-testid": "options-1rm",
                                                on_click: eh!(mut edit_dialog; {
                                                    let mut state = OneRepMaxCalculatorState::new(reps.into(), f32::from(weight));
                                                    state.visible = true;
                                                    *ONE_REP_MAX_CALCULATOR.write() = state;
                                                    *edit_dialog.write() = EditDialog::None;
                                                })
                                            }
                                        }
                                        if elements.len() > 1 {
                                            MenuOption {
                                                icon: "arrow-down-wide-short".to_string(),
                                                text: "Calculate drop sets".to_string(),
                                                "data-testid": "options-drop-set",
                                                on_click: eh!(mut edit_dialog, drop_set_target; training_session, recent_best_set, field_values, elements; {
                                                    let start_weight = entered_weight_of(&field_values.read(), elements.first().copied())
                                                        .or_else(|| recent_best_set.map(|(_, weight)| weight))
                                                        .or_else(|| target_weight_of(&training_session, elements.first().copied()));
                                                    DROP_SET_CALCULATOR.write().start_weight = start_weight.map_or(0.0, f32::from);
                                                    drop_set_target.set(Some(elements));
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
                                        modify_training_session_elements(&mut training_session, &field_values.read());
                                        training_session.add_set(element_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "plus".to_string(),
                                    text: "Add same exercise".to_string(),
                                    on_click: eh!(mut training_session; section_idx, exercise_id, close_dialog; {
                                        modify_training_session_elements(&mut training_session, &field_values.read());
                                        training_session.add_exercise(section_idx, exercise_id);
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
                                        modify_training_session_elements(&mut training_session, &field_values.read());
                                        training_session.move_section_up(section_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "arrow-down".to_string(),
                                    text: "Move down".to_string(),
                                    on_click: eh!(mut training_session; section_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, &field_values.read());
                                        training_session.move_section_down(section_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "arrow-right-arrow-left".to_string(),
                                    text: "Replace exercise".to_string(),
                                    "data-testid": "options-replace-exercise",
                                    on_click: eh!(mut edit_dialog; training_session, section_idx, exercise_id; {
                                        *edit_dialog.write() = EditDialog::ReplaceExercise { training_session, section_idx, exercise_id };
                                    })
                                },
                                MenuOption {
                                    icon: "times".to_string(),
                                    text: "Remove set".to_string(),
                                    on_click: eh!(mut training_session; section_idx, close_dialog; {
                                        modify_training_session_elements(&mut training_session, &field_values.read());
                                        training_session.remove_set(section_idx);
                                        save(training_session, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "times".to_string(),
                                    text: "Remove exercise".to_string(),
                                    "data-testid": "options-remove-exercise",
                                    on_click: eh!(mut training_session; section_idx, exercise_id, close_dialog; {
                                        modify_training_session_elements(&mut training_session, &field_values.read());
                                        training_session.remove_exercise(section_idx, exercise_id);
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
                            filter: domain::ExerciseFilter::default(),
                            on_exercise_click: {
                                let training_session = training_session.clone();
                                let section_idx = *section_idx;
                                move |(_, exercise_id)| {
                                    let mut training_session = training_session.clone();
                                    modify_training_session_elements(&mut training_session, &field_values.read());
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
            exercise_id,
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
                            filter: page::exercises::replacement_filter(*exercise_id, &cache),
                            on_exercise_click: {
                                let training_session = training_session.clone();
                                let section_idx = *section_idx;
                                let exercise_id = *exercise_id;
                                move |(_, replacement)| {
                                    let mut training_session = training_session.clone();
                                    modify_training_session_elements(&mut training_session, &field_values.read());
                                    training_session.replace_exercise(section_idx, exercise_id, replacement);
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
                            filter: domain::ExerciseFilter::default(),
                            on_exercise_click: {
                                let training_session = training_session.clone();
                                move |(_, exercise_id)| {
                                    let mut training_session = training_session.clone();
                                    modify_training_session_elements(&mut training_session, &field_values.read());
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
        EditDialog::SessionExerciseNotes {
            training_session,
            exercise_id,
        } => {
            rsx! {
                SessionExerciseNotesDialog {
                    training_session: training_session.clone(),
                    exercise_id: *exercise_id,
                    on_save: move |ts| save(ts, cache, close_dialog),
                    on_close: eh!(mut close_dialog; { close_dialog(); }),
                }
            }
        }
    }
}

/// Returns the target weight of the element at `element_idx`, if it is set.
fn target_weight_of(
    training_session: &domain::TrainingSession,
    element_idx: Option<usize>,
) -> Option<domain::Weight> {
    match training_session.elements.get(element_idx?) {
        Some(domain::TrainingSessionElement::Set { target_weight, .. }) => target_weight.non_zero(),
        _ => None,
    }
}

/// Returns the weight entered for the element at `element_idx`, if it is valid and not zero.
fn entered_weight_of(
    field_values: &HashMap<usize, SetFieldValues>,
    element_idx: Option<usize>,
) -> Option<domain::Weight> {
    field_values
        .get(&element_idx?)?
        .weight
        .validated
        .as_ref()
        .ok()
        .and_then(|weight| weight.non_zero())
}

/// Writes `weights` into the weight fields of `elements`, pairing them in order.
///
/// Surplus weights and surplus elements are left alone.
fn fill_weights(
    field_values: &mut HashMap<usize, SetFieldValues>,
    elements: &[usize],
    weights: &[domain::Weight],
) {
    for (element_idx, weight) in elements.iter().zip(weights) {
        if let Some(set_field_values) = field_values.get_mut(element_idx) {
            set_field_values.weight.input = weight.to_string();
            set_field_values.weight.validated = Ok(*weight);
        }
    }
}

#[component]
fn SessionExerciseNotesDialog(
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
            title: rsx! { "Notes for this session" },
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
            TextAreaField {
                value: note.clone(),
                has_changed: changed,
                autofocus: true,
                "data-testid": "session-exercise-notes-input",
                on_input: move |event: FormEvent| {
                    *note_input.write() = event.value();
                },
                on_mounted: move |textarea: web_sys::HtmlTextAreaElement| {
                    textarea_element.set(Some(textarea));
                },
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
                            "data-testid": "previous-session-exercise-note",
                            {note.clone()}
                            button {
                                class: "button is-overlay-top-right p-0 mr-2",
                                r#type: "button",
                                "data-testid": "session-exercise-notes-reuse",
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

/// Returns the marker of the zero-based exercise number, as a circled digit where one exists.
fn exercise_marker(number: u32) -> String {
    if number < 20 {
        std::char::from_u32(0x2460 + number).map(String::from)
    } else {
        None
    }
    .unwrap_or_else(|| format!("({})", number + 1))
}

fn modify_training_session_elements(
    training_session: &mut domain::TrainingSession,
    field_values: &HashMap<usize, SetFieldValues>,
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
            *reps = set_field_values.reps.validated.clone().unwrap_or_default();
            *time = set_field_values.time.validated.clone().unwrap_or_default();
            *weight = set_field_values
                .weight
                .validated
                .clone()
                .unwrap_or_default();
            *rpe = set_field_values.rpe.validated.clone().unwrap_or_default();
        }
    }
}

async fn save(
    training_session: domain::TrainingSession,
    cache: Cache,
    mut close_dialog: impl FnMut(),
) {
    let _loading = LoadingFlag::set(&IS_LOADING);
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
            notify("modify training session", &err);
        }
    }
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
        self.timer_service.unset();
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
        exercise_id: domain::ExerciseID,
    },
    AddExercise {
        training_session: domain::TrainingSession,
        section_idx: usize,
    },
    ReplaceExercise {
        training_session: domain::TrainingSession,
        section_idx: usize,
        exercise_id: domain::ExerciseID,
    },
    AppendExercise {
        training_session: domain::TrainingSession,
    },
    SessionExerciseNotes {
        training_session: domain::TrainingSession,
        exercise_id: domain::ExerciseID,
    },
}

#[cfg(test)]
mod tests {

    use crate::{
        ongoing_training_session::State,
        test_render::{
            TestCache, all_attributes_of, all_text_of, attribute_of, cells_of, contains,
            provide_ongoing_training_session, provide_settings, render, rows_of, text_of,
        },
    };

    fn recorded_set(exercise_id: u128) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            reps: domain::Reps::new(10).unwrap(),
            time: domain::Time::new(30).unwrap(),
            weight: domain::Weight::new(50.0).unwrap(),
            rpe: domain::RPE::new(8.0).unwrap(),
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::ZERO,
            automatic: false,
        }
    }

    fn recorded_session() -> TestCache {
        TestCache::default()
            .with_exercises(vec![exercise(1, "Squat")])
            .with_training_sessions(vec![domain::TrainingSession {
                id: 1.into(),
                routine_id: 1.into(),
                date: chrono::Local::now().date_naive(),
                notes: String::new(),
                elements: vec![recorded_set(1), recorded_set(1)],
                exercise_notes: std::collections::BTreeMap::new(),
            }])
    }

    fn planned_session() -> TestCache {
        TestCache::default()
            .with_exercises(vec![exercise(1, "Squat")])
            .with_training_sessions(vec![domain::TrainingSession {
                id: 1.into(),
                routine_id: 1.into(),
                date: chrono::Local::now().date_naive(),
                notes: String::new(),
                elements: vec![domain::TrainingSessionElement::Set {
                    exercise_id: 1.into(),
                    reps: domain::Reps::default(),
                    time: domain::Time::default(),
                    weight: domain::Weight::default(),
                    rpe: domain::RPE::ZERO,
                    target_reps: domain::Reps::new(10).unwrap(),
                    target_tempo: domain::Tempo::default(),
                    target_weight: domain::Weight::default(),
                    target_rpe: domain::RPE::ZERO,
                    automatic: false,
                }],
                exercise_notes: std::collections::BTreeMap::new(),
            }])
    }

    fn timed_session() -> TestCache {
        TestCache::default()
            .with_exercises(vec![exercise(1, "Plank")])
            .with_training_sessions(vec![domain::TrainingSession {
                id: 1.into(),
                routine_id: 1.into(),
                date: chrono::Local::now().date_naive(),
                notes: String::new(),
                elements: vec![domain::TrainingSessionElement::Set {
                    exercise_id: 1.into(),
                    reps: domain::Reps::default(),
                    time: domain::Time::default(),
                    weight: domain::Weight::default(),
                    rpe: domain::RPE::ZERO,
                    target_reps: domain::Reps::default(),
                    target_tempo: domain::Tempo::new(&[60]).unwrap(),
                    target_weight: domain::Weight::default(),
                    target_rpe: domain::RPE::ZERO,
                    automatic: false,
                }],
                exercise_notes: std::collections::BTreeMap::new(),
            }])
    }

    fn render_training_session(
        id: u128,
        settings: web_app::Settings,
        cache: impl Fn() -> TestCache + 'static,
    ) -> String {
        render(move || {
            cache().provide();
            provide_settings(settings);
            provide_ongoing_training_session(State::None);
            rsx! { TrainingSession { id: domain::TrainingSessionID::from(id) } }
        })
    }

    #[test]
    fn test_the_sets_of_the_session_are_shown() {
        let html = render_training_session(1, web_app::Settings::default(), recorded_session);

        assert_eq!(all_text_of(&html, "exercise-name"), vec!["Squat"]);
        assert_eq!(
            rows_of(&html, "session")[1],
            vec!["", "10 ×", "30 s", "50 kg", "@ 8"]
        );
    }

    #[test]
    fn test_the_recorded_values_are_shown_independently_of_the_rpe_and_tut_settings() {
        let html = render_training_session(
            1,
            web_app::Settings {
                show_rpe: false,
                show_tut: false,
                ..web_app::Settings::default()
            },
            recorded_session,
        );

        assert_eq!(
            rows_of(&html, "session")[1],
            vec!["", "10 ×", "30 s", "50 kg", "@ 8"]
        );
    }

    #[test]
    fn test_the_columns_of_the_input_fields_are_labeled() {
        let html = render_training_session(1, web_app::Settings::default(), planned_session);

        assert_eq!(
            cells_of(&html, "column-header"),
            vec!["", "Reps", "Time (s)", "Weight (kg)", "RPE", ""]
        );
    }

    #[test]
    fn test_the_column_labels_follow_the_rpe_and_tut_settings() {
        let html = render_training_session(
            1,
            web_app::Settings {
                show_rpe: false,
                show_tut: false,
                ..web_app::Settings::default()
            },
            planned_session,
        );

        assert_eq!(
            cells_of(&html, "column-header"),
            vec!["", "Reps", "", "Weight (kg)", "", ""]
        );
    }

    #[test]
    fn test_the_columns_of_a_section_without_input_fields_are_not_labeled() {
        let html = render_training_session(1, web_app::Settings::default(), timed_session);

        assert!(!contains(&html, "column-header"), "{html}");
    }

    fn tempo_session(
        phases: &'static [u32],
        target_reps: u32,
        automatic: bool,
        recorded: bool,
    ) -> impl Fn() -> TestCache + 'static {
        move || {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Squat")])
                .with_training_sessions(vec![domain::TrainingSession {
                    id: 1.into(),
                    routine_id: 1.into(),
                    date: chrono::Local::now().date_naive(),
                    notes: String::new(),
                    elements: vec![domain::TrainingSessionElement::Set {
                        exercise_id: 1.into(),
                        reps: if recorded {
                            domain::Reps::new(target_reps).unwrap()
                        } else {
                            domain::Reps::default()
                        },
                        time: domain::Time::default(),
                        weight: domain::Weight::default(),
                        rpe: domain::RPE::ZERO,
                        target_reps: domain::Reps::new(target_reps).unwrap(),
                        target_tempo: domain::Tempo::new(phases).unwrap(),
                        target_weight: domain::Weight::default(),
                        target_rpe: domain::RPE::ZERO,
                        automatic,
                    }],
                    exercise_notes: std::collections::BTreeMap::new(),
                }])
        }
    }

    #[test]
    fn test_a_set_with_a_tempo_shows_the_bar_and_keeps_its_input_fields() {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            tempo_session(&[3, 1, 1, 0], 8, false, false),
        );

        assert!(contains(&html, "set-tempo-bar"), "{html}");
        assert!(!contains(&html, "countdown"), "{html}");
        assert!(contains(&html, "column-header"), "{html}");
    }

    #[test]
    fn test_an_automatic_set_with_a_tempo_is_counted_down() {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            tempo_session(&[3, 1, 1, 0], 8, true, false),
        );

        assert_eq!(text_of(&html, "countdown-detail"), "1/8");
        assert!(!contains(&html, "set-tempo-bar"), "{html}");
    }

    #[test]
    fn test_a_countdown_without_reps_has_no_detail() {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            tempo_session(&[3, 1, 1, 0], 0, true, false),
        );

        assert!(contains(&html, "countdown"), "{html}");
        assert!(!contains(&html, "countdown-detail"), "{html}");
    }

    #[test]
    fn test_a_countdown_awaiting_its_automatic_start_does_not_blink() {
        assert_eq!(countdown_class(true), "");
        assert_eq!(countdown_class(false), "is-blinking");
    }

    /// Renders a countdown that has not been started and returns its class.
    fn countdown_class(start_pending: bool) -> String {
        let html = render(move || rsx! { UnstartedCountdown { start_pending } });

        attribute_of(&html, "countdown", "class")
    }

    #[component]
    fn UnstartedCountdown(start_pending: bool) -> Element {
        let progress = use_store(|| Progress::new(1.into()));
        let phase_clock = PhaseClock {
            bar: use_signal(|| None),
            elapsed: use_signal(|| 0.),
            countdown_starts: use_signal(|| 0),
            countdown_start_pending: use_signal(|| start_pending),
        };

        rsx! {
            SetCountdown {
                target_reps: domain::Reps::new(8).unwrap(),
                target_tempo: domain::Tempo::new(&[3, 1, 1, 0]).unwrap(),
                total: 40,
                progress,
                phase_clock,
            }
        }
    }

    #[test]
    fn test_a_current_rest_shows_a_phase_bar_only_with_a_target_time() {
        for (target_time, has_bar) in [(60, true), (0, false)] {
            let html = render_training_session(1, web_app::Settings::default(), move || {
                rest_session(target_time)
            });

            assert_eq!(contains(&html, "phase-bar"), has_bar, "{html}");
        }
    }

    fn rest_session(target_time: u32) -> TestCache {
        TestCache::default().with_training_sessions(vec![domain::TrainingSession {
            id: 1.into(),
            routine_id: 1.into(),
            date: chrono::Local::now().date_naive(),
            notes: String::new(),
            elements: vec![domain::TrainingSessionElement::Rest {
                target_time: domain::Time::new(target_time).unwrap(),
                automatic: false,
            }],
            exercise_notes: std::collections::BTreeMap::new(),
        }])
    }

    #[test]
    fn test_a_set_without_a_tempo_keeps_its_input_fields() {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            tempo_session(&[], 8, true, false),
        );

        assert!(!contains(&html, "countdown"), "{html}");
        assert!(!contains(&html, "set-tempo-bar"), "{html}");
    }

    #[test]
    fn test_a_recorded_set_is_neither_counted_down_nor_guided() {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            tempo_session(&[3, 1, 1, 0], 8, true, true),
        );

        assert!(!contains(&html, "countdown"), "{html}");
        assert!(!contains(&html, "set-tempo-bar"), "{html}");
    }

    #[test]
    fn test_a_set_with_a_tempo_is_counted_down_without_the_tut_setting() {
        for target_reps in [8, 0] {
            let html = render_training_session(
                1,
                web_app::Settings {
                    show_tut: false,
                    ..web_app::Settings::default()
                },
                tempo_session(&[3, 1, 1, 0], target_reps, true, false),
            );

            assert!(contains(&html, "countdown"), "{html}");
        }
    }

    #[test]
    fn test_a_prescribed_tempo_shows_the_time_column_without_the_tut_setting() {
        let html = render_training_session(
            1,
            web_app::Settings {
                show_tut: false,
                ..web_app::Settings::default()
            },
            tempo_session(&[3, 1, 1, 0], 8, false, false),
        );

        assert_eq!(
            cells_of(&html, "column-header"),
            vec!["", "Reps", "Time (s)", "Weight (kg)", "RPE", ""]
        );
    }

    #[test]
    fn test_a_prescribed_rpe_shows_the_rpe_column_without_the_rpe_setting() {
        let html = render_training_session(
            1,
            web_app::Settings {
                show_rpe: false,
                show_tut: false,
                ..web_app::Settings::default()
            },
            || {
                TestCache::default()
                    .with_exercises(vec![exercise(1, "Squat")])
                    .with_training_sessions(vec![domain::TrainingSession {
                        id: 1.into(),
                        routine_id: 1.into(),
                        date: chrono::Local::now().date_naive(),
                        notes: String::new(),
                        elements: vec![domain::TrainingSessionElement::Set {
                            exercise_id: 1.into(),
                            reps: domain::Reps::default(),
                            time: domain::Time::default(),
                            weight: domain::Weight::default(),
                            rpe: domain::RPE::ZERO,
                            target_reps: domain::Reps::new(10).unwrap(),
                            target_tempo: domain::Tempo::default(),
                            target_weight: domain::Weight::default(),
                            target_rpe: domain::RPE::NINE,
                            automatic: false,
                        }],
                        exercise_notes: std::collections::BTreeMap::new(),
                    }])
            },
        );

        assert_eq!(
            cells_of(&html, "column-header"),
            vec!["", "Reps", "", "Weight (kg)", "RPE", ""]
        );
    }

    #[test]
    fn test_scroll_snapping_follows_the_setting() {
        let with_snapping = render_training_session(
            1,
            web_app::Settings {
                scroll_snapping: true,
                ..web_app::Settings::default()
            },
            planned_session,
        );
        let without = render_training_session(1, web_app::Settings::default(), planned_session);

        assert!(with_snapping.contains("element-snap"), "{with_snapping}");
        assert!(!without.contains("element-snap"), "{without}");
    }

    #[test]
    fn test_the_offered_values_follow_the_shown_columns() {
        let html = render_training_session(
            1,
            web_app::Settings {
                show_rpe: false,
                show_tut: false,
                ..web_app::Settings::default()
            },
            || {
                // The time and the RPE are prescribed for another set, so that the columns are
                // shown although the settings hide them.
                let prescribing_set = domain::TrainingSessionElement::Set {
                    exercise_id: 2.into(),
                    reps: domain::Reps::default(),
                    time: domain::Time::default(),
                    weight: domain::Weight::default(),
                    rpe: domain::RPE::ZERO,
                    target_reps: domain::Reps::new(10).unwrap(),
                    target_tempo: domain::Tempo::new(&[3, 1, 1, 0]).unwrap(),
                    target_weight: domain::Weight::default(),
                    target_rpe: domain::RPE::NINE,
                    automatic: false,
                };
                let recorded_set = domain::TrainingSessionElement::Set {
                    exercise_id: 1.into(),
                    reps: domain::Reps::new(10).unwrap(),
                    time: domain::Time::new(3).unwrap(),
                    weight: domain::Weight::new(50.0).unwrap(),
                    rpe: domain::RPE::EIGHT,
                    target_reps: domain::Reps::default(),
                    target_tempo: domain::Tempo::default(),
                    target_weight: domain::Weight::default(),
                    target_rpe: domain::RPE::ZERO,
                    automatic: false,
                };
                TestCache::default()
                    .with_exercises(vec![exercise(1, "Squat"), exercise(2, "Bench Press")])
                    .with_training_sessions(vec![
                        earlier_session(2, 7, vec![recorded_set]),
                        domain::TrainingSession {
                            id: 1.into(),
                            routine_id: 1.into(),
                            date: chrono::Local::now().date_naive(),
                            notes: String::new(),
                            elements: vec![planned_set(1), prescribing_set],
                            exercise_notes: std::collections::BTreeMap::new(),
                        },
                    ])
            },
        );

        assert_eq!(
            all_text_of(&html, "set-value"),
            vec![
                "10 \u{00d7} 50 kg @ 8 (3 s)",
                "10 @ 9 (3\u{00b7}1\u{00b7}1\u{00b7}0)"
            ]
        );
    }

    fn planned_session_with_history(
        earlier_sessions: Vec<domain::TrainingSession>,
    ) -> impl Fn() -> TestCache + 'static {
        move || {
            let mut training_sessions = earlier_sessions.clone();
            training_sessions.push(domain::TrainingSession {
                id: 1.into(),
                routine_id: 1.into(),
                date: chrono::Local::now().date_naive(),
                notes: String::new(),
                elements: vec![planned_set(1)],
                exercise_notes: std::collections::BTreeMap::new(),
            });
            TestCache::default()
                .with_exercises(vec![exercise(1, "Squat")])
                .with_training_sessions(training_sessions)
        }
    }

    fn planned_set(exercise_id: u128) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            reps: domain::Reps::default(),
            time: domain::Time::default(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::ZERO,
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::ZERO,
            automatic: false,
        }
    }

    fn earlier_session(
        id: u128,
        days_ago: i64,
        elements: Vec<domain::TrainingSessionElement>,
    ) -> domain::TrainingSession {
        domain::TrainingSession {
            id: id.into(),
            routine_id: 1.into(),
            date: chrono::Local::now().date_naive() - chrono::Duration::days(days_ago),
            notes: String::new(),
            elements,
            exercise_notes: std::collections::BTreeMap::new(),
        }
    }

    fn performed_set(exercise_id: u128, weight: f32) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            reps: domain::Reps::new(10).unwrap(),
            time: domain::Time::default(),
            weight: domain::Weight::new(weight).unwrap(),
            rpe: domain::RPE::ZERO,
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::ZERO,
            automatic: false,
        }
    }

    #[test]
    fn test_the_values_of_an_exercise_skipped_in_the_last_session_are_offered() {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            planned_session_with_history(vec![
                earlier_session(2, 7, vec![performed_set(2, 60.0)]),
                earlier_session(3, 14, vec![performed_set(1, 50.0)]),
            ]),
        );

        assert_eq!(all_text_of(&html, "set-value"), vec!["10 × 50 kg"]);
    }

    #[test]
    fn test_the_values_of_an_exercise_are_not_offered_without_an_earlier_session() {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            planned_session_with_history(vec![]),
        );

        assert!(all_text_of(&html, "set-value").is_empty(), "{html}");
        assert!(!contains(&html, "set-history"), "{html}");
    }

    #[test]
    fn test_the_sets_of_the_recent_sessions_can_be_shown() {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            planned_session_with_history(vec![earlier_session(2, 7, vec![performed_set(1, 50.0)])]),
        );

        assert!(contains(&html, "set-history"), "{html}");
    }

    #[component]
    fn ExpandedHistory(
        history: Vec<(chrono::NaiveDate, Vec<domain::Set>)>,
        set_index: usize,
        with_set_buttons: bool,
    ) -> Element {
        let field_values = use_signal(|| HashMap::from([(0, set_field_values(0))]));
        let expanded_history = use_signal(|| HashSet::from([0]));
        let settings = use_context::<Settings>();
        let set_buttons = if with_set_buttons {
            IndexMap::from([(
                domain::Set {
                    reps: domain::Reps::new(10).unwrap(),
                    time: domain::Time::default(),
                    weight: domain::Weight::new(50.0).unwrap(),
                    rpe: domain::RPE::ZERO,
                },
                SetButton {
                    icons: vec!["calendar-minus".to_string()],
                    label: None,
                },
            )])
        } else {
            IndexMap::new()
        };
        rsx! {
            table {
                tbody {
                    {set_value_buttons(
                        set_buttons,
                        &history,
                        set_index,
                        0,
                        field_values,
                        expanded_history,
                        settings.show_tut(),
                        settings.show_rpe(),
                    )}
                }
            }
        }
    }

    fn render_expanded_history(
        history: Vec<(chrono::NaiveDate, Vec<domain::Set>)>,
        set_index: usize,
    ) -> String {
        render_expanded_history_with_set_buttons(history, set_index, true)
    }

    fn render_expanded_history_with_set_buttons(
        history: Vec<(chrono::NaiveDate, Vec<domain::Set>)>,
        set_index: usize,
        with_set_buttons: bool,
    ) -> String {
        render(move || {
            provide_settings(web_app::Settings::default());
            rsx! {
                ExpandedHistory {
                    history: history.clone(),
                    set_index,
                    with_set_buttons,
                }
            }
        })
    }

    fn recent_set(weight: f32) -> domain::Set {
        domain::Set {
            reps: domain::Reps::new(10).unwrap(),
            time: domain::Time::default(),
            weight: domain::Weight::new(weight).unwrap(),
            rpe: domain::RPE::ZERO,
        }
    }

    #[test]
    fn test_the_expanded_history_shows_the_sets_of_every_session_with_its_date() {
        let html = render_expanded_history(
            vec![
                (
                    chrono::NaiveDate::from_ymd_opt(2026, 8, 28).unwrap(),
                    vec![recent_set(50.0), recent_set(52.5)],
                ),
                (
                    chrono::NaiveDate::from_ymd_opt(2026, 8, 21).unwrap(),
                    vec![recent_set(47.5)],
                ),
            ],
            0,
        );

        assert_eq!(
            all_text_of(&html, "set-history-session"),
            vec!["2026-08-2810 × 50 kg10 × 52.5 kg", "2026-08-2110 × 47.5 kg"]
        );
    }

    #[test]
    fn test_the_sets_of_other_positions_are_de_emphasized() {
        let html = render_expanded_history(
            vec![(
                chrono::NaiveDate::from_ymd_opt(2026, 8, 28).unwrap(),
                vec![recent_set(50.0), recent_set(52.5), recent_set(55.0)],
            )],
            1,
        );

        assert_eq!(
            all_attributes_of(&html, "set-value", "class")
                .iter()
                .map(|class| class.contains("is-semitransparent"))
                .collect::<Vec<_>>(),
            vec![false, true, false, true]
        );
    }

    #[test]
    fn test_the_history_can_be_expanded_without_set_buttons() {
        let html = render_expanded_history_with_set_buttons(
            vec![(
                chrono::NaiveDate::from_ymd_opt(2026, 8, 28).unwrap(),
                vec![recent_set(50.0)],
            )],
            0,
            false,
        );

        assert!(contains(&html, "set-history"), "{html}");
        assert_eq!(
            all_text_of(&html, "set-history-session"),
            vec!["2026-08-2810 × 50 kg"]
        );
    }

    #[test]
    fn test_the_sets_of_a_session_without_that_position_are_not_de_emphasized() {
        let html = render_expanded_history(
            vec![(
                chrono::NaiveDate::from_ymd_opt(2026, 8, 28).unwrap(),
                vec![recent_set(50.0), recent_set(52.5)],
            )],
            2,
        );

        assert_eq!(
            all_attributes_of(&html, "set-value", "class")
                .iter()
                .map(|class| class.contains("is-semitransparent"))
                .collect::<Vec<_>>(),
            vec![false, false, false]
        );
    }

    #[test]
    fn test_an_unknown_session_is_reported() {
        let html = render_training_session(2, web_app::Settings::default(), recorded_session);

        assert_eq!(text_of(&html, "error-page"), "Training session not found");
    }

    #[test]
    fn test_unread_sessions_are_shown_as_loading() {
        let html = render_training_session(1, web_app::Settings::default(), TestCache::loading);

        assert!(contains(&html, "loading-page"));
    }

    #[test]
    fn test_unreadable_sessions_are_shown_as_an_error() {
        let html = render_training_session(1, web_app::Settings::default(), TestCache::failing);

        assert_eq!(text_of(&html, "error-page"), "No connection");
    }
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    fn set(exercise_id: u128, reps: u32) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            reps: domain::Reps::new(reps).unwrap(),
            time: domain::Time::default(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::default(),
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::default(),
            automatic: false,
        }
    }

    fn set_with_target_weight(
        exercise_id: u128,
        target_weight: f32,
    ) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            reps: domain::Reps::default(),
            time: domain::Time::default(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::default(),
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::new(target_weight).unwrap(),
            target_rpe: domain::RPE::default(),
            automatic: false,
        }
    }

    fn rest() -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Rest {
            target_time: domain::Time::default(),
            automatic: false,
        }
    }

    fn training_session(elements: Vec<domain::TrainingSessionElement>) -> domain::TrainingSession {
        domain::TrainingSession {
            id: 1.into(),
            routine_id: 1.into(),
            date: chrono::NaiveDate::default(),
            notes: String::new(),
            elements,
            exercise_notes: std::collections::BTreeMap::new(),
        }
    }

    fn set_field_values(reps: u32) -> SetFieldValues {
        SetFieldValues {
            reps: FieldValue::new(domain::Reps::new(reps).unwrap()),
            time: FieldValue::new(domain::Time::default()),
            weight: FieldValue::new(domain::Weight::default()),
            rpe: FieldValue::new(domain::RPE::default()),
        }
    }

    fn weight(value: f32) -> domain::Weight {
        domain::Weight::new(value).unwrap()
    }

    fn filled_weights(elements: &[usize], weights: &[domain::Weight]) -> Vec<(usize, String)> {
        let mut field_values =
            HashMap::from([(0, set_field_values(10)), (1, set_field_values(10))]);

        fill_weights(&mut field_values, elements, weights);

        let mut filled = field_values
            .into_iter()
            .map(|(idx, values)| (idx, values.weight.input))
            .collect::<Vec<_>>();
        filled.sort();
        filled
    }

    #[test]
    fn test_an_entered_weight_is_used_as_start_weight() {
        let mut values = set_field_values(10);
        values.weight = FieldValue::new(weight(80.0));
        let field_values = HashMap::from([(0, values)]);

        assert_eq!(
            entered_weight_of(&field_values, Some(0)),
            Some(weight(80.0))
        );
    }

    #[test]
    fn test_an_unset_or_missing_weight_is_no_start_weight() {
        let field_values = HashMap::from([(0, set_field_values(10))]);

        assert_eq!(entered_weight_of(&field_values, Some(0)), None);
        assert_eq!(entered_weight_of(&field_values, Some(1)), None);
        assert_eq!(entered_weight_of(&field_values, None), None);
    }

    #[test]
    fn test_a_target_weight_is_used_as_start_weight() {
        let training_session = training_session(vec![set_with_target_weight(1, 80.0)]);

        assert_eq!(
            target_weight_of(&training_session, Some(0)),
            Some(weight(80.0))
        );
    }

    #[test]
    fn test_an_unset_target_weight_or_a_rest_is_no_start_weight() {
        let training_session = training_session(vec![set(1, 10), rest()]);

        assert_eq!(target_weight_of(&training_session, Some(0)), None);
        assert_eq!(target_weight_of(&training_session, Some(1)), None);
        assert_eq!(target_weight_of(&training_session, Some(2)), None);
        assert_eq!(target_weight_of(&training_session, None), None);
    }

    #[test]
    fn test_a_surplus_set_keeps_its_weight() {
        assert_eq!(
            filled_weights(&[0, 1], &[weight(50.0)]),
            vec![(0, "50".to_string()), (1, "0".to_string())]
        );
    }

    #[test]
    fn test_a_surplus_weight_is_ignored() {
        assert_eq!(
            filled_weights(&[0], &[weight(50.0), weight(40.0)]),
            vec![(0, "50".to_string()), (1, "0".to_string())]
        );
    }

    #[test]
    fn test_an_element_without_fields_is_skipped() {
        assert_eq!(
            filled_weights(&[2, 1], &[weight(50.0), weight(40.0)]),
            vec![(0, "0".to_string()), (1, "40".to_string())]
        );
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

    #[test]
    fn test_modify_training_session_elements_writes_validated_values() {
        let mut training_session = training_session(vec![set(1, 5), set(1, 5)]);

        modify_training_session_elements(
            &mut training_session,
            &HashMap::from([(1, set_field_values(10))]),
        );

        assert_eq!(training_session.elements, vec![set(1, 5), set(1, 10)]);
    }

    #[test]
    fn test_modify_training_session_elements_keeps_rest_elements() {
        let mut training_session = training_session(vec![rest()]);

        modify_training_session_elements(
            &mut training_session,
            &HashMap::from([(0, set_field_values(10))]),
        );

        assert_eq!(training_session.elements, vec![rest()]);
    }

    #[test]
    fn test_modify_training_session_elements_resets_invalid_values() {
        let mut training_session = training_session(vec![set(1, 5)]);
        let field_values = SetFieldValues {
            reps: FieldValue::default(),
            ..set_field_values(10)
        };

        modify_training_session_elements(
            &mut training_session,
            &HashMap::from([(0, field_values)]),
        );

        assert_eq!(training_session.elements, vec![set(1, 0)]);
    }

    #[test]
    fn test_exercise_name() {
        let exercises = [exercise(1, "A")];

        assert_eq!(exercise_name(1.into(), &exercises), "A");
    }

    #[test]
    fn test_exercise_name_of_unknown_exercise() {
        assert_eq!(exercise_name(1.into(), &[]), "Exercise#1");
    }

    #[test]
    fn test_exercise_number_of_single_exercise() {
        assert_eq!(exercise_number(&1.into(), &[1.into()]), None);
    }

    #[rstest]
    #[case(1, Some(0))]
    #[case(2, Some(1))]
    #[case(3, None)]
    fn test_exercise_number(#[case] exercise_id: u128, #[case] expected: Option<u32>) {
        assert_eq!(
            exercise_number(&exercise_id.into(), &[1.into(), 2.into()]),
            expected
        );
    }

    #[test]
    fn test_unique_keeps_first_occurrence_in_order() {
        assert_eq!(unique(vec![3, 1, 3, 2, 1]), vec![3, 1, 2]);
    }

    #[rstest]
    #[case(0, "\u{2460}")]
    #[case(19, "\u{2473}")]
    #[case(20, "(21)")]
    #[case(99, "(100)")]
    fn test_exercise_marker(#[case] number: u32, #[case] expected: &str) {
        assert_eq!(exercise_marker(number), expected);
    }
}
