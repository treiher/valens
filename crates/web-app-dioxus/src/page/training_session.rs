use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    hash::Hash,
    rc::Rc,
};

use dioxus::{prelude::*, web::WebEventExt};
use futures_util::StreamExt;
use gloo_timers::future::IntervalStream;
use indexmap::IndexMap;

use valens_domain::{self as domain, TrainingSessionService};
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE, DROP_SET_CALCULATOR, ONE_REP_MAX_CALCULATOR, Route,
    audio::{METRONOME_START_DELAY, MetronomeService, TICK_INTERVAL_MS, Timer, TimerService},
    cache::{self, Cache, CacheState},
    dialog::{drop_set::DropSetCalculator, one_rep_max::OneRepMaxCalculatorState},
    eh,
    loading::LoadingFlag,
    muscle::SetsPerMuscle,
    notification::notify,
    ongoing_training_session::OngoingTrainingSession,
    page,
    routing::ExerciseTab,
    settings::Settings,
    ui::{
        element::{
            ActivityBar, Block, CenteredBlock, Color, Dialog, ErrorPage, FloatingActionButton,
            Icon, IconText, Loading, LoadingDialog, LoadingPage, MenuOption, OptionsMenu, PhaseBar,
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

/// The sets of a recent session offered to a set, with the index of the one that corresponds to
/// it.
type RecentSession = (chrono::NaiveDate, Rc<[domain::Set]>, Option<usize>);

/// Seconds a side of a pair waits after the other side before its countdown starts on its own.
const SIDE_SWITCH_SECONDS: u32 = 5;

pub(crate) const COLUMN_HEADER_CLASS: &str =
    "p-1 has-text-centered is-size-7 has-text-weak has-text-weight-normal";

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
    // The groups of elements the drop set calculator fills, `None` while it is closed.
    let mut drop_set_target: Signal<Option<Vec<Vec<usize>>>> = use_signal(|| None);

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
    // The records, their targets and the recent sessions are kept apart from the form, which is
    // rendered anew with every tick of the timer.
    let personal_records = use_memo(move || {
        let (CacheState::Ready(training_sessions), CacheState::Ready(exercises)) =
            (&*cache.training_sessions.read(), &*cache.exercises.read())
        else {
            return domain::PersonalRecords::default();
        };
        domain::PersonalRecords::with_targets(training_sessions, exercises, id)
    });
    let recent_sessions_by_element = use_memo(move || {
        let (CacheState::Ready(training_sessions), Some(training_session)) =
            (&*cache.training_sessions.read(), &*training_session.read())
        else {
            return HashMap::new();
        };
        let recent_sets_by_exercise = DOMAIN_SERVICE().get_recent_session_sets_by_exercise(
            training_session,
            training_sessions,
            RECENT_SESSIONS,
        );
        let set_indices = training_session.set_indices();
        let group_indices = training_session.group_indices();
        let mut offered_sets = HashMap::new();
        let mut result = HashMap::new();
        for (element_idx, element) in training_session.elements.iter().enumerate() {
            let domain::TrainingSessionElement::Set { exercise_id, .. } = element else {
                continue;
            };
            let Some(recent_sets) = recent_sets_by_exercise.get(exercise_id) else {
                continue;
            };
            // The side entered for the set can differ from its stored side.
            for side in [
                domain::Side::Unset,
                domain::Side::Left,
                domain::Side::Right,
                domain::Side::Alternating,
            ] {
                let sessions = offered_sets
                    .entry((*exercise_id, side))
                    .or_insert_with(|| {
                        recent_sets
                            .iter()
                            .map(|(date, sets)| {
                                let offered = domain::OfferedSets::new(sets, side);
                                let shared: Rc<[domain::Set]> = offered.sets().into();
                                (*date, shared, offered)
                            })
                            .collect::<Vec<_>>()
                    })
                    .iter()
                    .map(|(date, sets, offered)| {
                        (
                            *date,
                            sets.clone(),
                            offered.index(group_indices[&element_idx], set_indices[&element_idx]),
                        )
                    })
                    .collect::<Vec<RecentSession>>();
                result.insert((element_idx, side), sessions);
            }
        }
        result
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
        start_delay: use_signal(|| None),
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
            let mut phase_clock = phase_clock;
            if phase_clock.tick_start_delay(
                *progress.element_idx().peek(),
                timer.peek().is_set(),
                chrono::Utc::now(),
            ) {
                timer.write().start();
            }
            timer.peek().sync();
            // Writing on every tick would re-run the effects reading the timer ten times a second.
            if timer.peek().needs_update() {
                timer.write().update();
            }
            if metronome.peek().is_active() {
                metronome.write().update();
            }
            // Only the phase bars move between two seconds, so only they read this.
            let mut elapsed = phase_clock.elapsed;
            let value = phase_clock.seconds(*progress.element_idx().peek(), &timer.peek());
            if (*elapsed.peek() - value).abs() > f64::EPSILON {
                elapsed.set(value);
            }
        }
    });

    let mut field_values = use_signal(HashMap::new);
    let mut carried_inputs = use_signal(|| None::<CarriedInputs>);
    let inputs = SetInputs {
        values: field_values,
        carried: carried_inputs,
    };
    let record_targets = use_memo(move || {
        let Some(training_session) = &*training_session.read() else {
            return HashMap::new();
        };
        record_targets(
            &personal_records.read(),
            training_session,
            &field_values.read(),
        )
    });
    let mut expanded_history: Signal<HashSet<usize>> = use_signal(HashSet::new);
    use_memo(move || {
        if let Some(training_session) = training_session() {
            expanded_history.set(HashSet::new());
            let carried = carried_inputs
                .take()
                .filter(|carried| carried.elements == training_session.elements)
                .map(|carried| carried.values)
                .unwrap_or_default();
            field_values.set(
                training_session
                    .elements
                    .iter()
                    .cloned()
                    .enumerate()
                    .filter_map(|(idx, element)| {
                        if let domain::TrainingSessionElement::Set {
                            side,
                            reps,
                            time,
                            weight,
                            rpe,
                            ..
                        } = element
                        {
                            let stored = SetFieldValues {
                                reps: FieldValue::new_with_empty_default(reps),
                                time: FieldValue::new_with_empty_default(time),
                                weight: FieldValue::new_with_empty_default(weight),
                                rpe: FieldValue::new_with_empty_default(rpe),
                                side: FieldValue::new(side),
                            };
                            Some((
                                idx,
                                match carried.get(&idx) {
                                    Some(previous) => stored.with_unstored_inputs_of(previous),
                                    None => stored,
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
            // The countdown of a single-phase hold gives its cues by itself.
            .and_then(|element| match element {
                domain::TrainingSessionElement::Set {
                    target_reps,
                    target_tempo,
                    countdown,
                    ..
                } if target_reps.non_zero().is_some()
                    || target_tempo.phases().len() > 1
                    || !countdown =>
                {
                    Some((
                        target_tempo.non_zero()?,
                        domain::SetPacing::new(target_reps, target_tempo, countdown)
                            .countdown_seconds(),
                    ))
                }
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
        // Only a set starting automatically waits for its delayed start.
        let set_start_delay = move |delay: Option<StartDelay>| {
            let mut start_delay = phase_clock.start_delay;
            if *start_delay.peek() != delay {
                start_delay.set(delay);
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
                    countdown,
                    ..
                } => {
                    let kind = || {
                        start_delay_kind(
                            progress.peek().completed_preceding,
                            training_session.completes_pair(element_idx),
                        )
                    };
                    let total =
                        match domain::SetPacing::new(*target_reps, *target_tempo, *countdown) {
                            domain::SetPacing::Untimed => {
                                set_start_delay(None);
                                return;
                            }
                            domain::SetPacing::Tempo => {
                                let awaits_start = current_values_are_unperformed()
                                    && !phase_clock.has_bar_clock(element_idx)
                                    && !other_session_running();
                                set_start_delay(automatic.then(|| {
                                    tempo_bar_start_delay(
                                        *phase_clock.start_delay.peek(),
                                        element_idx,
                                        awaits_start,
                                        kind,
                                        chrono::Utc::now(),
                                    )
                                }));
                                return;
                            }
                            domain::SetPacing::Countdown(total) => total,
                        };
                    if !current_values_are_unperformed() {
                        set_start_delay(None);
                        // Filling a set by hand takes it out of its countdown.
                        if progress.timer_service().peek().is_set() {
                            progress.timer_service().write().unset();
                        }
                        return;
                    }
                    if progress.timer_service().read().is_set() {
                        if progress.timer_service().read().seconds() <= 0 {
                            progress.write().complete(element_idx);
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
                                save(training_session, inputs, cache, || {}).await;
                            });
                        }
                    } else {
                        // A set guided by its countdown is not guided by a bar clock.
                        let mut bar = phase_clock.bar;
                        bar.set(None);
                        progress.timer_service().write().set(i64::from(total));
                        set_start_delay(
                            automatic
                                .then(|| StartDelay::new(element_idx, kind(), chrono::Utc::now())),
                        );
                    }
                }
                domain::TrainingSessionElement::Rest {
                    target_time,
                    automatic,
                } => {
                    set_start_delay(None);
                    if let Some(target_time) = target_time.non_zero() {
                        if progress.timer_service().read().is_set() {
                            if *automatic && progress.timer_service().read().seconds() <= 0 {
                                progress.write().complete(element_idx);
                            }
                        } else {
                            progress.timer_service().write().set(i64::from(target_time));
                            progress.timer_service().write().start();
                        }
                    } else if *automatic {
                        progress.write().complete(element_idx);
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
                Some(domain::TrainingSessionElement::Set {
                    exercise_id: current_exercise_id,
                    side,
                    ..
                }) => {
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
                                match side_name(*side) {
                                    Some(side) if id == *current_exercise_id => {
                                        format!("{number}{name} \u{00b7} {side}")
                                    }
                                    _ => format!("{number}{name}"),
                                }
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
            let personal_records = personal_records.read();
            rsx! {
                if edit() {
                    {view_form(inputs, progress, focus, edit_dialog, exercise_dialog, training_session, &recent_sessions_by_element.read(), &personal_records.record_sets(training_session.id), &record_targets.read(), exercises, settings, cache, element_elements, expanded_history, phase_clock)},
                } else {
                    {view_list(edit_dialog, training_session, &personal_records.record_sets(training_session.id), exercises)},
                    {view_muscles(training_session, exercises)}
                }
                Notes { notes, edit },
                {view_edit_dialog(edit_dialog, exercise_dialog, inputs, drop_set_target, progress, phase_clock, training_sessions, cache)}
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
                                save(training_session.clone(), inputs, cache, || {}).await;
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

/// The entered set values by element index, and the inputs held over a save of the session.
#[derive(Clone, Copy)]
struct SetInputs {
    values: Signal<HashMap<usize, SetFieldValues>>,
    carried: Signal<Option<CarriedInputs>>,
}

impl SetInputs {
    /// Holds the entered values for the fields rebuilt from the stored session, moved by `map` if
    /// the save changed the elements.
    ///
    /// The fields keep the inputs only if they are rebuilt from `elements`, so that a session
    /// changed elsewhere does not receive inputs at the wrong elements.
    fn carry_over(
        mut self,
        elements: Vec<domain::TrainingSessionElement>,
        map: Option<&domain::ElementMap>,
    ) {
        let values = self
            .values
            .peek()
            .iter()
            .filter_map(|(element_idx, values)| {
                let element_idx = match map.map(|map| map.get(*element_idx)) {
                    None => *element_idx,
                    Some(Some(domain::ElementMove::Kept(element_idx))) => *element_idx,
                    Some(_) => return None,
                };
                Some((element_idx, values.clone()))
            })
            .collect();
        self.carried.set(Some(CarriedInputs { elements, values }));
    }
}

struct CarriedInputs {
    elements: Vec<domain::TrainingSessionElement>,
    values: HashMap<usize, SetFieldValues>,
}

#[derive(Debug, Clone, PartialEq)]
struct SetFieldValues {
    reps: FieldValue<domain::Reps>,
    time: FieldValue<domain::Time>,
    weight: FieldValue<domain::Weight>,
    rpe: FieldValue<domain::RPE>,
    side: FieldValue<domain::Side>,
}

impl SetFieldValues {
    /// The values of a stored set, keeping each input of `previous` that the set does not hold.
    fn with_unstored_inputs_of(mut self, previous: &SetFieldValues) -> Self {
        keep_unstored_input(&mut self.reps, &previous.reps);
        keep_unstored_input(&mut self.time, &previous.time);
        keep_unstored_input(&mut self.weight, &previous.weight);
        keep_unstored_input(&mut self.rpe, &previous.rpe);
        keep_unstored_input(&mut self.side, &previous.side);
        self
    }

    /// Fills the values a set is recorded with when its countdown runs out or it is confirmed.
    ///
    /// The rating is what the set felt like and is therefore left to the user.
    fn fill(&mut self, reps: domain::Reps, time: domain::Time, weight: domain::Weight) {
        fill_field(&mut self.reps, reps);
        fill_field(&mut self.time, time);
        fill_field(&mut self.weight, weight);
    }

    /// Advances the side through none, left, right and alternating.
    fn cycle_side(&mut self) {
        let side = match self.side.validated.clone().unwrap_or_default() {
            domain::Side::Unset => domain::Side::Left,
            domain::Side::Left => domain::Side::Right,
            domain::Side::Right => domain::Side::Alternating,
            domain::Side::Alternating => domain::Side::Unset,
        };
        self.side.input = side.to_string();
        self.side.validated = Ok(side);
    }

    fn valid(&self) -> bool {
        self.reps.valid()
            && self.time.valid()
            && self.weight.valid()
            && self.rpe.valid()
            && self.side.valid()
    }

    fn changed(&self) -> bool {
        self.values_changed() || self.side.changed()
    }

    /// Whether a value differs from the recorded one, leaving the side aside.
    fn values_changed(&self) -> bool {
        self.reps.changed() || self.time.changed() || self.weight.changed() || self.rpe.changed()
    }

    fn has_valid_changes(&self) -> bool {
        FieldValue::has_valid_changes(&[
            &self.reps,
            &self.time,
            &self.weight,
            &self.rpe,
            &self.side,
        ])
    }

    /// Whether the buttons filling the values of the set are offered.
    fn offers_values(&self) -> bool {
        self.is_empty() || self.values_changed()
    }

    /// Whether the action of the set saves changes rather than activating it.
    fn has_pending_save(&self) -> bool {
        !self.is_unperformed() && self.has_valid_changes()
    }

    fn is_empty(&self) -> bool {
        self.reps.input.is_empty()
            && self.time.input.is_empty()
            && self.weight.input.is_empty()
            && self.rpe.input.is_empty()
    }

    /// Whether the set carries neither recorded nor entered values.
    fn is_unperformed(&self) -> bool {
        self.is_empty() && !self.values_changed()
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

/// Keeps a changed input of `previous` in a field rebuilt from a stored value, unless the input is
/// the stored value.
fn keep_unstored_input<T: Clone + PartialEq>(field: &mut FieldValue<T>, previous: &FieldValue<T>) {
    if previous.changed() && previous.validated != field.validated {
        field.input.clone_from(&previous.input);
        field.validated.clone_from(&previous.validated);
    }
}

/// The clock a set is guided by, and the state its bar and its countdown are read from.
#[derive(Clone, Copy, PartialEq)]
struct PhaseClock {
    /// The clock of a set performed with its input fields, which has no countdown of its own.
    bar: Signal<Option<BarClock>>,
    /// The seconds elapsed on the clock of the current element, written on every tick.
    ///
    /// They lag behind a change of the element until the next tick, so they are only read to
    /// render anew, see `elapsed`.
    elapsed: Signal<f64>,
    /// The wait of the current element before it starts on its own.
    start_delay: Signal<Option<StartDelay>>,
}

impl PhaseClock {
    /// The seconds elapsed on the clock of `element_idx`.
    fn seconds(&self, element_idx: usize, timer: &TimerService) -> f64 {
        match *self.bar.peek() {
            Some(bar) if bar.element_idx == element_idx => bar.seconds(),
            _ => timer.elapsed_exact(),
        }
    }

    /// The seconds elapsed on the clock of `element_idx`, rendered anew on every tick.
    fn elapsed(&self, element_idx: usize, timer: &TimerService) -> f64 {
        self.elapsed.read();
        self.seconds(element_idx, timer)
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

    /// Moves the bar clock and the start delay along with their element, which they do not
    /// outlive.
    fn follow(&mut self, map: &domain::ElementMap) {
        let bar = *self.bar.peek();
        if let Some(bar) = bar {
            self.bar.set(match map.get(bar.element_idx) {
                Some(domain::ElementMove::Kept(element_idx)) => Some(BarClock {
                    element_idx: *element_idx,
                    ..bar
                }),
                _ => None,
            });
        }
        let delay = *self.start_delay.peek();
        if let Some(delay) = delay {
            self.start_delay.set(match map.get(delay.element_idx) {
                Some(domain::ElementMove::Kept(element_idx)) => Some(StartDelay {
                    element_idx: *element_idx,
                    ..delay
                }),
                _ => None,
            });
        }
    }

    /// The pending start delay of `element_idx`, rendered anew when it changes.
    fn pending_start_delay(&self, element_idx: usize) -> Option<PendingStartDelay> {
        self.start_delay
            .read()
            .filter(|delay| delay.element_idx == element_idx)
            .and_then(|delay| delay.pending)
    }

    fn end_start_delay(&mut self, element_idx: usize) {
        let delay = *self.start_delay.peek();
        if let Some(delay) = delay
            && delay.element_idx == element_idx
            && delay.pending.is_some()
        {
            self.start_delay.set(Some(delay.ended()));
        }
    }

    /// Advances the start delay to `now` and returns whether the set timer of the current
    /// `element_idx` is to be started.
    ///
    /// The end of the delay of the current element starts its bar clock unless it has a set timer.
    fn tick_start_delay(
        &mut self,
        element_idx: usize,
        timer_is_set: bool,
        now: chrono::DateTime<chrono::Utc>,
    ) -> bool {
        let Some(delay) = *self.start_delay.peek() else {
            return false;
        };
        let Some(pending) = delay.pending else {
            return false;
        };
        let remaining = pending.remaining_at(now);
        if remaining > 0. {
            self.start_delay.set(Some(StartDelay {
                pending: Some(PendingStartDelay {
                    remaining,
                    ..pending
                }),
                ..delay
            }));
            return false;
        }
        self.start_delay.set(Some(delay.ended()));
        if delay.element_idx != element_idx {
            return false;
        }
        if !timer_is_set && !self.has_bar_clock(element_idx) {
            self.bar.set(Some(BarClock::started(element_idx)));
        }
        timer_is_set
    }

    /// Starts the bar clock of `element_idx` at once if it awaits its start, or holds it where it
    /// stands.
    fn tap_bar(&mut self, element_idx: usize) {
        self.end_start_delay(element_idx);
        self.toggle_bar(element_idx);
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

/// The start delay of an element, which is kept once it has ended, so that it is started only
/// once while its element is current.
#[derive(Clone, Copy, Debug, PartialEq)]
struct StartDelay {
    element_idx: usize,
    /// `None` once the delay has ended.
    pending: Option<PendingStartDelay>,
}

impl StartDelay {
    fn new(element_idx: usize, kind: StartDelayKind, now: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            element_idx,
            pending: Some(PendingStartDelay::new(kind, now)),
        }
    }

    fn ended(self) -> Self {
        Self {
            pending: None,
            ..self
        }
    }
}

/// The wait of a set before it starts on its own, which a tap on the set cuts short.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PendingStartDelay {
    kind: StartDelayKind,
    ends_at: chrono::DateTime<chrono::Utc>,
    /// The seconds left, written on every tick.
    remaining: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum StartDelayKind {
    /// The set starts with the first beep.
    LeadIn,
    /// The second side of a pair leaves the time to change position.
    SideSwitch,
}

impl PendingStartDelay {
    #[allow(clippy::cast_possible_truncation)]
    fn new(kind: StartDelayKind, now: chrono::DateTime<chrono::Utc>) -> Self {
        let seconds = match kind {
            StartDelayKind::LeadIn => METRONOME_START_DELAY,
            StartDelayKind::SideSwitch => f64::from(SIDE_SWITCH_SECONDS),
        };
        Self {
            kind,
            ends_at: now + chrono::Duration::milliseconds((seconds * 1000.) as i64),
            remaining: seconds,
        }
    }

    /// The seconds left at `now`, zero once the delay has ended.
    fn remaining_at(&self, now: chrono::DateTime<chrono::Utc>) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        let remaining = self.ends_at.signed_duration_since(now).num_milliseconds() as f64 / 1000.;
        remaining.max(0.)
    }

    /// The whole seconds left, as shown.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn remaining_seconds(&self) -> u32 {
        self.remaining.ceil() as u32
    }
}

/// The second side of a pair reached by completing the first one leaves the time to change
/// position.
fn start_delay_kind(completed_preceding: bool, completes_pair: bool) -> StartDelayKind {
    if completed_preceding && completes_pair {
        StartDelayKind::SideSwitch
    } else {
        StartDelayKind::LeadIn
    }
}

/// The start delay of the set with a tempo bar at `element_idx`, which is started once while the
/// set is current and ends without starting the bar once the set no longer awaits its start.
fn tempo_bar_start_delay(
    delay: Option<StartDelay>,
    element_idx: usize,
    awaits_start: bool,
    kind: impl FnOnce() -> StartDelayKind,
    now: chrono::DateTime<chrono::Utc>,
) -> StartDelay {
    match delay {
        Some(delay) if delay.element_idx == element_idx => {
            if awaits_start {
                delay
            } else {
                delay.ended()
            }
        }
        _ if awaits_start => StartDelay::new(element_idx, kind(), now),
        _ => StartDelay {
            element_idx,
            pending: None,
        },
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
    inputs: SetInputs,
    mut progress: Store<Progress>,
    focus: SetFocus,
    mut edit_dialog: Signal<EditDialog>,
    exercise_dialog: Signal<page::exercises::ExerciseDialog>,
    training_session: &domain::TrainingSession,
    recent_sessions_by_element: &HashMap<(usize, domain::Side), Vec<RecentSession>>,
    record_sets: &BTreeMap<usize, domain::RecordKind>,
    record_targets: &HashMap<usize, SetRecordTargets>,
    exercises: &[domain::Exercise],
    settings: Settings,
    cache: Cache,
    mut element_elements: Signal<HashMap<usize, web_sys::Element>>,
    expanded_history: Signal<HashSet<usize>>,
    phase_clock: PhaseClock,
) -> Element {
    let mut field_values = inputs.values;
    let sections = training_session.compute_sections();
    let section_element_offsets = first_element_indices(&sections);
    let progress_element_idx = progress.read().element_idx;
    let progress_section_idx = training_session.section_idx(progress_element_idx);
    let progress_section_idx_lookahead =
        training_session.section_idx_lookahead(progress_element_idx);
    let sets_by_exercise = DOMAIN_SERVICE().get_sets_by_exercise(training_session);
    let set_indices = training_session.set_indices();
    // A stretch has no dose without its time.
    let shows_time = settings.show_tut() || training_session.has_time() || {
        let session_exercises = training_session.exercises();
        exercises
            .iter()
            .any(|e| e.is_stretch_only() && session_exercises.contains(&e.id))
    };
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
            let measurements = training_session.measurements_of(*id);
            let measurements_are_empty = measurements.is_empty();
            let exercise_id = *id;
            let is_last_exercise = i == exercise_ids_len - 1;
            let is_last_row_before_note = is_last_exercise && note_is_empty;
            rsx! {
                tr {
                    class: if is_current_section { "" } else { "is-semitransparent" },
                    td {
                        class: "has-text-centered has-text-weight-bold",
                        class: if i == 0 { "pt-2" },
                        class: if is_last_row_before_note && notes_are_empty && measurements_are_empty { "pb-1" },
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
                            to: Route::Exercise { id: *id, tab: ExerciseTab::Exercise },
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
                            class: if is_last_row_before_note && measurements_are_empty { "pb-1" },
                            colspan: 6,
                            div {
                                class: "is-clickable is-italic is-size-7 has-text-weak has-text-centered is-preserving-line-breaks",
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
                if !measurements_are_empty {
                    tr {
                        class: if is_current_section { "" } else { "is-semitransparent" },
                        td {
                            class: "px-2",
                            class: if is_last_row_before_note { "pb-1" },
                            colspan: 6,
                            {view_measurement_row(&measurements, Some(eh!(mut edit_dialog; training_session; {
                                *edit_dialog.write() = EditDialog::SessionExerciseNotes {
                                    training_session,
                                    exercise_id,
                                };
                            })))}
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
                    countdown,
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
                            *countdown,
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
                domain::TrainingSessionElement::Set { exercise_id, side, target_reps, target_tempo, target_weight, target_rpe, countdown, .. } => {
                    let set_index = set_indices[&element_idx];
                    let set_field_values = &field_values.read()[&element_idx];

                    let show_set_buttons = is_current_section && set_field_values.offers_values();
                    let entered_side = set_field_values.side.validated.clone().unwrap_or_default();
                    let set_targets = record_targets.get(&element_idx).copied().filter(|_| is_current_section).unwrap_or_default();
                    let targets = shown_targets(set_targets.entered, set_field_values);
                    // The help line stays while the saved set has targets, so that entering a value does not shift the rows below.
                    let reps_help = match (targets.reps, targets.time) {
                        (Some(reps), _) => Some(view_record_target(reps)),
                        (None, None) if set_targets.saved != domain::RecordTargets::default() => Some(view_record_target_placeholder()),
                        (None, _) => None,
                    };
                    let set_record = saved_record(record_sets, element_idx, set_field_values);

                    let history: &[RecentSession] = if show_set_buttons {
                        recent_sessions_by_element.get(&(element_idx, entered_side)).map_or(&[], Vec::as_slice)
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
                        if let Some(set) = history.first().and_then(|(_, sets, index)| sets.get((*index)?)).cloned() {
                            set_buttons.entry(set).or_default().icons.push("calendar-minus".to_string());
                        }
                    }

                    let number = exercise_number(exercise_id, &exercise_ids);
                    let recorded = (*target_reps, target_tempo.seconds_per_rep(), *target_weight);
                    let side_changed = set_field_values.side.changed();
                    let is_unilateral = cache::laterality(exercises, *exercise_id) == Some(domain::Laterality::Unilateral);
                    // A side can be set where a marker invites it and cleared wherever it is set.
                    let cycles_side = is_unilateral || *side != domain::Side::Unset || entered_side != domain::Side::Unset;
                    let set_marker = rsx! {
                        td {
                            class: "p-1 is-beside-fields",
                            class: if cycles_side { "is-clickable" },
                            "data-testid": "set-number",
                            onclick: move |_| {
                                if cycles_side && let Some(set_field_values) = field_values.write().get_mut(&element_idx) {
                                    set_field_values.cycle_side();
                                }
                            },
                            div {
                                if let Some(number) = number {
                                    "{exercise_marker(number)}"
                                }
                                if entered_side != domain::Side::Unset {
                                    span {
                                        class: if side_changed { "has-text-info" },
                                        "data-testid": "set-side",
                                        "{entered_side}"
                                    }
                                } else if is_unilateral || side_changed {
                                    span {
                                        class: if side_changed { "has-text-info" } else { "has-text-weak" },
                                        "data-testid": "set-side",
                                        "–"
                                    }
                                }
                            }
                        }
                    };

                    match countdown_total(*target_reps, *target_tempo, *countdown, set_field_values, focus) {
                        None => rsx! {
                            tr {
                                class: "has-fields",
                                class: if is_current_section { "" } else { "is-semitransparent" },
                                {set_marker.clone()}
                                td {
                                    class: "p-1 has-text-centered",
                                    InputField {
                                        inputmode: "numeric",
                                        "aria-label": "Reps",
                                        help: reps_help,
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
                                            help: if let Some(time) = targets.time { view_record_target(time) },
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
                                    class: "p-1 is-beside-fields",
                                    div {
                                        if set_field_values.valid() && !(set_field_values.is_unperformed() && focus.is_focused(element_idx)) {
                                            button {
                                                class: "button is-small",
                                                class: if set_field_values.has_pending_save() { "is-link is-outlined" } else if !set_field_values.is_empty() { "is-ghost" },
                                                "data-testid": "set-action",
                                                "aria-label": match (set_field_values.is_unperformed(), set_record) {
                                                    (true, _) => None,
                                                    (false, Some(kind)) => Some(format!("Complete set ({})", record_label(kind).to_lowercase())),
                                                    (false, None) => Some("Complete set".to_string()),
                                                },
                                                disabled: focus.other_session_running() && set_field_values.is_unperformed(),
                                                onclick: eh!(mut training_session; field_values, set_field_values; {
                                                    if set_field_values.is_unperformed() {
                                                        focus.take_ownership();
                                                        progress.write().set_element_idx(element_idx);
                                                    } else {
                                                        progress.write().complete(element_idx);
                                                        modify_training_session_elements(&mut training_session, &field_values.read());
                                                        spawn(async move {
                                                            save(training_session.clone(), inputs, cache, || {}).await;
                                                        });
                                                    }
                                                }),
                                                if set_field_values.is_unperformed() {
                                                    Icon { name: "angles-left" }
                                                } else if let Some(kind) = set_record {
                                                    {view_record_marker(kind)}
                                                } else {
                                                    Icon { name: "check" }
                                                }
                                            }
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
                                            side: *side,
                                            target_tempo: *target_tempo,
                                            progress,
                                            phase_clock,
                                        }
                                    }
                                    td {}
                                }
                            }
                            if is_current_section {
                                {set_value_buttons(set_buttons, history, element_idx, field_values, expanded_history, shows_time, shows_rpe)}
                            }
                        },
                        Some(total) => rsx! {
                            tr {
                                class: if is_current_section { "" } else { "is-semitransparent" },
                                {set_marker.clone()}
                                td {
                                    class: "p-1",
                                    colspan: 4,
                                    div {
                                        class: "notification is-link has-text-centered px-6 pt-1",
                                        // The bottom padding of the focused block is the space above its phase bar
                                        class: if focus.is_focused(element_idx) { "is-size-1 pb-3 has-phase-bar" } else { "pb-1" },
                                        if focus.is_focused(element_idx) {
                                            SetCountdown {
                                                side: *side,
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
                                                progress.write().complete(element_idx);
                                                if let Some(set_field_values) = field_values.write().get_mut(&element_idx) {
                                                    let (reps, time, weight) = recorded;
                                                    set_field_values.fill(reps, time, weight);
                                                }
                                                modify_training_session_elements(&mut training_session, &field_values.read());
                                                spawn(async move {
                                                    save(training_session.clone(), inputs, cache, || {}).await;
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
                                {set_value_buttons(set_buttons, history, element_idx, field_values, expanded_history, shows_time, shows_rpe)}
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
                                                position: Some((0, f64::from(u32::from(target_time)) - phase_clock.elapsed(element_idx, &progress.timer_service().peek()))),
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
                                            progress.write().complete(element_idx);
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

/// The name of a side in a notification, `None` for a set without a side or an alternating set.
fn side_name(side: domain::Side) -> Option<&'static str> {
    match side {
        domain::Side::Unset | domain::Side::Alternating => None,
        domain::Side::Left => Some("Left"),
        domain::Side::Right => Some("Right"),
    }
}

/// The seconds a set is counted down for, or `None` when it is rendered with its input fields.
fn countdown_total(
    target_reps: domain::Reps,
    target_tempo: domain::Tempo,
    countdown: bool,
    set_field_values: &SetFieldValues,
    focus: SetFocus,
) -> Option<u32> {
    if !set_field_values.is_unperformed() || focus.other_session_running() {
        return None;
    }
    domain::SetPacing::new(target_reps, target_tempo, countdown).countdown_seconds()
}

/// The countdown of a set, showing the phase it is in and how far it has got.
#[component]
fn SetCountdown(
    side: domain::Side,
    target_reps: domain::Reps,
    target_tempo: domain::Tempo,
    total: u32,
    progress: Store<Progress>,
    phase_clock: PhaseClock,
) -> Element {
    let timer = progress.timer_service();
    let element_idx = *progress.element_idx().read();
    // A countdown about to start automatically is not waiting for a tap.
    let start_delay = phase_clock.pending_start_delay(element_idx);
    let is_running = timer.read().is_active() || start_delay.is_some();
    let side_switch = start_delay.filter(|delay| delay.kind == StartDelayKind::SideSwitch);
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
            "data-state": if is_running { "running" } else { "waiting" },
            class: if is_running { "" } else { "is-blinking" },
            onclick: move |_| {
                let mut phase_clock = phase_clock;
                phase_clock.end_start_delay(element_idx);
                progress.timer_service().write().start_pause();
            },
            div { "{phase.map_or(remaining, |(_, _, remaining)| remaining.ceil() as i64)} s" }
            if let Some(delay) = side_switch {
                div { class: "is-size-6", "data-testid": "countdown-detail", "{side_name(side).unwrap_or_default()} side in {delay.remaining_seconds()} s" }
            } else if let Some(reps) = target_reps.non_zero() {
                div { class: "is-size-6", "data-testid": "countdown-detail", "{repetition}/{reps}" }
            }
        }
        if let Some(delay) = side_switch {
            PhaseBar {
                class: "phase-bar-pinned",
                phases: vec![SIDE_SWITCH_SECONDS],
                position: Some((0, delay.remaining)),
            }
        } else {
            PhaseBar {
                class: "phase-bar-pinned",
                phases: target_tempo.phases().iter().copied().map(u32::from).collect::<Vec<_>>(),
                position: target_tempo
                    .phase_at(phase_clock.elapsed(element_idx, &timer.peek()).max(0.))
                    .map(|(_, index, remaining)| (index, remaining)),
            }
        }
    }
}

/// The tempo bar of a set performed with its input fields, which the user starts and holds by
/// tapping, and which a set starting automatically starts after its start delay.
#[component]
fn SetTempoBar(
    element_idx: usize,
    side: domain::Side,
    target_tempo: domain::Tempo,
    progress: Store<Progress>,
    phase_clock: PhaseClock,
) -> Element {
    let side_switch = phase_clock
        .pending_start_delay(element_idx)
        .filter(|delay| delay.kind == StartDelayKind::SideSwitch);
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
            "data-state": match (has_started, is_running) {
                (_, true) => "running",
                (true, false) => "paused",
                (false, false) => "stopped",
            },
            // Vertical padding, so the thin bar can be hit with a finger
            class: "py-2",
            class: if has_started && !is_running { "is-blinking" },
            onclick: move |_| {
                let mut phase_clock = phase_clock;
                phase_clock.tap_bar(element_idx);
            },
            if let Some(delay) = side_switch {
                div { class: "is-size-7 has-text-centered", "data-testid": "tempo-bar-detail", "{side_name(side).unwrap_or_default()} side in {delay.remaining_seconds()} s" }
                PhaseBar {
                    phases: vec![SIDE_SWITCH_SECONDS],
                    position: Some((0, delay.remaining)),
                }
            } else {
                PhaseBar {
                    phases: target_tempo.phases().iter().copied().map(u32::from).collect::<Vec<_>>(),
                    position: has_started
                        .then(|| target_tempo.phase_at(phase_clock.elapsed(element_idx, &progress.timer_service().peek()).max(0.)))
                        .flatten()
                        .map(|(_, index, remaining)| (index, remaining)),
                }
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
    history: &[RecentSession],
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
                    for (session_index, (date, sets, set_index)) in history.iter().enumerate() {
                        div {
                            class: if session_index < last_session_index { "mb-2" },
                            "data-testid": "set-history-session",
                            div { class: "is-size-7", "{date}" }
                            div {
                                class: "is-flex is-flex-wrap-wrap is-justify-content-center is-flex-gap-row-gap-1",
                                for (index, set) in sets.iter().cloned().enumerate() {
                                    {set_value_button(&set, None, None, set_index.is_some_and(|set_index| index != set_index), element_idx, field_values, shows_time, shows_rpe)}
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
    edit_dialog: Signal<EditDialog>,
    training_session: &domain::TrainingSession,
    record_sets: &BTreeMap<usize, domain::RecordKind>,
    exercises: &[domain::Exercise],
) -> Element {
    let sections = training_session.compute_sections();
    let section_element_offsets = first_element_indices(&sections);
    let record_marker = |element_idx: usize| {
        record_sets
            .get(&element_idx)
            .map(|kind| view_record_marker(*kind))
    };
    let rows = sections.iter().zip(section_element_offsets).map(|(section, first_element_idx)| {
        let exercise_ids = unique(section.exercise_ids());
        let exercise_ids_len = exercise_ids.len();
        let exercise_names = exercise_ids.clone().into_iter().enumerate().map(|(i, id)| {
            let name = exercise_name(id, exercises);
            let number = exercise_number(&id, &exercise_ids);
            let note = training_session.exercise_notes.get(&id).cloned().unwrap_or_default();
            let note_is_empty = note.is_empty();
            let measurements = training_session.measurements_of(id);
            let measurements_are_empty = measurements.is_empty();
            let is_last_row_before_note = i == exercise_ids_len - 1 && note_is_empty;
            rsx! {
                tr {
                    td {
                        class: "has-text-centered has-text-weight-bold",
                        class: if i == 0 { "pt-2" },
                        class: if is_last_row_before_note && measurements_are_empty { "pb-1" },
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
                            to: Route::Exercise { id, tab: ExerciseTab::Exercise },
                            "{name}"
                        }
                    }
                }
                if !measurements_are_empty {
                    tr {
                        td {
                            class: "px-2",
                            class: if is_last_row_before_note { "pb-1" },
                            colspan: 6,
                            {view_measurement_row(&measurements, Some(eh!(mut edit_dialog; training_session; {
                                *edit_dialog.write() = EditDialog::SessionExerciseNotes {
                                    training_session,
                                    exercise_id: id,
                                };
                            })))}
                        }
                    }
                }
                if !note_is_empty {
                    tr {
                        td {
                            class: "px-2",
                            class: if i == exercise_ids_len - 1 { "pb-1" },
                            colspan: 6,
                            div {
                                class: "is-clickable is-italic has-text-centered",
                                "data-testid": "session-exercise-notes",
                                onclick: eh!(mut edit_dialog; training_session; {
                                    *edit_dialog.write() = EditDialog::SessionExerciseNotes {
                                        training_session,
                                        exercise_id: id,
                                    };
                                }),
                                { note }
                            }
                        }
                    }
                }
            }
        });

        let groups = section.groups();
        let first_sides_group = groups
            .iter()
            .position(|group| matches!(group, domain::ElementGroup::Sides { .. }));
        let sets = groups.into_iter().enumerate().map(|(group_idx, group)| {
            let (element_idx, element) = match group {
                domain::ElementGroup::Sides { left, right } => {
                    let number = left
                        .or(right)
                        .and_then(|(_, element)| match element {
                            domain::TrainingSessionElement::Set { exercise_id, .. } => {
                                exercise_number(exercise_id, &exercise_ids)
                            }
                            domain::TrainingSessionElement::Rest { .. } => None,
                        });
                    return rsx! {
                        if first_sides_group == Some(group_idx) {
                            tr {
                                "data-testid": "side-header",
                                td {}
                                th { class: COLUMN_HEADER_CLASS, colspan: 2, scope: "col", "Left" }
                                th { class: COLUMN_HEADER_CLASS, colspan: 2, scope: "col", "Right" }
                                td {}
                            }
                        }
                        tr {
                            "data-testid": "set-sides-row",
                            td {
                                class: "px-2 has-text-centered",
                                if let Some(number) = number {
                                    "{exercise_marker(number)}"
                                }
                            }
                            for element in [left, right] {
                                td {
                                    class: "px-2 has-text-centered",
                                    colspan: 2,
                                    "data-testid": "set-side-values",
                                    if let Some((element_idx, element)) = element {
                                        span {
                                            class: "icon-text is-flex-wrap-nowrap is-align-items-center",
                                            span { {side_cell(element)} }
                                            {record_marker(first_element_idx + element_idx)}
                                        }
                                    }
                                }
                            }
                            td {}
                        }
                    };
                }
                domain::ElementGroup::Single(element_idx, element) => (element_idx, element),
            };
            rsx! {
                match element {
                    domain::TrainingSessionElement::Set { exercise_id, side, reps, time, weight, rpe, .. } => {
                        let number = exercise_number(exercise_id, &exercise_ids);
                        let alternating = *side == domain::Side::Alternating;
                        rsx! {
                            tr {
                                if *reps == domain::Reps::default() && *time == domain::Time::default() && *weight == domain::Weight::default() && *rpe == domain::RPE::ZERO {
                                    td {
                                        class: "px-2 has-text-centered",
                                        colspan: 6,
                                        if number.is_some() || alternating {
                                            span {
                                                class: "pr-2",
                                                if let Some(number) = number {
                                                    "{exercise_marker(number)}"
                                                }
                                                if alternating {
                                                    span { "data-testid": "set-side", "{side}" }
                                                }
                                                " "
                                            }
                                        }
                                        span {
                                            class: if number.is_some() || alternating { "pr-5" },
                                            "–"
                                        }
                                    }
                                } else {
                                    td {
                                        class: "px-2 has-text-centered",
                                        if let Some(number) = number {
                                            "{exercise_marker(number)}"
                                        }
                                        if alternating {
                                            span { "data-testid": "set-side", "{side}" }
                                        }
                                    }
                                    td {
                                        class: "px-2 has-text-right",
                                        if *reps > domain::Reps::default() {
                                            if *weight > domain::Weight::default()
                                                || *time > domain::Time::default()
                                            {
                                                "{reps} ×"
                                            } else {
                                                "{reps}"
                                            }
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
                                    td {
                                        class: "px-1",
                                        {record_marker(first_element_idx + element_idx)}
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

/// The session index of the first element of each section.
fn first_element_indices(sections: &[domain::TrainingSessionSection]) -> Vec<usize> {
    sections
        .iter()
        .scan(0, |offset, section| {
            let first_element_idx = *offset;
            *offset += section.elements().len();
            Some(first_element_idx)
        })
        .collect()
}

/// The values of a set with a side, stating the time and the rating whenever they are set.
fn side_cell(element: &domain::TrainingSessionElement) -> String {
    let values = element.to_string(true, true);
    if values.is_empty() {
        "–".to_string()
    } else {
        values
    }
}

fn view_muscles(
    training_session: &domain::TrainingSession,
    exercises: &[domain::Exercise],
) -> Element {
    let stretch_per_muscle =
        if training_session.stretch_sets(&domain::StretchOnlyExercises::new(exercises)) > 0. {
            training_session.stretch_per_muscle(exercises)
        } else {
            BTreeMap::new()
        };
    rsx! {
        SetsPerMuscle {
            stimulus_per_muscle: training_session.stimulus_per_muscle(exercises),
            stretch_per_muscle,
            worked_caption: "Worked (hard sets)",
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn view_edit_dialog(
    mut edit_dialog: Signal<EditDialog>,
    exercise_dialog: Signal<page::exercises::ExerciseDialog>,
    inputs: SetInputs,
    drop_set_target: Signal<Option<Vec<Vec<usize>>>>,
    progress: Store<Progress>,
    phase_clock: PhaseClock,
    training_sessions: &[domain::TrainingSession],
    cache: Cache,
) -> Element {
    let field_values = inputs.values;
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
                                    text: "Edit measurement and session notes".to_string(),
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
                                                    let start_weight = drop_set_start_weight(&field_values.read(), recent_best_set, &training_session, &elements);
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
                                    on_click: eh!(training_session, element_idx, close_dialog; {
                                        let mut training_session = latest(&training_session, cache);
                                        modify_training_session_elements(&mut training_session, &field_values.read());
                                        let map = training_session.add_set(element_idx);
                                        save_moved(training_session, map, progress, phase_clock, inputs, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "plus".to_string(),
                                    text: "Add same exercise".to_string(),
                                    "data-testid": "options-add-same-exercise",
                                    on_click: eh!(training_session, section_idx, exercise_id, close_dialog; {
                                        let mut training_session = latest(&training_session, cache);
                                        modify_training_session_elements(&mut training_session, &field_values.read());
                                        let map = training_session.add_exercise(section_idx, exercise_id, cache.laterality(exercise_id));
                                        save_moved(training_session, map, progress, phase_clock, inputs, cache, close_dialog)
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
                                    on_click: eh!(training_session, section_idx, close_dialog; {
                                        let mut training_session = latest(&training_session, cache);
                                        modify_training_session_elements(&mut training_session, &field_values.read());
                                        let map = training_session.move_section_up(section_idx);
                                        save_moved(training_session, map, progress, phase_clock, inputs, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "arrow-down".to_string(),
                                    text: "Move down".to_string(),
                                    on_click: eh!(training_session, section_idx, close_dialog; {
                                        let mut training_session = latest(&training_session, cache);
                                        modify_training_session_elements(&mut training_session, &field_values.read());
                                        let map = training_session.move_section_down(section_idx);
                                        save_moved(training_session, map, progress, phase_clock, inputs, cache, close_dialog)
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
                                    on_click: eh!(training_session, section_idx, close_dialog; {
                                        let mut training_session = latest(&training_session, cache);
                                        modify_training_session_elements(&mut training_session, &field_values.read());
                                        let map = training_session.remove_set(section_idx);
                                        save_moved(training_session, map, progress, phase_clock, inputs, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "times".to_string(),
                                    text: "Remove exercise".to_string(),
                                    "data-testid": "options-remove-exercise",
                                    on_click: eh!(training_session, section_idx, exercise_id, close_dialog; {
                                        let mut training_session = latest(&training_session, cache);
                                        modify_training_session_elements(&mut training_session, &field_values.read());
                                        let map = training_session.remove_exercise(section_idx, exercise_id);
                                        save_moved(training_session, map, progress, phase_clock, inputs, cache, close_dialog)
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
                                    let mut training_session = latest(&training_session, cache);
                                    modify_training_session_elements(&mut training_session, &field_values.read());
                                    let map = training_session.add_exercise(section_idx, exercise_id, cache.laterality(exercise_id));
                                    save_moved(training_session, map, progress, phase_clock, inputs, cache, close_dialog)
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
                                    let mut training_session = latest(&training_session, cache);
                                    modify_training_session_elements(&mut training_session, &field_values.read());
                                    let map = training_session.replace_exercise(section_idx, exercise_id, replacement, cache.laterality(replacement));
                                    save_moved(training_session, map, progress, phase_clock, inputs, cache, close_dialog)
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
                                    let mut training_session = latest(&training_session, cache);
                                    modify_training_session_elements(&mut training_session, &field_values.read());
                                    let map = training_session.append_exercise(exercise_id, cache.laterality(exercise_id));
                                    save_moved(training_session, map, progress, phase_clock, inputs, cache, close_dialog)
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
                    on_save: {
                        let exercise_id = *exercise_id;
                        move |edited: domain::TrainingSession| {
                            let mut training_session = latest(&edited, cache);
                            training_session.copy_exercise_entries_from(&edited, exercise_id);
                            save(training_session, inputs, cache, close_dialog)
                        }
                    },
                    on_close: eh!(mut close_dialog; { close_dialog(); }),
                }
            }
        }
    }
}

/// Returns the weight the drop sets of `groups` start from, read from the first element of the
/// first group, as both sets of a pair carry the same weight.
fn drop_set_start_weight(
    field_values: &HashMap<usize, SetFieldValues>,
    recent_best_set: Option<(domain::Reps, domain::Weight)>,
    training_session: &domain::TrainingSession,
    groups: &[Vec<usize>],
) -> Option<domain::Weight> {
    let first_element_idx = groups.first().and_then(|group| group.first()).copied();
    entered_weight_of(field_values, first_element_idx)
        .or_else(|| recent_best_set.map(|(_, weight)| weight))
        .or_else(|| target_weight_of(training_session, first_element_idx))
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

/// Writes `weights` into the weight fields of the groups of elements, pairing them in order and
/// giving every element of a group the weight of the group.
///
/// Surplus weights and surplus groups are left alone.
fn fill_weights(
    field_values: &mut HashMap<usize, SetFieldValues>,
    groups: &[Vec<usize>],
    weights: &[domain::Weight],
) {
    for (group, weight) in groups.iter().zip(weights) {
        for element_idx in group {
            if let Some(set_field_values) = field_values.get_mut(element_idx) {
                set_field_values.weight.input = weight.to_string();
                set_field_values.weight.validated = Ok(*weight);
            }
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
    let stored_measurements = training_session.measurements_of(exercise_id);
    let mut measurement_fields = use_signal(|| {
        training_session
            .measurement_sides(exercise_id)
            .into_iter()
            .map(|side| {
                (
                    side,
                    FieldValue::from_option(stored_measurements.get(&side).copied()),
                )
            })
            .collect::<BTreeMap<_, _>>()
    });
    let mut measurement_elements =
        use_signal(BTreeMap::<domain::Side, web_sys::HtmlInputElement>::new);
    let changed = note_input.read().trim() != note.trim()
        || measurement_fields
            .read()
            .values()
            .any(FieldValueState::changed);
    let valid = measurement_fields
        .read()
        .values()
        .all(FieldValueState::valid);
    let sides = measurement_fields
        .read()
        .keys()
        .copied()
        .collect::<Vec<_>>();
    let routine_name = |routines: &[domain::Routine], routine_id: domain::RoutineID| {
        routines
            .iter()
            .find(|routine| routine.id == routine_id)
            .map_or_else(|| "-".to_string(), |routine| routine.name.to_string())
    };
    let (previous_notes, previous_measurements) =
        match (&*cache.training_sessions.read(), &*cache.routines.read()) {
            (CacheState::Ready(training_sessions), CacheState::Ready(routines)) => (
                training_session
                    .previous_exercise_notes(exercise_id, training_sessions)
                    .into_iter()
                    .map(|previous_note| {
                        (
                            previous_note.date,
                            routine_name(routines, previous_note.routine_id),
                            previous_note.note,
                        )
                    })
                    .collect::<Vec<_>>(),
                training_session
                    .previous_exercise_measurements(exercise_id, training_sessions)
                    .into_iter()
                    .map(|previous| {
                        (
                            previous.date,
                            routine_name(routines, previous.routine_id),
                            previous.values,
                        )
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => (vec![], vec![]),
        };
    let fields = measurement_fields.read().clone();
    rsx! {
        SaveDialog {
            title: rsx! { "Measurement and session notes" },
            on_close,
            on_save: eh!(mut training_session; exercise_id; {
                let note = note_input.read().trim().to_string();
                if note.is_empty() {
                    training_session.exercise_notes.remove(&exercise_id);
                } else {
                    training_session.exercise_notes.insert(exercise_id, note);
                }
                for (side, field) in measurement_fields.read().iter() {
                    match field.validated {
                        Ok(Some(measurement)) => {
                            training_session
                                .exercise_measurements
                                .insert((exercise_id, *side), measurement);
                        }
                        Ok(None) | Err(_) => {
                            training_session
                                .exercise_measurements
                                .remove(&(exercise_id, *side));
                        }
                    }
                }
                on_save.call(training_session);
            }),
            is_loading: IS_LOADING(),
            disabled: IS_LOADING() || !changed || !valid,
            for (side, field) in fields {
                InputField {
                    label: measurement_label(side),
                    r#type: "number",
                    step: "0.01",
                    value: field.input.clone(),
                    error: if let Err(err) = &field.validated { err.clone() },
                    has_changed: field.changed(),
                    "data-testid": "session-exercise-measurement-input-{measurement_label(side).to_lowercase()}",
                    on_input: move |event: FormEvent| {
                        let bad_input = measurement_elements
                            .read()
                            .get(&side)
                            .is_some_and(|input| input.validity().bad_input());
                        if let Some(field) = measurement_fields.write().get_mut(&side) {
                            update_measurement_field(field, &event.value(), bad_input);
                        }
                    },
                    on_mounted: move |input: web_sys::HtmlInputElement| {
                        measurement_elements.write().insert(side, input);
                    },
                }
            }
            TextAreaField {
                label: "Notes",
                value: note.clone(),
                has_changed: note_input.read().trim() != note.trim(),
                autofocus: true,
                "data-testid": "session-exercise-notes-input",
                on_input: move |event: FormEvent| {
                    *note_input.write() = event.value();
                },
                on_mounted: move |textarea: web_sys::HtmlTextAreaElement| {
                    textarea_element.set(Some(textarea));
                },
            }
            for (date, routine_name, values) in previous_measurements {
                div {
                    div {
                        class: "block has-text-centered has-text-weight-bold mb-1",
                        "{date} {routine_name}"
                    }
                    div {
                        class: "is-relative has-text-centered mb-2",
                        "data-testid": "previous-session-exercise-measurement",
                        {measurement_values_to_string(&values)}
                        if values.keys().copied().eq(sides.iter().copied()) {
                            button {
                                class: "button is-overlay-top-right p-0 mr-2",
                                r#type: "button",
                                "data-testid": "session-exercise-measurement-reuse",
                                onclick: move |_| {
                                    reuse_measurements(&mut measurement_fields.write(), &values);
                                    for (side, value) in &values {
                                        if let Some(input) = measurement_elements.read().get(side) {
                                            input.set_value(&value.to_string());
                                        }
                                    }
                                },
                                Icon { name: "reply".to_string() }
                            }
                        }
                    }
                }
            }
            if !previous_notes.is_empty() {
                for (date, routine_name, note) in previous_notes {
                    div {
                        div {
                            class: "block has-text-centered has-text-weight-bold mb-1",
                            "{date} {routine_name}"
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

fn measurement_label(side: domain::Side) -> &'static str {
    match side {
        domain::Side::Unset | domain::Side::Alternating => "Measurement",
        domain::Side::Left => "Left",
        domain::Side::Right => "Right",
    }
}

/// Update a measurement field by an input.
///
/// The browser reads content of a number field it cannot parse as an empty value, so such an
/// input is marked as invalid while the field keeps its previous input, rather than being read as
/// a removed value.
fn update_measurement_field(
    field: &mut FieldValue<Option<domain::Measurement>>,
    value: &str,
    bad_input: bool,
) {
    if bad_input {
        field.validated = Err(domain::MeasurementError::ParseError.to_string());
        return;
    }
    value.clone_into(&mut field.input);
    field.validated = if value.trim().is_empty() {
        Ok(None)
    } else {
        domain::Measurement::try_from(value)
            .map(Some)
            .map_err(|err| err.to_string())
    };
}

fn reuse_measurements(
    fields: &mut BTreeMap<domain::Side, FieldValue<Option<domain::Measurement>>>,
    values: &BTreeMap<domain::Side, domain::Measurement>,
) {
    for (side, value) in values {
        if let Some(field) = fields.get_mut(side) {
            field.input = value.to_string();
            field.validated = Ok(Some(*value));
        }
    }
}

/// The kind of record set of a set whose changes are saved, so that the marker follows the saved
/// values.
fn saved_record(
    record_sets: &BTreeMap<usize, domain::RecordKind>,
    element_idx: usize,
    values: &SetFieldValues,
) -> Option<domain::RecordKind> {
    record_sets
        .get(&element_idx)
        .copied()
        .filter(|_| !values.has_pending_save())
}

/// Show the trophy marking a record set of `kind`.
pub fn view_record_marker(kind: domain::RecordKind) -> Element {
    let test_id = match kind {
        domain::RecordKind::AllTime => "personal-record",
        domain::RecordKind::Recent => "recent-personal-record",
    };
    rsx! {
        Icon {
            name: "trophy",
            is_small: true,
            class: record_class(kind),
            role: "img",
            "aria-label": record_label(kind),
            "data-testid": test_id,
        }
    }
}

fn record_label(kind: domain::RecordKind) -> &'static str {
    match kind {
        domain::RecordKind::AllTime => "Personal record",
        domain::RecordKind::Recent => "Recent personal record",
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct SetRecordTargets {
    /// The targets following the entered values of all sets.
    entered: domain::RecordTargets,
    /// The targets following the saved values of all sets.
    saved: domain::RecordTargets,
}

/// The targets of the sets per element index.
fn record_targets(
    personal_records: &domain::PersonalRecords,
    training_session: &domain::TrainingSession,
    field_values: &HashMap<usize, SetFieldValues>,
) -> HashMap<usize, SetRecordTargets> {
    let mut entered = training_session.clone();
    modify_training_session_elements(&mut entered, field_values);
    field_values
        .iter()
        .map(|(element_idx, values)| {
            let targets = SetRecordTargets {
                entered: field_targets(
                    personal_records,
                    training_session,
                    &entered,
                    *element_idx,
                    values,
                ),
                saved: personal_records.targets(training_session, *element_idx),
            };
            (*element_idx, targets)
        })
        .collect()
}

/// The targets of a set with valid weight, RPE and side.
///
/// Whether the set is recorded follows its saved values, so that entering a value of the set
/// keeps its targets.
fn field_targets(
    personal_records: &domain::PersonalRecords,
    training_session: &domain::TrainingSession,
    entered: &domain::TrainingSession,
    element_idx: usize,
    values: &SetFieldValues,
) -> domain::RecordTargets {
    let (Ok(_), Ok(_), Ok(_)) = (
        &values.weight.validated,
        &values.rpe.validated,
        &values.side.validated,
    ) else {
        return domain::RecordTargets::default();
    };
    let mut session = entered.clone();
    if let (
        Some(domain::TrainingSessionElement::Set { reps, time, .. }),
        Some(domain::TrainingSessionElement::Set {
            reps: saved_reps,
            time: saved_time,
            ..
        }),
    ) = (
        session.elements.get_mut(element_idx),
        training_session.elements.get(element_idx),
    ) {
        *reps = *saved_reps;
        *time = *saved_time;
    }
    personal_records.targets(&session, element_idx)
}

/// The targets shown below the empty reps and time fields of a set.
fn shown_targets(targets: domain::RecordTargets, values: &SetFieldValues) -> domain::RecordTargets {
    let reps_is_empty = values.reps.input.is_empty();
    domain::RecordTargets {
        reps: targets.reps.filter(|_| reps_is_empty),
        time: targets
            .time
            .filter(|_| reps_is_empty && values.time.input.is_empty()),
    }
}

/// Show the value a set needs for a recent record.
fn view_record_target(value: impl std::fmt::Display) -> Element {
    rsx! {
        span {
            class: "icon-text is-flex-wrap-nowrap is-align-items-center has-text-weak",
            Icon {
                name: "trophy",
                is_small: true,
                class: record_class(domain::RecordKind::Recent),
            }
            span { class: "is-sr-only", "Needed for a recent personal record: " }
            span { "data-testid": "record-target", "{value}" }
        }
    }
}

/// Keep the height of a record target.
fn view_record_target_placeholder() -> Element {
    rsx! {
        span {
            class: "icon-text is-align-items-center is-invisible",
            "aria-hidden": "true",
            Icon { name: "trophy", is_small: true }
            span { "\u{a0}" }
        }
    }
}

pub fn record_class(kind: domain::RecordKind) -> &'static str {
    match kind {
        domain::RecordKind::AllTime => "has-text-gold",
        domain::RecordKind::Recent => "has-text-weak",
    }
}

/// Show the measurement of an exercise, opening the dialog of `on_click` when clicked.
pub fn view_measurement_row(
    values: &BTreeMap<domain::Side, domain::Measurement>,
    mut on_click: Option<impl FnMut(MouseEvent) + 'static>,
) -> Element {
    let text = measurement_values_to_string(values);
    rsx! {
        div {
            class: "has-text-centered",
            class: if on_click.is_some() { "is-clickable" },
            "data-testid": "session-exercise-measurement",
            "aria-label": "Measurement",
            onclick: move |event| {
                if let Some(on_click) = &mut on_click {
                    on_click(event);
                }
            },
            IconText { icon: "ruler-horizontal", "{text}" }
        }
    }
}

/// The values of a measurement in the order of the sides, as in `4.5 · L 12.5 · R 10`.
pub fn measurement_values_to_string(
    values: &BTreeMap<domain::Side, domain::Measurement>,
) -> String {
    values
        .iter()
        .map(|(side, value)| match side {
            domain::Side::Unset | domain::Side::Alternating => value.to_string(),
            domain::Side::Left | domain::Side::Right => format!("{side} {value}"),
        })
        .collect::<Vec<_>>()
        .join(" · ")
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

/// The training session as currently cached, falling back to `training_session`, so that a change
/// made in a dialog keeps what was saved while the dialog was open.
fn latest(training_session: &domain::TrainingSession, cache: Cache) -> domain::TrainingSession {
    match &*cache.training_sessions.peek() {
        CacheState::Ready(training_sessions) => training_sessions
            .iter()
            .find(|t| t.id == training_session.id)
            .cloned(),
        _ => None,
    }
    .unwrap_or_else(|| training_session.clone())
}

/// Writes the entered set values into the training session, leaving the stored value of an
/// invalid input in place.
fn modify_training_session_elements(
    training_session: &mut domain::TrainingSession,
    field_values: &HashMap<usize, SetFieldValues>,
) {
    for (element_idx, element) in &mut training_session.elements.iter_mut().enumerate() {
        if let domain::TrainingSessionElement::Set {
            side,
            reps,
            time,
            weight,
            rpe,
            ..
        } = element
            && let Some(set_field_values) = field_values.get(&element_idx)
        {
            store_valid_input(side, &set_field_values.side);
            store_valid_input(reps, &set_field_values.reps);
            store_valid_input(time, &set_field_values.time);
            store_valid_input(weight, &set_field_values.weight);
            store_valid_input(rpe, &set_field_values.rpe);
        }
    }
}

fn store_valid_input<T: Clone>(value: &mut T, field: &FieldValue<T>) {
    if let Ok(input) = &field.validated {
        value.clone_from(input);
    }
}

async fn save(
    training_session: domain::TrainingSession,
    inputs: SetInputs,
    cache: Cache,
    mut close_dialog: impl FnMut(),
) {
    let loading = LoadingFlag::set(&IS_LOADING);
    let elements = training_session.elements.clone();
    let stored = store(training_session).await;
    drop(loading);
    if stored {
        inputs.carry_over(elements, None);
        cache.refresh_training_sessions();
    }
    close_dialog();
}

/// Saves a training session whose elements were moved as given by `map`, and moves the current
/// element along.
async fn save_moved(
    training_session: domain::TrainingSession,
    map: domain::ElementMap,
    mut progress: Store<Progress>,
    mut phase_clock: PhaseClock,
    inputs: SetInputs,
    cache: Cache,
    mut close_dialog: impl FnMut(),
) {
    let loading = LoadingFlag::set(&IS_LOADING);
    let elements = training_session.elements.clone();
    if store(training_session).await {
        inputs.carry_over(elements, Some(&map));
        cache.load_training_sessions().await;
        // Moved before the guide reacts to the reloaded session, as a position past its end would
        // end the ongoing training session.
        progress.write().follow(&map);
        phase_clock.follow(&map);
    }
    drop(loading);
    close_dialog();
}

/// Returns whether the training session was stored.
async fn store(training_session: domain::TrainingSession) -> bool {
    match DOMAIN_SERVICE()
        .modify_training_session(
            training_session.id,
            Some(training_session.notes),
            Some(training_session.elements),
            Some(training_session.exercise_notes),
            Some(training_session.exercise_measurements),
        )
        .await
    {
        Ok(_) => true,
        Err(err) => {
            notify("modify training session", &err);
            false
        }
    }
}

#[derive(Store, Clone)]
struct Progress {
    training_session_id: domain::TrainingSessionID,
    start_time: chrono::DateTime<chrono::Utc>,
    element_idx: usize,
    element_start_time: chrono::DateTime<chrono::Utc>,
    timer_service: TimerService,
    /// Whether the current element was reached by completing the preceding one while it was
    /// current.
    completed_preceding: bool,
}

impl Progress {
    fn new(training_session_id: domain::TrainingSessionID) -> Self {
        Self {
            training_session_id,
            start_time: chrono::Utc::now(),
            element_idx: usize::MAX,
            element_start_time: chrono::Utc::now(),
            timer_service: TimerService::default(),
            completed_preceding: false,
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
        self.completed_preceding = false;
    }

    /// Moves on to the element following the completed `element_idx`.
    fn complete(&mut self, element_idx: usize) {
        let was_current = element_idx == self.element_idx;
        self.set_element_idx(element_idx + 1);
        self.completed_preceding = was_current;
    }

    fn reset(&mut self) {
        self.start_time = chrono::Utc::now();
    }

    /// Follows the current element to where `map` moved it, or starts anew on the element that
    /// followed it if it was removed.
    fn follow(&mut self, map: &domain::ElementMap) {
        match map.get(self.element_idx) {
            Some(domain::ElementMove::Kept(element_idx)) => self.element_idx = *element_idx,
            Some(domain::ElementMove::Removed(element_idx)) => self.set_element_idx(*element_idx),
            None => {}
        }
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
            completed_preceding: false,
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
            provide_ongoing_training_session, provide_settings, render, rows_containing, rows_of,
            text_of,
        },
    };

    fn recorded_set(exercise_id: u128) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            side: domain::Side::Unset,
            reps: domain::Reps::new(10).unwrap(),
            time: domain::Time::new(30).unwrap(),
            weight: domain::Weight::new(50.0).unwrap(),
            rpe: domain::RPE::new(8.0).unwrap(),
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::ZERO,
            automatic: false,
            countdown: false,
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
                exercise_measurements: std::collections::BTreeMap::new(),
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
                    side: domain::Side::Unset,
                    reps: domain::Reps::default(),
                    time: domain::Time::default(),
                    weight: domain::Weight::default(),
                    rpe: domain::RPE::ZERO,
                    target_reps: domain::Reps::new(10).unwrap(),
                    target_tempo: domain::Tempo::default(),
                    target_weight: domain::Weight::default(),
                    target_rpe: domain::RPE::ZERO,
                    automatic: false,
                    countdown: false,
                }],
                exercise_notes: std::collections::BTreeMap::new(),
                exercise_measurements: std::collections::BTreeMap::new(),
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
                    side: domain::Side::Unset,
                    reps: domain::Reps::default(),
                    time: domain::Time::default(),
                    weight: domain::Weight::default(),
                    rpe: domain::RPE::ZERO,
                    target_reps: domain::Reps::default(),
                    target_tempo: domain::Tempo::new(&[60]).unwrap(),
                    target_weight: domain::Weight::default(),
                    target_rpe: domain::RPE::ZERO,
                    automatic: false,
                    countdown: true,
                }],
                exercise_notes: std::collections::BTreeMap::new(),
                exercise_measurements: std::collections::BTreeMap::new(),
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
            vec!["", "10 ×", "30 s", "50 kg", "@ 8", ""]
        );
    }

    #[test]
    fn test_a_set_with_a_side_is_shown_in_the_column_of_its_side() {
        let sided = |side| {
            let mut set = recorded_set(1);
            if let domain::TrainingSessionElement::Set { side: set_side, .. } = &mut set {
                *set_side = side;
            }
            set
        };
        let html = render_training_session(1, web_app::Settings::default(), move || {
            recorded_session().with_training_sessions(vec![domain::TrainingSession {
                id: 1.into(),
                routine_id: 1.into(),
                date: chrono::Local::now().date_naive(),
                notes: String::new(),
                elements: vec![
                    sided(domain::Side::Left),
                    sided(domain::Side::Right),
                    sided(domain::Side::Left),
                    recorded_set(1),
                    sided(domain::Side::Right),
                ],
                exercise_notes: std::collections::BTreeMap::new(),
                exercise_measurements: std::collections::BTreeMap::new(),
            }])
        });

        assert_eq!(
            rows_of(&html, "session")[1..],
            [
                vec!["", "Left", "Right", ""],
                vec!["", "10 × 50 kg @ 8 (30 s)", "10 × 50 kg @ 8 (30 s)", ""],
                vec!["", "10 × 50 kg @ 8 (30 s)", "", ""],
                vec!["", "10 ×", "30 s", "50 kg", "@ 8", ""],
                vec!["", "", "10 × 50 kg @ 8 (30 s)", ""],
            ]
        );
    }

    #[test]
    fn test_an_alternating_set_is_shown_like_a_set_without_a_side_marked_by_its_side() {
        let alternating = || {
            let mut set = recorded_set(1);
            if let domain::TrainingSessionElement::Set { side, .. } = &mut set {
                *side = domain::Side::Alternating;
            }
            set
        };
        let html = render_training_session(1, web_app::Settings::default(), move || {
            recorded_session().with_training_sessions(vec![domain::TrainingSession {
                id: 1.into(),
                routine_id: 1.into(),
                date: chrono::Local::now().date_naive(),
                notes: String::new(),
                elements: vec![alternating(), alternating()],
                exercise_notes: std::collections::BTreeMap::new(),
                exercise_measurements: std::collections::BTreeMap::new(),
            }])
        });

        assert_eq!(
            rows_of(&html, "session")[1..],
            [
                vec!["L+R", "10 ×", "30 s", "50 kg", "@ 8", ""],
                vec!["L+R", "10 ×", "30 s", "50 kg", "@ 8", ""],
            ]
        );
        assert!(!contains(&html, "side-header"));
    }

    #[test]
    fn test_a_right_set_directly_followed_by_its_left_set_shares_one_row_left_first() {
        let sided = |side, reps| {
            let mut set = recorded_set(1);
            if let domain::TrainingSessionElement::Set {
                side: set_side,
                reps: set_reps,
                ..
            } = &mut set
            {
                *set_side = side;
                *set_reps = domain::Reps::new(reps).unwrap();
            }
            set
        };
        let html = render_training_session(1, web_app::Settings::default(), move || {
            recorded_session().with_training_sessions(vec![domain::TrainingSession {
                id: 1.into(),
                routine_id: 1.into(),
                date: chrono::Local::now().date_naive(),
                notes: String::new(),
                elements: vec![
                    sided(domain::Side::Right, 11),
                    sided(domain::Side::Left, 12),
                ],
                exercise_notes: std::collections::BTreeMap::new(),
                exercise_measurements: std::collections::BTreeMap::new(),
            }])
        });

        assert_eq!(
            rows_of(&html, "session")[1..],
            [
                vec!["", "Left", "Right", ""],
                vec!["", "12 × 50 kg @ 8 (30 s)", "11 × 50 kg @ 8 (30 s)", ""]
            ]
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
            vec!["", "10 ×", "30 s", "50 kg", "@ 8", ""]
        );
    }

    #[rstest]
    #[case::reps_only(0, 0.0, vec!["", "10", "", "", "@ 8", ""])]
    #[case::with_time(30, 0.0, vec!["", "10 ×", "30 s", "", "@ 8", ""])]
    #[case::with_weight(0, 50.0, vec!["", "10 ×", "", "50 kg", "@ 8", ""])]
    fn test_the_reps_are_followed_by_a_multiplication_sign_only_before_a_time_or_weight(
        #[case] time: u32,
        #[case] weight: f32,
        #[case] expected: Vec<&'static str>,
    ) {
        let html = render_training_session(1, web_app::Settings::default(), move || {
            let mut set = recorded_set(1);
            if let domain::TrainingSessionElement::Set {
                time: set_time,
                weight: set_weight,
                ..
            } = &mut set
            {
                *set_time = domain::Time::new(time).unwrap();
                *set_weight = domain::Weight::new(weight).unwrap();
            }
            recorded_session().with_training_sessions(vec![domain::TrainingSession {
                id: 1.into(),
                routine_id: 1.into(),
                date: chrono::Local::now().date_naive(),
                notes: String::new(),
                elements: vec![set],
                exercise_notes: std::collections::BTreeMap::new(),
                exercise_measurements: std::collections::BTreeMap::new(),
            }])
        });

        assert_eq!(rows_of(&html, "session")[1], expected);
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
        countdown: bool,
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
                        side: domain::Side::Unset,
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
                        countdown,
                    }],
                    exercise_notes: std::collections::BTreeMap::new(),
                    exercise_measurements: std::collections::BTreeMap::new(),
                }])
        }
    }

    #[rstest]
    #[case::reps(&[3, 1, 1, 0], 8)]
    #[case::hold(&[30], 0)]
    fn test_a_set_with_a_tempo_shows_the_bar_and_keeps_its_input_fields(
        #[case] phases: &'static [u32],
        #[case] target_reps: u32,
    ) {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            tempo_session(phases, target_reps, false, false, false),
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
            tempo_session(&[3, 1, 1, 0], 8, true, true, false),
        );

        assert_eq!(text_of(&html, "countdown-detail"), "1/8");
        assert!(!contains(&html, "set-tempo-bar"), "{html}");
    }

    #[test]
    fn test_a_countdown_without_reps_has_no_detail() {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            tempo_session(&[3, 1, 1, 0], 0, true, true, false),
        );

        assert!(contains(&html, "countdown"), "{html}");
        assert!(!contains(&html, "countdown-detail"), "{html}");
    }

    #[test]
    fn test_a_countdown_awaiting_its_automatic_start_does_not_wait_for_a_tap() {
        assert_eq!(countdown_state(true), "running");
        assert_eq!(countdown_state(false), "waiting");
    }

    /// Renders a countdown that has not been started and returns its state.
    fn countdown_state(start_pending: bool) -> String {
        let html = render(move || {
            rsx! { UnstartedCountdown { side: domain::Side::Unset, start_delay: start_pending.then_some(StartDelayKind::LeadIn), remaining: 0.5 } }
        });

        attribute_of(&html, "countdown", "data-state")
    }

    #[rstest]
    #[case::right(domain::Side::Right, "Right side in 3 s")]
    #[case::left(domain::Side::Left, "Left side in 3 s")]
    fn test_a_side_awaiting_its_start_names_the_side_and_does_not_wait_for_a_tap(
        #[case] side: domain::Side,
        #[case] detail: &str,
    ) {
        let html = render(move || {
            rsx! { UnstartedCountdown { side, start_delay: Some(StartDelayKind::SideSwitch), remaining: 2.5 } }
        });

        assert_eq!(attribute_of(&html, "countdown", "data-state"), "running");
        assert_eq!(text_of(&html, "countdown-detail"), detail);
    }

    #[test]
    fn test_a_start_delay_counts_down_to_its_end() {
        let now = chrono::Utc::now();
        let delay = PendingStartDelay::new(StartDelayKind::SideSwitch, now);

        assert_approx_eq!(
            delay.remaining_at(now),
            f64::from(SIDE_SWITCH_SECONDS),
            1e-9
        );
        assert_approx_eq!(
            delay.remaining_at(delay.ends_at - chrono::Duration::milliseconds(1500)),
            1.5,
            1e-9
        );
        assert_approx_eq!(
            delay.remaining_at(delay.ends_at + chrono::Duration::seconds(1)),
            0.,
            1e-9
        );
    }

    #[rstest]
    #[case::manual(false)]
    #[case::automatic(true)]
    fn test_a_set_with_reps_and_the_countdown_is_counted_down_with_either_start(
        #[case] automatic: bool,
    ) {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            tempo_session(&[3, 1, 1, 0], 8, automatic, true, false),
        );

        assert_eq!(text_of(&html, "countdown-detail"), "1/8");
        assert!(!contains(&html, "set-tempo-bar"), "{html}");
    }

    #[test]
    fn test_an_automatic_set_with_input_fields_shows_its_bar() {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            tempo_session(&[3, 1, 1, 0], 8, true, false, false),
        );

        assert!(contains(&html, "set-tempo-bar"), "{html}");
        assert!(!contains(&html, "countdown"), "{html}");
    }

    #[test]
    fn test_a_tempo_bar_is_not_running_during_its_start_delay() {
        let html = render(|| {
            rsx! { DelayedTempoBar { side: domain::Side::Unset, kind: StartDelayKind::LeadIn } }
        });

        assert_eq!(
            attribute_of(&html, "set-tempo-bar", "data-state"),
            "stopped"
        );
        assert!(!contains(&html, "tempo-bar-detail"), "{html}");
    }

    #[test]
    fn test_a_tempo_bar_during_a_side_switch_names_the_side() {
        let html = render(|| {
            rsx! { DelayedTempoBar { side: domain::Side::Left, kind: StartDelayKind::SideSwitch } }
        });

        assert_eq!(text_of(&html, "tempo-bar-detail"), "Left side in 3 s");
    }

    #[component]
    fn DelayedTempoBar(side: domain::Side, kind: StartDelayKind) -> Element {
        let progress = use_store(|| Progress::new(1.into()));
        let phase_clock = PhaseClock {
            bar: use_signal(|| None),
            elapsed: use_signal(|| 0.),
            start_delay: use_signal(|| {
                Some(StartDelay {
                    element_idx: 0,
                    pending: Some(PendingStartDelay {
                        remaining: 2.5,
                        ..PendingStartDelay::new(kind, chrono::Utc::now())
                    }),
                })
            }),
        };

        rsx! {
            SetTempoBar {
                element_idx: 0,
                side,
                target_tempo: domain::Tempo::new(&[3, 1, 1, 0]).unwrap(),
                progress,
                phase_clock,
            }
        }
    }

    #[test]
    fn test_a_change_of_the_session_during_a_start_delay_keeps_the_delay() {
        let now = chrono::Utc::now();
        let delay = StartDelay::new(2, StartDelayKind::SideSwitch, now);
        let later = now + chrono::Duration::seconds(1);

        assert_eq!(
            tempo_bar_start_delay(Some(delay), 2, true, || StartDelayKind::LeadIn, later),
            delay
        );

        let mut training_session = training_session(vec![planned_set(1), rest(), planned_set(2)]);
        let map = training_session.add_set(0);
        let followed = on_phase_clock(move |phase_clock| {
            phase_clock.start_delay.set(Some(delay));
            phase_clock.follow(&map);
            *phase_clock.start_delay.peek()
        });

        assert_eq!(
            followed,
            Some(StartDelay {
                element_idx: 4,
                ..delay
            })
        );
        assert_eq!(
            tempo_bar_start_delay(followed, 4, true, || StartDelayKind::LeadIn, later),
            followed.unwrap()
        );
    }

    #[test]
    fn test_a_tap_on_the_tempo_bar_during_its_start_delay_starts_the_bar() {
        let (delay, is_running) = on_phase_clock(|phase_clock| {
            phase_clock.start_delay.set(Some(StartDelay::new(
                0,
                StartDelayKind::LeadIn,
                chrono::Utc::now(),
            )));
            phase_clock.tap_bar(0);
            (
                *phase_clock.start_delay.peek(),
                phase_clock.bar_is_running(0),
            )
        });

        assert_eq!(delay.map(|delay| delay.pending), Some(None));
        assert!(is_running);
    }

    #[test]
    fn test_input_during_a_start_delay_ends_it_for_good() {
        let now = chrono::Utc::now();
        let delay = StartDelay::new(0, StartDelayKind::LeadIn, now);

        let ended = tempo_bar_start_delay(Some(delay), 0, false, || StartDelayKind::LeadIn, now);
        assert_eq!(ended, delay.ended());

        // Clearing the entered value lets the set await its start again.
        assert_eq!(
            tempo_bar_start_delay(Some(ended), 0, true, || StartDelayKind::LeadIn, now),
            ended
        );
    }

    #[test]
    fn test_a_start_delay_is_started_for_a_set_that_has_become_current() {
        let now = chrono::Utc::now();
        let previous = StartDelay::new(0, StartDelayKind::LeadIn, now).ended();

        assert_eq!(
            tempo_bar_start_delay(Some(previous), 2, true, || StartDelayKind::SideSwitch, now),
            StartDelay::new(2, StartDelayKind::SideSwitch, now)
        );
    }

    #[test]
    fn test_a_resumed_session_starts_the_delay_of_a_pair_with_the_lead_in() {
        let mut progress = Progress::new(1.into());
        progress.set_element_idx(0);
        progress.complete(0);
        let resumed = Progress::from(web_app::OngoingTrainingSession::from(progress));

        assert_eq!(
            start_delay_kind(resumed.completed_preceding, true),
            StartDelayKind::LeadIn
        );
    }

    #[test]
    fn test_the_end_of_a_start_delay_starts_the_bar_of_its_current_set() {
        let now = chrono::Utc::now();
        let ended = now + chrono::Duration::seconds(1);
        let (starts_timer, is_running) = on_phase_clock(move |phase_clock| {
            phase_clock
                .start_delay
                .set(Some(StartDelay::new(0, StartDelayKind::LeadIn, now)));
            let starts_timer = phase_clock.tick_start_delay(0, false, ended);
            (starts_timer, phase_clock.bar_is_running(0))
        });

        assert!(!starts_timer);
        assert!(is_running);
    }

    #[test]
    fn test_a_start_delay_ending_after_another_element_has_become_current_starts_no_bar() {
        let now = chrono::Utc::now();
        let ended = now + chrono::Duration::seconds(1);
        let (starts_timer, has_bar) = on_phase_clock(move |phase_clock| {
            phase_clock
                .start_delay
                .set(Some(StartDelay::new(0, StartDelayKind::LeadIn, now)));
            let starts_timer = phase_clock.tick_start_delay(1, true, ended);
            (
                starts_timer,
                phase_clock.has_bar_clock(0) || phase_clock.has_bar_clock(1),
            )
        });

        assert!(!starts_timer);
        assert!(!has_bar);
    }

    #[test]
    fn test_a_countdown_after_its_side_switch_shows_its_repetition() {
        let html = render(|| {
            rsx! { UnstartedCountdown { side: domain::Side::Right, start_delay: None, remaining: 0. } }
        });

        assert_eq!(text_of(&html, "countdown-detail"), "1/8");
    }

    /// Runs `scenario` on a phase clock inside a rendered component and returns its result.
    fn on_phase_clock<T: 'static>(scenario: impl Fn(&mut PhaseClock) -> T + 'static) -> T {
        let result = Rc::new(RefCell::new(None));
        let output = Rc::clone(&result);
        let scenario: Rc<dyn Fn(&mut PhaseClock)> = Rc::new(move |phase_clock| {
            *output.borrow_mut() = Some(scenario(phase_clock));
        });
        render(move || {
            provide_context(Rc::clone(&scenario));
            rsx! { PhaseClockScenario {} }
        });
        result.borrow_mut().take().unwrap()
    }

    #[component]
    fn PhaseClockScenario() -> Element {
        let scenario = use_context::<Rc<dyn Fn(&mut PhaseClock)>>();
        let mut phase_clock = PhaseClock {
            bar: use_signal(|| None),
            elapsed: use_signal(|| 0.),
            start_delay: use_signal(|| None),
        };
        use_hook(|| scenario(&mut phase_clock));
        rsx! { div {} }
    }

    #[component]
    fn UnstartedCountdown(
        side: domain::Side,
        start_delay: Option<StartDelayKind>,
        remaining: f64,
    ) -> Element {
        let progress = use_store(|| {
            let mut progress = Progress::new(1.into());
            progress.set_element_idx(0);
            progress
        });
        let phase_clock = PhaseClock {
            bar: use_signal(|| None),
            elapsed: use_signal(|| 0.),
            start_delay: use_signal(|| {
                Some(StartDelay {
                    element_idx: 0,
                    pending: start_delay.map(|kind| PendingStartDelay {
                        remaining,
                        ..PendingStartDelay::new(kind, chrono::Utc::now())
                    }),
                })
            }),
        };

        rsx! {
            SetCountdown {
                side,
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
            exercise_measurements: std::collections::BTreeMap::new(),
        }])
    }

    #[test]
    fn test_a_set_without_a_tempo_keeps_its_input_fields() {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            tempo_session(&[], 8, true, true, false),
        );

        assert!(!contains(&html, "countdown"), "{html}");
        assert!(!contains(&html, "set-tempo-bar"), "{html}");
    }

    #[test]
    fn test_a_recorded_set_is_neither_counted_down_nor_guided() {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            tempo_session(&[3, 1, 1, 0], 8, true, true, true),
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
                tempo_session(&[3, 1, 1, 0], target_reps, true, true, false),
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
            tempo_session(&[3, 1, 1, 0], 8, false, false, false),
        );

        assert_eq!(
            cells_of(&html, "column-header"),
            vec!["", "Reps", "Time (s)", "Weight (kg)", "RPE", ""]
        );
    }

    fn stretch_session(
        muscles: Vec<domain::ExerciseMuscle>,
        time: u32,
    ) -> impl Fn() -> TestCache + 'static {
        move || {
            TestCache::default()
                .with_exercises(vec![domain::Exercise {
                    muscles: muscles.clone(),
                    ..exercise(1, "Forward Fold")
                }])
                .with_training_sessions(vec![domain::TrainingSession {
                    id: 1.into(),
                    routine_id: 1.into(),
                    date: chrono::Local::now().date_naive(),
                    notes: String::new(),
                    elements: vec![domain::TrainingSessionElement::Set {
                        exercise_id: 1.into(),
                        side: domain::Side::Unset,
                        reps: domain::Reps::default(),
                        time: domain::Time::new(time).unwrap(),
                        weight: domain::Weight::default(),
                        rpe: domain::RPE::ZERO,
                        target_reps: domain::Reps::default(),
                        target_tempo: domain::Tempo::default(),
                        target_weight: domain::Weight::default(),
                        target_rpe: domain::RPE::ZERO,
                        automatic: false,
                        countdown: false,
                    }],
                    exercise_notes: std::collections::BTreeMap::new(),
                    exercise_measurements: std::collections::BTreeMap::new(),
                }])
        }
    }

    fn stretched(muscle_id: domain::MuscleID) -> domain::ExerciseMuscle {
        domain::ExerciseMuscle {
            muscle_id,
            stimulus: domain::Stimulus::NONE,
            stretch: domain::Stimulus::PRIMARY,
        }
    }

    fn worked(muscle_id: domain::MuscleID) -> domain::ExerciseMuscle {
        domain::ExerciseMuscle {
            muscle_id,
            stimulus: domain::Stimulus::PRIMARY,
            stretch: domain::Stimulus::NONE,
        }
    }

    #[test]
    fn test_the_stretch_per_muscle_of_a_session_with_a_stretch_is_shown() {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            stretch_session(vec![stretched(domain::MuscleID::Hamstrings)], 90),
        );

        assert_eq!(
            text_of(&html, "stretch-sets-per-muscle"),
            "Hamstrings1Back of thighs"
        );
        assert_eq!(
            text_of(&html, "stretch-time-per-muscle"),
            "Hamstrings1.5 minBack of thighs"
        );
    }

    #[rstest::rstest]
    #[case::hybrid(
        vec![worked(domain::MuscleID::ErectorSpinae), stretched(domain::MuscleID::Hamstrings)],
        90
    )]
    #[case::unrecorded_stretch(vec![stretched(domain::MuscleID::Hamstrings)], 0)]
    fn test_no_stretch_per_muscle_is_shown_without_stretch_sets(
        #[case] muscles: Vec<domain::ExerciseMuscle>,
        #[case] time: u32,
    ) {
        let html = render_training_session(
            1,
            web_app::Settings::default(),
            stretch_session(muscles, time),
        );

        assert!(!contains(&html, "stretch-sets-per-muscle"));
        assert!(!contains(&html, "stretch-time-per-muscle"));
    }

    #[rstest::rstest]
    #[case::stretch(vec![stretched(domain::MuscleID::Hamstrings)], true)]
    #[case::hybrid(
        vec![worked(domain::MuscleID::ErectorSpinae), stretched(domain::MuscleID::Hamstrings)],
        false
    )]
    fn test_an_unprescribed_stretch_shows_the_time_column_without_the_tut_setting(
        #[case] muscles: Vec<domain::ExerciseMuscle>,
        #[case] expected: bool,
    ) {
        let html = render_training_session(
            1,
            web_app::Settings {
                show_tut: false,
                ..web_app::Settings::default()
            },
            stretch_session(muscles, 0),
        );

        assert_eq!(
            cells_of(&html, "column-header").contains(&"Time (s)".to_string()),
            expected
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
                            side: domain::Side::Unset,
                            reps: domain::Reps::default(),
                            time: domain::Time::default(),
                            weight: domain::Weight::default(),
                            rpe: domain::RPE::ZERO,
                            target_reps: domain::Reps::new(10).unwrap(),
                            target_tempo: domain::Tempo::default(),
                            target_weight: domain::Weight::default(),
                            target_rpe: domain::RPE::NINE,
                            automatic: false,
                            countdown: false,
                        }],
                        exercise_notes: std::collections::BTreeMap::new(),
                        exercise_measurements: std::collections::BTreeMap::new(),
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
                    side: domain::Side::Unset,
                    reps: domain::Reps::default(),
                    time: domain::Time::default(),
                    weight: domain::Weight::default(),
                    rpe: domain::RPE::ZERO,
                    target_reps: domain::Reps::new(10).unwrap(),
                    target_tempo: domain::Tempo::new(&[3, 1, 1, 0]).unwrap(),
                    target_weight: domain::Weight::default(),
                    target_rpe: domain::RPE::NINE,
                    automatic: false,
                    countdown: false,
                };
                let recorded_set = domain::TrainingSessionElement::Set {
                    exercise_id: 1.into(),
                    side: domain::Side::Unset,
                    reps: domain::Reps::new(10).unwrap(),
                    time: domain::Time::new(3).unwrap(),
                    weight: domain::Weight::new(50.0).unwrap(),
                    rpe: domain::RPE::EIGHT,
                    target_reps: domain::Reps::default(),
                    target_tempo: domain::Tempo::default(),
                    target_weight: domain::Weight::default(),
                    target_rpe: domain::RPE::ZERO,
                    automatic: false,
                    countdown: false,
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
                            exercise_measurements: std::collections::BTreeMap::new(),
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
                exercise_measurements: std::collections::BTreeMap::new(),
            });
            TestCache::default()
                .with_exercises(vec![exercise(1, "Squat")])
                .with_training_sessions(training_sessions)
        }
    }

    fn planned_set(exercise_id: u128) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            side: domain::Side::Unset,
            reps: domain::Reps::default(),
            time: domain::Time::default(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::ZERO,
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::ZERO,
            automatic: false,
            countdown: false,
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
            exercise_measurements: std::collections::BTreeMap::new(),
        }
    }

    fn performed_set(exercise_id: u128, weight: f32) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            side: domain::Side::Unset,
            reps: domain::Reps::new(10).unwrap(),
            time: domain::Time::default(),
            weight: domain::Weight::new(weight).unwrap(),
            rpe: domain::RPE::ZERO,
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::ZERO,
            automatic: false,
            countdown: false,
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
        set_index: Option<usize>,
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
        let history = history
            .into_iter()
            .map(|(date, sets)| (date, sets.into(), set_index))
            .collect::<Vec<RecentSession>>();
        rsx! {
            table {
                tbody {
                    {set_value_buttons(
                        set_buttons,
                        &history,
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
        set_index: Option<usize>,
    ) -> String {
        render_expanded_history_with_set_buttons(history, set_index, true)
    }

    fn render_expanded_history_with_set_buttons(
        history: Vec<(chrono::NaiveDate, Vec<domain::Set>)>,
        set_index: Option<usize>,
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
            Some(0),
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
            Some(1),
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
            Some(0),
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
            None,
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
    use assert_approx_eq::assert_approx_eq;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    fn set(exercise_id: u128, reps: u32) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            side: domain::Side::Unset,
            reps: domain::Reps::new(reps).unwrap(),
            time: domain::Time::default(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::default(),
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::default(),
            automatic: false,
            countdown: false,
        }
    }

    fn set_with_target_weight(
        exercise_id: u128,
        target_weight: f32,
    ) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            side: domain::Side::Unset,
            reps: domain::Reps::default(),
            time: domain::Time::default(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::default(),
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::new(target_weight).unwrap(),
            target_rpe: domain::RPE::default(),
            automatic: false,
            countdown: false,
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
            exercise_measurements: std::collections::BTreeMap::new(),
        }
    }

    fn set_field_values(reps: u32) -> SetFieldValues {
        SetFieldValues {
            reps: FieldValue::new(domain::Reps::new(reps).unwrap()),
            time: FieldValue::new(domain::Time::default()),
            weight: FieldValue::new(domain::Weight::default()),
            rpe: FieldValue::new(domain::RPE::default()),
            side: FieldValue::new(domain::Side::Unset),
        }
    }

    fn unperformed_set_field_values() -> SetFieldValues {
        SetFieldValues {
            reps: FieldValue::new_with_empty_default(domain::Reps::default()),
            time: FieldValue::new_with_empty_default(domain::Time::default()),
            weight: FieldValue::new_with_empty_default(domain::Weight::default()),
            rpe: FieldValue::new_with_empty_default(domain::RPE::default()),
            side: FieldValue::new(domain::Side::Unset),
        }
    }

    #[test]
    fn test_the_side_cycles_through_none_left_right_and_alternating() {
        let mut values = unperformed_set_field_values();

        let sides = (0..4)
            .map(|_| {
                values.cycle_side();
                values.side.validated.clone().unwrap()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            sides,
            [
                domain::Side::Left,
                domain::Side::Right,
                domain::Side::Alternating,
                domain::Side::Unset
            ]
        );
        assert!(!values.changed());
    }

    #[test]
    fn test_a_side_changed_on_an_unperformed_set_is_saved_but_keeps_it_unperformed() {
        let mut values = unperformed_set_field_values();

        values.cycle_side();

        assert!(values.is_empty());
        assert!(values.is_unperformed());
        assert!(values.changed());
        assert!(values.has_valid_changes());
        assert!(!values.has_pending_save());
    }

    #[test]
    fn test_a_side_changed_on_a_recorded_set_offers_no_values() {
        let mut values = set_field_values(10);

        values.cycle_side();

        assert!(!values.is_unperformed());
        assert!(!values.values_changed());
        assert!(values.changed());
        assert!(!values.offers_values());
        assert!(values.has_pending_save());
    }

    fn weight(value: f32) -> domain::Weight {
        domain::Weight::new(value).unwrap()
    }

    fn filled_weights(groups: &[Vec<usize>], weights: &[domain::Weight]) -> Vec<(usize, String)> {
        let mut field_values =
            HashMap::from([(0, set_field_values(10)), (1, set_field_values(10))]);

        fill_weights(&mut field_values, groups, weights);

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
            filled_weights(&[vec![0], vec![1]], &[weight(50.0)]),
            vec![(0, "50".to_string()), (1, "0".to_string())]
        );
    }

    #[test]
    fn test_a_surplus_weight_is_ignored() {
        assert_eq!(
            filled_weights(&[vec![0]], &[weight(50.0), weight(40.0)]),
            vec![(0, "50".to_string()), (1, "0".to_string())]
        );
    }

    #[test]
    fn test_an_element_without_fields_is_skipped() {
        assert_eq!(
            filled_weights(&[vec![2], vec![1]], &[weight(50.0), weight(40.0)]),
            vec![(0, "0".to_string()), (1, "40".to_string())]
        );
    }

    #[test]
    fn test_completing_an_element_moves_on_from_it() {
        let mut progress = Progress::new(1.into());
        progress.set_element_idx(0);

        progress.complete(0);

        assert_eq!(progress.element_idx, 1);
        assert!(progress.completed_preceding);
    }

    #[test]
    fn test_completing_an_element_other_than_the_current_one_does_not_complete_the_preceding_one() {
        let mut progress = Progress::new(1.into());
        progress.set_element_idx(5);

        progress.complete(2);

        assert_eq!(progress.element_idx, 3);
        assert!(!progress.completed_preceding);
    }

    #[test]
    fn test_choosing_an_element_does_not_complete_the_preceding_one() {
        let mut progress = Progress::new(1.into());
        progress.complete(0);

        progress.set_element_idx(1);

        assert!(!progress.completed_preceding);
    }

    #[test]
    fn test_the_current_element_keeps_its_start_when_moved() {
        let mut training_session = training_session(vec![
            planned_set(1),
            rest(),
            planned_set(2),
            rest(),
            planned_set(3),
        ]);
        let map = training_session.move_section_up(1);
        let mut progress = Progress::new(1.into());
        progress.set_element_idx(0);
        let element_start_time = progress.element_start_time;

        progress.follow(&map);

        assert_eq!(progress.element_idx, 2);
        assert_eq!(progress.element_start_time, element_start_time);
    }

    #[test]
    fn test_the_element_following_a_removed_current_element_starts_anew() {
        let mut training_session = training_session(vec![
            planned_set(1),
            rest(),
            planned_set(1),
            rest(),
            planned_set(2),
        ]);
        let map = training_session.remove_set(0);
        let mut progress = Progress::new(1.into());
        progress.set_element_idx(2);
        progress.element_start_time = chrono::DateTime::<chrono::Utc>::default();

        progress.follow(&map);

        assert_eq!(progress.element_idx, 2);
        assert_ne!(
            progress.element_start_time,
            chrono::DateTime::<chrono::Utc>::default()
        );
    }

    #[test]
    fn test_both_elements_of_a_pair_get_the_weight_of_the_group() {
        assert_eq!(
            filled_weights(&[vec![0, 1]], &[weight(50.0), weight(40.0)]),
            vec![(0, "50".to_string()), (1, "50".to_string())]
        );
    }

    #[test]
    fn test_the_start_weight_is_read_from_the_first_element_of_the_first_group() {
        let mut first = set_field_values(10);
        first.weight = FieldValue::new(weight(80.0));
        let mut second = set_field_values(10);
        second.weight = FieldValue::new(weight(70.0));
        let field_values = HashMap::from([(1, first), (0, second)]);
        let training_session = training_session(vec![]);

        assert_eq!(
            drop_set_start_weight(
                &field_values,
                None,
                &training_session,
                &[vec![1, 0], vec![2]]
            ),
            Some(weight(80.0))
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
    fn test_modify_training_session_elements_keeps_stored_value_of_invalid_input() {
        let mut training_session = training_session(vec![set(1, 5)]);
        let field_values = SetFieldValues {
            reps: invalid_reps("1000"),
            ..set_field_values(10)
        };

        modify_training_session_elements(
            &mut training_session,
            &HashMap::from([(0, field_values)]),
        );

        assert_eq!(training_session.elements, vec![set(1, 5)]);
    }

    #[test]
    fn test_rebuilt_set_keeps_invalid_input() {
        let previous = SetFieldValues {
            reps: invalid_reps("1000"),
            ..set_field_values(10)
        };

        let values = set_field_values(5).with_unstored_inputs_of(&previous);

        assert_eq!(values.reps.input, "1000");
        assert!(!values.reps.valid());
        assert!(values.reps.changed());
    }

    #[test]
    fn test_rebuilt_set_keeps_unstored_input() {
        let mut previous = set_field_values(5);
        previous.reps.input = "10".to_string();
        previous.reps.validated = Ok(domain::Reps::new(10).unwrap());

        let values = set_field_values(5).with_unstored_inputs_of(&previous);

        assert_eq!(values.reps.validated, Ok(domain::Reps::new(10).unwrap()));
        assert!(values.reps.changed());
    }

    #[test]
    fn test_rebuilt_set_shows_stored_value_instead_of_unchanged_input() {
        let values = set_field_values(10).with_unstored_inputs_of(&set_field_values(5));

        assert_eq!(values, set_field_values(10));
    }

    #[test]
    fn test_rebuilt_set_shows_stored_input() {
        let mut previous = set_field_values(5);
        previous.reps.input = "010".to_string();
        previous.reps.validated = Ok(domain::Reps::new(10).unwrap());

        let values = set_field_values(10).with_unstored_inputs_of(&previous);

        assert_eq!(values, set_field_values(10));
    }

    fn invalid_reps(input: &str) -> FieldValue<domain::Reps> {
        FieldValue {
            input: input.to_string(),
            validated: domain::Reps::try_from(input).map_err(|err| err.to_string()),
            orig: String::new(),
        }
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

    fn measurement(value: f64) -> domain::Measurement {
        domain::Measurement::new(value).unwrap()
    }

    fn session_with_measurements(
        id: u128,
        days_ago: i64,
        sides: &[domain::Side],
        measurements: &[(domain::Side, f64)],
    ) -> domain::TrainingSession {
        domain::TrainingSession {
            id: id.into(),
            routine_id: 1.into(),
            date: chrono::Local::now().date_naive() - chrono::Duration::days(days_ago),
            notes: String::new(),
            elements: sides
                .iter()
                .map(|side| {
                    let mut set = recorded_set(1);
                    if let domain::TrainingSessionElement::Set { side: s, .. } = &mut set {
                        *s = *side;
                    }
                    set
                })
                .collect(),
            exercise_notes: std::collections::BTreeMap::new(),
            exercise_measurements: measurements
                .iter()
                .map(|(side, value)| ((1.into(), *side), measurement(*value)))
                .collect(),
        }
    }

    #[rstest]
    #[case::without_measurement(&[], None)]
    #[case::side_less(&[(domain::Side::Unset, -4.5)], Some("-4.5"))]
    #[case::per_side(
        &[(domain::Side::Left, 12.5), (domain::Side::Right, 10.0)],
        Some("L 12.5 · R 10")
    )]
    #[case::both_forms(
        &[(domain::Side::Unset, 4.5), (domain::Side::Left, 12.5), (domain::Side::Right, 10.0)],
        Some("4.5 · L 12.5 · R 10")
    )]
    fn test_the_measurement_of_an_exercise_is_shown_in_a_row(
        #[case] measurements: &'static [(domain::Side, f64)],
        #[case] expected: Option<&str>,
    ) {
        let html = render_training_session(1, web_app::Settings::default(), move || {
            recorded_session().with_training_sessions(vec![session_with_measurements(
                1,
                0,
                &[domain::Side::Unset],
                measurements,
            )])
        });

        assert_eq!(
            contains(&html, "session-exercise-measurement")
                .then(|| text_of(&html, "session-exercise-measurement")),
            expected.map(str::to_string)
        );
        if expected.is_some() {
            assert_eq!(
                attribute_of(&html, "session-exercise-measurement", "aria-label"),
                "Measurement"
            );
        }
    }

    fn render_measurement_dialog(
        session: domain::TrainingSession,
        previous: Vec<domain::TrainingSession>,
    ) -> String {
        render(move || {
            let mut sessions = previous.clone();
            sessions.push(session.clone());
            recorded_session()
                .with_routines(vec![domain::Routine {
                    id: 1.into(),
                    name: domain::Name::new("R").unwrap(),
                    notes: String::new(),
                    archived: false,
                    sections: vec![],
                }])
                .with_training_sessions(sessions)
                .provide();
            rsx! {
                SessionExerciseNotesDialog {
                    training_session: session.clone(),
                    exercise_id: domain::ExerciseID::from(1),
                    on_save: |_| {},
                    on_close: |_| {},
                }
            }
        })
    }

    #[rstest]
    #[case::sets_without_side(&[domain::Side::Unset], &[], &["Measurement"])]
    #[case::sets_with_side(&[domain::Side::Left, domain::Side::Right], &[], &["Left", "Right"])]
    #[case::stored_value_per_side(
        &[domain::Side::Unset],
        &[(domain::Side::Right, 1.0)],
        &["Measurement", "Right"]
    )]
    fn test_the_measurement_fields_follow_the_sides_of_the_sets(
        #[case] sides: &'static [domain::Side],
        #[case] measurements: &'static [(domain::Side, f64)],
        #[case] expected: &[&str],
    ) {
        let html =
            render_measurement_dialog(session_with_measurements(1, 0, sides, measurements), vec![]);

        for label in ["Measurement", "Left", "Right"] {
            assert_eq!(
                contains(
                    &html,
                    &format!(
                        "session-exercise-measurement-input-{}",
                        label.to_lowercase()
                    )
                ),
                expected.contains(&label),
                "{label}"
            );
        }
    }

    #[test]
    fn test_a_previous_measurement_offers_a_reuse_only_for_matching_sides() {
        let html = render_measurement_dialog(
            session_with_measurements(1, 0, &[domain::Side::Unset], &[]),
            vec![
                session_with_measurements(
                    2,
                    2,
                    &[domain::Side::Unset],
                    &[(domain::Side::Unset, 3.0)],
                ),
                session_with_measurements(
                    3,
                    1,
                    &[domain::Side::Left],
                    &[(domain::Side::Left, 2.0), (domain::Side::Right, 1.5)],
                ),
            ],
        );

        assert_eq!(
            all_text_of(&html, "previous-session-exercise-measurement"),
            vec!["L 2 · R 1.5", "3"]
        );
        assert_eq!(
            all_text_of(&html, "session-exercise-measurement-reuse").len(),
            1
        );
    }

    fn measurement_field(input: &str) -> FieldValue<Option<domain::Measurement>> {
        FieldValue {
            input: input.to_string(),
            validated: Ok(None),
            orig: String::new(),
        }
    }

    #[rstest]
    #[case::negative("-12.5", Ok(Some(-12.5)))]
    #[case::empty("", Ok(None))]
    #[case::finer_resolution("1.005", Err("measurement must be a multiple of 0.01"))]
    fn test_update_measurement_field(
        #[case] value: &str,
        #[case] expected: Result<Option<f64>, &str>,
    ) {
        let mut field = measurement_field("1");

        update_measurement_field(&mut field, value, false);

        assert_eq!(field.input, value);
        assert_eq!(
            field.validated,
            expected
                .map(|value| value.map(measurement))
                .map_err(str::to_string)
        );
    }

    #[test]
    fn test_update_measurement_field_with_bad_input_keeps_the_input() {
        let mut field = measurement_field("-");

        update_measurement_field(&mut field, "", true);

        assert_eq!(field.input, "-");
        assert_eq!(
            field.validated,
            Err("measurement must be a decimal".to_string())
        );
    }

    #[rstest]
    #[case::one_field(&[domain::Side::Unset], &[(domain::Side::Unset, 4.5)])]
    #[case::both_fields(
        &[domain::Side::Left, domain::Side::Right],
        &[(domain::Side::Left, 2.0), (domain::Side::Right, -1.5)]
    )]
    fn test_reuse_measurements(
        #[case] sides: &[domain::Side],
        #[case] values: &[(domain::Side, f64)],
    ) {
        let mut fields = sides
            .iter()
            .map(|side| (*side, measurement_field("-")))
            .collect::<BTreeMap<_, _>>();
        let values = values
            .iter()
            .map(|(side, value)| (*side, measurement(*value)))
            .collect::<BTreeMap<_, _>>();

        reuse_measurements(&mut fields, &values);

        assert_eq!(
            fields
                .into_iter()
                .map(|(side, field)| (side, field.input, field.validated))
                .collect::<Vec<_>>(),
            values
                .into_iter()
                .map(|(side, value)| (side, value.to_string(), Ok(Some(value))))
                .collect::<Vec<_>>()
        );
    }

    /// Render session 1 holding `elements` after `earlier_sessions`.
    fn render_after(
        earlier_sessions: Vec<domain::TrainingSession>,
        elements: Vec<domain::TrainingSessionElement>,
    ) -> String {
        render(move || {
            let mut training_sessions = earlier_sessions.clone();
            training_sessions.push(earlier_session(1, 0, elements.clone()));
            TestCache::default()
                .with_exercises(vec![exercise(1, "Pull Up")])
                .with_training_sessions(training_sessions)
                .provide();
            provide_settings(web_app::Settings::default());
            provide_ongoing_training_session(State::None);
            rsx! { TrainingSession { id: domain::TrainingSessionID::from(1) } }
        })
    }

    fn sided_set(side: domain::Side, reps: u32) -> domain::TrainingSessionElement {
        let mut element = set(1, reps);
        if let domain::TrainingSessionElement::Set { side: s, .. } = &mut element {
            *s = side;
        }
        element
    }

    #[rstest::rstest]
    #[case::all_time(21, Some("personal-record"))]
    #[case::recent(11, Some("recent-personal-record"))]
    #[case::equal(10, None)]
    fn test_a_record_set_carries_its_marker_at_the_end_of_its_row(
        #[case] reps: u32,
        #[case] expected: Option<&str>,
    ) {
        let html = render_after(
            vec![
                earlier_session(2, 400, vec![set(1, 20)]),
                earlier_session(3, 10, vec![set(1, 10)]),
            ],
            vec![set(1, reps)],
        );

        for marker in ["personal-record", "recent-personal-record"] {
            let rows = rows_containing(&html, "session", marker);
            assert_eq!(
                rows.iter().any(|row| row.len() == 6 && row[5]),
                expected == Some(marker),
                "{marker}"
            );
        }
    }

    #[test]
    fn test_a_record_set_with_unsaved_changes_shows_the_check() {
        let record_sets = BTreeMap::from([(0, domain::RecordKind::AllTime)]);
        let mut values = set_field_values(21);

        assert_eq!(
            saved_record(&record_sets, 0, &values),
            Some(domain::RecordKind::AllTime)
        );

        values.reps.input = "22".to_string();
        values.reps.validated = Ok(domain::Reps::new(22).unwrap());

        assert_eq!(saved_record(&record_sets, 0, &values), None);
    }

    #[test]
    fn test_the_right_set_of_a_pair_carries_the_marker_of_its_record() {
        let html = render_after(
            vec![earlier_session(
                2,
                10,
                vec![
                    sided_set(domain::Side::Left, 10),
                    sided_set(domain::Side::Right, 10),
                ],
            )],
            vec![
                sided_set(domain::Side::Right, 12),
                sided_set(domain::Side::Left, 9),
            ],
        );

        assert_eq!(
            rows_containing(&html, "session", "personal-record")
                .into_iter()
                .filter(|row| row.contains(&true))
                .collect::<Vec<_>>(),
            [vec![false, false, true, false]]
        );
    }

    #[test]
    fn test_only_the_first_of_two_equal_record_sets_carries_the_marker() {
        let html = render_after(
            vec![earlier_session(2, 10, vec![set(1, 20)])],
            vec![set(1, 21), set(1, 21)],
        );

        assert_eq!(
            rows_containing(&html, "session", "personal-record")
                .into_iter()
                .filter(|row| row.len() == 6)
                .map(|row| row[5])
                .collect::<Vec<_>>(),
            [true, false]
        );
    }

    /// The targets of the set of a session after one 10 days ago holding `earlier`.
    fn targets_after(
        earlier: Vec<domain::TrainingSessionElement>,
        element: domain::TrainingSessionElement,
        values: &SetFieldValues,
    ) -> domain::RecordTargets {
        targets_of_sets_after(earlier, vec![(element, values.clone())])[&0].entered
    }

    /// The targets of the sets of a session after one 10 days ago holding `earlier`.
    fn targets_of_sets_after(
        earlier: Vec<domain::TrainingSessionElement>,
        sets: Vec<(domain::TrainingSessionElement, SetFieldValues)>,
    ) -> HashMap<usize, SetRecordTargets> {
        let (elements, values): (Vec<_>, Vec<_>) = sets.into_iter().unzip();
        let current = earlier_session(1, 0, elements);
        let personal_records = domain::PersonalRecords::with_targets(
            &[earlier_session(2, 10, earlier), current.clone()],
            &[],
            current.id,
        );
        record_targets(
            &personal_records,
            &current,
            &values.into_iter().enumerate().collect(),
        )
    }

    fn element(
        reps: u32,
        time: u32,
        weight: f32,
        target_tempo: &[u32],
    ) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: 1.into(),
            side: domain::Side::Unset,
            reps: domain::Reps::new(reps).unwrap(),
            time: domain::Time::new(time).unwrap(),
            weight: domain::Weight::new(weight).unwrap(),
            rpe: domain::RPE::ZERO,
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::new(target_tempo).unwrap(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::ZERO,
            automatic: false,
            countdown: !target_tempo.is_empty(),
        }
    }

    fn weighted_set(reps: u32, weight: f32) -> domain::TrainingSessionElement {
        element(reps, 0, weight, &[])
    }

    fn hold(time: u32) -> domain::TrainingSessionElement {
        element(0, time, 0.0, &[])
    }

    #[test]
    fn test_the_reps_target_of_a_weighted_set() {
        let mut values = unperformed_set_field_values();
        values.weight.input = "100".to_string();
        values.weight.validated = Ok(domain::Weight::new(100.0).unwrap());

        assert_eq!(
            targets_after(vec![weighted_set(5, 100.0)], planned_set(1), &values).reps,
            Some(domain::Reps::new(6).unwrap())
        );
    }

    #[test]
    fn test_the_reps_and_the_time_target_of_an_unweighted_set_are_shown() {
        let html = render_training_session(1, web_app::Settings::default(), || {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Plank")])
                .with_training_sessions(vec![
                    earlier_session(2, 10, vec![set(1, 10), hold(30)]),
                    earlier_session(1, 0, vec![planned_set(1)]),
                ])
        });

        assert_eq!(all_text_of(&html, "record-target"), ["11", "31"]);
    }

    #[rstest::rstest]
    #[case::reps_entered("5", "", None, None)]
    #[case::time_entered("", "20", Some(11), None)]
    #[case::nothing_entered("", "", Some(11), Some(31))]
    fn test_a_target_is_shown_only_below_an_empty_field(
        #[case] reps: &str,
        #[case] time: &str,
        #[case] reps_target: Option<u32>,
        #[case] time_target: Option<u32>,
    ) {
        let mut values = unperformed_set_field_values();
        values.reps.input = reps.to_string();
        values.reps.validated = Ok(domain::Reps::try_from(reps).unwrap_or_default());
        values.time.input = time.to_string();
        values.time.validated = Ok(domain::Time::try_from(time).unwrap_or_default());

        assert_eq!(
            shown_targets(
                targets_after(vec![set(1, 10), hold(30)], planned_set(1), &values),
                &values
            ),
            domain::RecordTargets {
                reps: reps_target.map(|reps| domain::Reps::new(reps).unwrap()),
                time: time_target.map(|time| domain::Time::new(time).unwrap()),
            }
        );
    }

    #[test]
    fn test_the_target_beats_the_unsaved_values_of_an_earlier_set() {
        let mut entered = unperformed_set_field_values();
        entered.reps = FieldValue::new(domain::Reps::new(12).unwrap());

        assert_eq!(
            targets_of_sets_after(
                vec![set(1, 10)],
                vec![
                    (planned_set(1), entered),
                    (planned_set(1), unperformed_set_field_values()),
                ],
            )[&1]
                .entered
                .reps,
            Some(domain::Reps::new(13).unwrap())
        );
    }

    #[test]
    fn test_the_saved_target_ignores_an_entered_weight_without_a_weight_record() {
        let mut values = unperformed_set_field_values();
        values.weight.input = "10".to_string();
        values.weight.validated = Ok(domain::Weight::new(10.0).unwrap());

        let targets = targets_of_sets_after(vec![set(1, 10)], vec![(planned_set(1), values)])[&0];

        assert_eq!(targets.entered, domain::RecordTargets::default());
        assert_eq!(targets.saved.reps, Some(domain::Reps::new(11).unwrap()));
    }

    #[test]
    fn test_no_target_without_a_recent_record() {
        let html = render_training_session(1, web_app::Settings::default(), || {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Squat")])
                .with_training_sessions(vec![earlier_session(1, 0, vec![planned_set(1)])])
        });

        assert!(contains(&html, "column-header"));
        assert!(!contains(&html, "record-target"));
    }

    #[test]
    fn test_no_target_for_a_countdown() {
        let html = render_training_session(1, web_app::Settings::default(), || {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Plank")])
                .with_training_sessions(vec![
                    earlier_session(2, 10, vec![hold(30)]),
                    earlier_session(1, 0, vec![element(0, 0, 0.0, &[60])]),
                ])
        });

        assert!(contains(&html, "countdown"));
        assert!(!contains(&html, "record-target"));
    }

    #[test]
    fn test_no_target_on_a_saved_hold_of_an_exercise_with_a_reps_record() {
        let mut values = unperformed_set_field_values();
        values.time = FieldValue::new(domain::Time::new(30).unwrap());

        assert_eq!(
            targets_after(vec![set(1, 10)], hold(30), &values),
            domain::RecordTargets::default()
        );
    }
}
