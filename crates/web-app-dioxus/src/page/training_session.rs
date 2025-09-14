use std::collections::HashMap;

use dioxus::prelude::*;

use valens_domain::{
    self as domain, ExerciseService, RoutineService, SessionService, TrainingSessionService,
};
use valens_web_app::{self as web_app, OngoingTrainingSessionService, SettingsService};

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE, NOTIFICATIONS, Route, WEB_APP_SERVICE,
    component::{
        component::{Timer, TimerService},
        element::{
            Block, CenteredBlock, ErrorMessage, FloatingActionButton, Icon, LoadingPage,
            NoConnection, Title,
        },
        form::{Field, FieldValue, FieldValueState, InputField},
    },
    eh, ensure_session, signal_changed_data,
};

#[component]
pub fn TrainingSession(id: domain::TrainingSessionID) -> Element {
    ensure_session!();

    let training_session = use_resource(move || async move {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_training_session(id).await
    });
    let memorized_training_session = use_memo(move || {
        training_session
            .read()
            .as_ref()
            .and_then(|e| e.as_ref().ok())
            .and_then(std::clone::Clone::clone)
    });
    let routine = use_resource(move || async move {
        let _ = DATA_CHANGED.read();
        if let Some(s) = memorized_training_session() {
            Some(DOMAIN_SERVICE.read().get_routine(s.routine_id).await)
        } else {
            None
        }
    });
    let exercises = use_resource(move || async move {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_exercises().await
    });

    let mut edit = use_signal(|| false);

    let mut progress = use_store(|| Progress::new(id));
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
                            < memorized_training_session
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
        if let Some(training_session) = memorized_training_session() {
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

    use_effect(move || {
        let element_idx = progress.read().element_idx;
        if let Some(training_session) = memorized_training_session() {
            if let Some(element) = training_session.elements.get(element_idx) {
                match element {
                    domain::TrainingSessionElement::Set {
                        target_time,
                        automatic,
                        ..
                    } => {
                        if let Some(target_time) = target_time {
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
                                        modify_training_session_elements(
                                            training_session.clone(),
                                            field_values,
                                        )
                                        .await;
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
                        }
                    }
                }
            }
        }
    });

    match (
        training_session.read().as_ref(),
        routine.read().as_ref(),
        exercises.read().as_ref(),
        settings.read().as_ref(),
    ) {
        (
            Some(Ok(Some(training_session))),
            Some(routine),
            Some(Ok(exercises)),
            Some(Ok(settings)),
        ) => rsx! {
            Title { title: "{training_session.date}" }
            if let Some(Ok(Some(routine))) = routine {
                Block {
                    Link {
                        to: Route::Routine { id: routine.id },
                        Title {
                            class: "has-text-link",
                            title: "{routine.name}"
                        }
                    }
                }
            }
            if edit() {
                {view_form(field_values, progress, training_session, exercises, *settings)},
            } else {
                {view_list(training_session, exercises, *settings)},
            }
            Notes {
                id: training_session.id,
                notes: training_session.notes.clone(),
                edit,
            },
            FloatingActionButton {
                icon: (if edit() { "eye" } else { "edit" }).to_string(),
                onclick: eh!(mut edit; {
                    edit.toggle();
                    if edit() && !progress.read().is_active() {
                        progress.write().set_element_idx(0);
                    }
                }),
            }
        },
        (Some(Ok(None)), _, _, _) => rsx! {
            ErrorMessage { message: "Training session not found" }
        },
        (Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))), _, _, _) => {
            rsx! { NoConnection {  } {} }
        }
        (Some(Err(err)), _, _, _) | (_, _, Some(Err(err)), _) => {
            rsx! { ErrorMessage { message: err } }
        }
        (_, _, _, Some(Err(err))) => {
            rsx! { ErrorMessage { message: err } }
        }
        (None, _, _, _) | (_, None, _, _) | (_, _, None, _) | (_, _, _, None) => {
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
    training_session: &domain::TrainingSession,
    exercises: &[domain::Exercise],
    settings: web_app::Settings,
) -> Element {
    let mut element_idx = 0;
    let sections = training_session.compute_sections();
    let rows = sections.iter().map(| section| {
        let mut exercise_ids = section.exercise_ids();
        exercise_ids.dedup();
        let len = exercise_ids.len();
        let exercise_names = exercise_ids.into_iter().enumerate().map(|(i, id)| {
            let name = exercise_name(id, exercises);
            rsx! {
                tr {
                    td {
                        class: "has-text-centered has-text-weight-bold",
                        class: if i == 0 { "pt-2" },
                        class: if i == len - 1 { "pb-1" },
                        colspan: 4,
                        Link {
                            class: "px-1",
                            to: Route::Exercise { id },
                            "{name}"
                        }
                        a {
                            class: "px-1 is-link",
                            onclick: |_| {},
                            // TODO: Open options dialog:
                            // - Replace exercise
                            // - Prefer exercise
                            // - Defer exercise
                            // - Add set
                            // - Add same exercise
                            // - Add other exercise
                            // - Remove set
                            // - Remove exercise
                            Icon { name: "ellipsis-vertical" }
                        }
                    }
                }
            }
        });
        let sets = section.elements().iter().map(|element| {
            let set = match element {
                domain::TrainingSessionElement::Set { target_reps, target_time, .. } => {
                    let set_field_values = &field_values.read()[&element_idx];
                    if set_field_values.is_empty() && !set_field_values.changed() && target_reps.is_none() && target_time.is_some() {
                        if let Some(target_time) = target_time {
                            rsx! {
                                tr {
                                    td {
                                        class: "p-1",
                                        colspan: 4,
                                        div {
                                            class: "notification is-link has-text-centered has-text-weight-bold",
                                            class: if progress.read().element_idx != element_idx { "is-semitransparent" },
                                            if progress.read().element_idx == element_idx {
                                                Timer { timer_service: progress.timer_service() }
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
                                            class: "button is-link is-outlined is-small",
                                            onclick: eh!(training_session, target_time; {
                                                progress.write().set_element_idx(element_idx + 1);
                                                progress.timer_service().write().unset();
                                                if let Some(set_field_values) = field_values.write().get_mut(&element_idx) {
                                                    set_field_values.time.validated = Ok(target_time);
                                                }
                                                modify_training_session_elements(training_session.clone(), field_values)
                                            }),
                                            Icon { name: "check" }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    } else {
                        rsx! {
                            tr {
                                class: if progress.read().element_idx == element_idx { "has-background-auto-text-95" },
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
                                    if set_field_values.valid() {
                                        button {
                                            class: "button is-small",
                                            class: if set_field_values.has_valid_changes() { "is-link is-outlined" } else if !set_field_values.is_empty() { "is-ghost" },
                                            onclick: eh!(training_session, field_values; {
                                                progress.write().set_element_idx(element_idx + 1);
                                                modify_training_session_elements(training_session.clone(), field_values)
                                            }),
                                            Icon { name: "check" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                domain::TrainingSessionElement::Rest { target_time, .. } => {
                    rsx! {
                        tr {
                            if progress.read().element_idx == element_idx {
                                td {
                                    class: "p-1",
                                    colspan: 4,
                                    div {
                                        class: "notification is-success has-text-centered has-text-weight-bold",
                                        if target_time.is_some() {
                                            Timer { timer_service: progress.timer_service() }
                                        }
                                    }
                                }
                                td {
                                    class: "p-1",
                                    style: "vertical-align: middle",
                                    button {
                                        class: "button is-success is-outlined is-small",
                                        onclick: move |_| {
                                            progress.write().set_element_idx(element_idx + 1);
                                            progress.timer_service().write().unset();
                                        },
                                        Icon { name: "check" }
                                    }
                                }
                            } else {
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
    }
}

#[component]
fn Notes(
    id: domain::TrainingSessionID,
    notes: ReadSignal<String>,
    edit: ReadSignal<bool>,
) -> Element {
    let mut field_value = use_memo(move || FieldValue::new(notes.read().clone()));
    if edit() {
        rsx! {
            Block {
                class: "px-2",
                Field {
                    label: "Notes",
                    textarea {
                        class: "textarea",
                        oninput: {
                            move |event| {
                                field_value.write().input = event.value();
                            }
                        },
                        {notes},
                    }
                }
                button {
                    class: "button",
                    class: if field_value.read().changed() { "is-link is-outlined" },
                    disabled: if !field_value.read().changed() { true },
                    onclick: move |_| { modify_training_session(id, Some(field_value.read().input.clone()), None) },
                    "Save"
                }
            }
        }
    } else {
        rsx! {
            if !notes.read().is_empty() {
                CenteredBlock {
                    Title { title: "Notes".to_string() },
                    p {
                        {notes.read().clone()}
                    }
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
        let mut exercise_ids = section.exercise_ids();
        exercise_ids.dedup();
        let len = exercise_ids.len();
        let exercise_names = exercise_ids.into_iter().enumerate().map(|(i, id)| {
            let name = exercise_name(id, exercises);
            rsx! {
                tr {
                    td {
                        class: "has-text-centered has-text-weight-bold",
                        class: if i == 0 { "pt-2" },
                        class: if i == len - 1 { "pb-1" },
                        colspan: 4,
                        Link {
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
                    domain::TrainingSessionElement::Set { reps, time, weight, rpe, .. } => {
                        rsx! {
                            tr {
                                if reps.is_none() && (time.is_none() || !settings.show_tut) && weight.is_none() && (rpe.is_none() || !settings.show_rpe) {
                                    td {
                                        class: "px-2 has-text-centered",
                                        colspan: 4,
                                        "–"
                                    }
                                } else {
                                    td {
                                        class: "px-2 has-text-right",
                                        if let Some(reps) = reps {
                                            if *reps > domain::Reps::default() {
                                                "{reps} ✕"
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

fn exercise_name(id: domain::ExerciseID, exercises: &[domain::Exercise]) -> String {
    exercises
        .iter()
        .find(|exercise| exercise.id == id)
        .map(|exercise| exercise.name.to_string())
        .unwrap_or(format!("Exercise#{}", id.as_u128()))
}

async fn modify_training_session_elements(
    mut training_session: domain::TrainingSession,
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
    modify_training_session(training_session.id, None, Some(training_session.elements)).await;
}

async fn modify_training_session(
    id: domain::TrainingSessionID,
    notes: Option<String>,
    elements: Option<Vec<valens_domain::TrainingSessionElement>>,
) {
    match DOMAIN_SERVICE
        .read()
        .modify_training_session(id, notes, elements)
        .await
    {
        Ok(_) => {
            signal_changed_data();
        }
        Err(err) => {
            debug!("ERROR");
            NOTIFICATIONS
                .write()
                .push(format!("Failed to modify training session: {err}"));
        }
    };
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
            timer_service: TimerService::new(),
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
            element_start_time: value.start_time,
            timer_service: TimerService::new(),
        }
    }
}

impl From<Progress> for web_app::OngoingTrainingSession {
    fn from(value: Progress) -> Self {
        web_app::OngoingTrainingSession {
            training_session_id: value.training_session_id.as_u128(),
            start_time: value.start_time,
            element_idx: value.element_idx,
            element_start_time: value.start_time,
            timer_state: value.timer_service.into(),
        }
    }
}
