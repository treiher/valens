use std::collections::{BTreeSet, HashSet};

use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use chrono::{Duration, Local};
use dioxus::prelude::*;
use log::error;

use valens_domain::{self as domain, ExerciseService, SessionService, TrainingSessionService};

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE, NOTIFICATIONS, Route,
    component::{
        element::{
            DeleteConfirmationDialog, Dialog, Error, FloatingActionButton, Icon, LoadingPage,
            MenuOption, NoConnection, OptionsMenu, SearchBox, Table, Title,
        },
        form::{FieldValue, FieldValueState, InputField},
    },
    ensure_session, signal_changed_data,
};

#[component]
pub fn Exercises(add: bool, filter: String) -> Element {
    ensure_session!();

    let exercises = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_exercises().await
    });
    let training_sessions = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_training_sessions().await
    });
    let mut dialog = use_signal(|| ExerciseDialog::None);

    let exercise_filter = domain::ExerciseFilter::try_from(
        ExerciseFilter::try_from_base64(&filter).unwrap_or_default(),
    )
    .unwrap_or_default();
    let name = exercise_filter.name.clone();
    let filter_string = filter.clone();
    let show_add_dialog = move || {
        let filter_string = filter_string.clone();
        async move {
            *dialog.write() = ExerciseDialog::Add {
                name: FieldValue {
                    input: name.clone(),
                    validated: DOMAIN_SERVICE
                        .read()
                        .validate_exercise_name(&name, domain::ExerciseID::nil())
                        .await
                        .map_err(|err| err.to_string()),
                    orig: name.clone(),
                },
            };
            navigator().replace(Route::Exercises {
                add: true,
                filter: filter_string,
            });
        }
    };

    let show_add_dialog_for_future = show_add_dialog.clone();
    use_future(move || {
        let show_add_dialog = show_add_dialog_for_future.clone();
        async move {
            if add {
                show_add_dialog().await;
            }
        }
    });

    let show_add_dialog_for_fab = show_add_dialog.clone();
    match (&*exercises.read(), &*training_sessions.read()) {
        (Some(Ok(exercises)), Some(Ok(training_sessions))) => {
            rsx! {
                {view_search_box(&exercise_filter)},
                {view_list(exercises, training_sessions, &exercise_filter, dialog)}
                {view_dialog(dialog, &filter)}
                FloatingActionButton {
                    icon: "plus".to_string(),
                    onclick: move |_| {
                        let show_add_dialog = show_add_dialog_for_fab.clone();
                        show_add_dialog()
                    },
                }
            }
        }
        (Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))), _) => {
            rsx! { NoConnection {  } {} }
        }
        (Some(Err(err)), _) | (_, Some(Err(err))) => {
            rsx! { Error { message: err } }
        }
        (None, _) | (_, None) => rsx! { LoadingPage {} },
    }
}

fn view_search_box(exercise_filter: &domain::ExerciseFilter) -> Element {
    let name = exercise_filter.name.clone();
    let exercise_filter = exercise_filter.clone();
    rsx! {
        div {
            class: "px-4",
            SearchBox {
                search_term: name,
                oninput: move |event: FormEvent| {
                    let mut exercise_filter = exercise_filter.clone();
                    exercise_filter.name = event.value();
                    let filter_string = ExerciseFilter::from(exercise_filter).to_base64();
                    let filter_string = filter_string.clone();
                    navigator().replace(Route::Exercises {
                        add: false,
                        filter: filter_string,
                    });
                }
            }
        }
    }
}

fn view_list(
    exercises: &[domain::Exercise],
    training_sessions: &[domain::TrainingSession],
    exercise_filter: &domain::ExerciseFilter,
    mut dialog: Signal<ExerciseDialog>,
) -> Element {
    const CURRENT_EXERCISE_CUTOFF_DAYS: i64 = 31;

    let cutoff = Local::now().date_naive() - Duration::days(CURRENT_EXERCISE_CUTOFF_DAYS);

    let current_exercise_ids = training_sessions
        .iter()
        .filter(|session| session.date >= cutoff)
        .flat_map(domain::TrainingSession::exercises)
        .collect::<BTreeSet<_>>();

    let previous_exercise_ids = training_sessions
        .iter()
        .filter(|session| session.date < cutoff)
        .flat_map(domain::TrainingSession::exercises)
        .collect::<BTreeSet<_>>();

    let exercises = exercise_filter.exercises(exercises.iter());

    let mut current_exercises = exercises
        .clone()
        .into_iter()
        .filter(|e| current_exercise_ids.contains(&e.id) || !previous_exercise_ids.contains(&e.id))
        .cloned()
        .collect::<Vec<_>>();
    current_exercises.sort_by(|a, b| a.name.cmp(&b.name));

    let mut previous_exercises = exercises
        .clone()
        .into_iter()
        .filter(|e| !current_exercise_ids.contains(&e.id) && previous_exercise_ids.contains(&e.id))
        .cloned()
        .collect::<Vec<_>>();
    previous_exercises.sort_by(|a, b| a.name.cmp(&b.name));

    let current_exercises_body = current_exercises
        .into_iter()
        .map(|e| {
            vec![
                rsx! {
                    Link {
                        to: Route::Exercise { id: e.id },
                        "{e.name}"
                    }
                },
                rsx! {
                    div {
                        class: "has-text-right",
                        a {
                            class: "mx-2",
                            onclick: move |_| { *dialog.write() = ExerciseDialog::Options(e.clone()); },
                            Icon { name: "ellipsis-vertical"}
                        }
                    }
                },
            ]
        })
        .collect::<Vec<_>>();

    let previous_exercises_body = previous_exercises
        .into_iter()
        .map(|e| {
            vec![
                rsx! {
                    Link {
                        to: Route::Exercise { id: e.id },
                        "{e.name}"
                    }
                },
                rsx! {
                    div {
                        class: "has-text-right",
                        a {
                            class: "mx-2",
                            onclick: move |_| { *dialog.write() = ExerciseDialog::Options(e.clone()); },
                            Icon { name: "ellipsis-vertical"}
                        }
                    }
                },
            ]
        })
        .collect::<Vec<_>>();

    let catalog_exercises_body = exercise_filter
        .catalog()
        .values()
        .map(|e| {
            let e = (*e).clone();
            vec![
                rsx! {
                    Link {
                        to: Route::Catalog { name: e.name.to_string() },
                        "{e.name}"
                    }
                },
                rsx! {
                    if exercises.iter().all(|x| x.name != e.name) {
                        div {
                            class: "has-text-right",
                            a {
                                class: "mx-2",
                                onclick: move |_| {
                                    let name = e.name.clone();
                                    let mut muscles = vec![];
                                    for (m, s) in e.muscles {
                                        muscles.push(domain::ExerciseMuscle {
                                            muscle_id: *m,
                                            stimulus: *s,
                                        });
                                    }
                                    async move {
                                            match DOMAIN_SERVICE
                                                .read()
                                                .create_exercise(name, muscles)
                                                .await
                                            {
                                                Ok(_) => {
                                                    signal_changed_data();
                                                }
                                                Err(err) => {
                                                    NOTIFICATIONS
                                                        .write()
                                                        .push(format!("Failed to add exercise from catalog: {err}"));
                                                }
                                            }
                                    }
                                },
                                Icon { name: "plus"}
                            }
                        }
                    }
                },
            ]
        })
        .collect::<Vec<_>>();

    rsx! {
        Table { body: current_exercises_body }
        if !previous_exercises_body.is_empty() {
            Title { title: "Previous exercises" }
            Table { body: previous_exercises_body }
        }
        if !catalog_exercises_body.is_empty() {
            Title { title: "Catalog exercises" }
            Table { body: catalog_exercises_body }
        }
    }
}

fn view_dialog(mut dialog: Signal<ExerciseDialog>, filter: &str) -> Element {
    let mut is_loading = use_signal(|| false);

    macro_rules! close_dialog {
        ($dialog:expr, $filter:expr) => {{
            *$dialog.write() = ExerciseDialog::None;
            navigator().replace(Route::Exercises {
                add: false,
                filter: $filter,
            });
        }};
    }

    macro_rules! is_loading {
        ($block:expr) => {
            *is_loading.write() = true;
            $block;
            *is_loading.write() = false;
        };
    }

    let filter_for_save = filter.to_string();
    let save = move |_| {
        let filter = filter_for_save.to_string();
        async move {
            let mut saved = false;
            is_loading! {
                if let ExerciseDialog::Add { name } | ExerciseDialog::Copy { name, .. } | ExerciseDialog::Rename { name, .. } = &*dialog.read() {
                    if let Ok(name) = name.validated.clone() {
                        match &*dialog.read() {
                            ExerciseDialog::Add { .. } => {
                                match DOMAIN_SERVICE
                                    .read()
                                    .create_exercise(name, vec![])
                                    .await
                                {
                                    Ok(_) => {
                                        saved = true;
                                        signal_changed_data();
                                    }
                                    Err(err) => {
                                        NOTIFICATIONS
                                            .write()
                                            .push(format!("Failed to add exercise: {err}"));
                                    }
                                }
                            }
                            ExerciseDialog::Copy { exercise_id, .. } => {
                                match DOMAIN_SERVICE.read().get_exercises().await {
                                    Ok(exercises) => {
                                        let muscles = exercises.iter().find(|e| e.id == *exercise_id).map(|exercise| {
                                            exercise
                                                .muscles
                                                .clone()
                                        }).unwrap_or_default();
                                        match DOMAIN_SERVICE
                                            .read()
                                            .create_exercise(name, muscles)
                                            .await
                                        {
                                            Ok(_) => {
                                                saved = true;
                                                signal_changed_data();
                                            }
                                            Err(err) => {
                                                NOTIFICATIONS
                                                    .write()
                                                    .push(format!("Failed to copy exercise: {err}"));
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        NOTIFICATIONS
                                            .write()
                                            .push(format!("Failed to copy exercise: {err}"));
                                        }
                                }
                            }
                            ExerciseDialog::Rename { exercise_id, .. } => {
                                match DOMAIN_SERVICE
                                    .read()
                                    .replace_exercise(domain::Exercise {
                                        id: *exercise_id,
                                        name,
                                        muscles: vec![], // TODO: shouldn't reset muscles
                                    })
                                    .await
                                {
                                    Ok(_) => {
                                        saved = true;
                                        signal_changed_data();
                                    }
                                    Err(err) => {
                                        NOTIFICATIONS
                                            .write()
                                            .push(format!("Failed to rename exercise: {err}"));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            if saved {
                close_dialog!(dialog, filter);
            }
        }
    };
    let filter_for_delete = filter.to_string();
    let delete = move |_| {
        let filter = filter_for_delete.to_string();
        async move {
            let mut deleted = false;
            is_loading! {
                if let ExerciseDialog::Delete(exercise) = &*dialog.read() {
                    match DOMAIN_SERVICE.read().delete_exercise(exercise.id).await {
                        Ok(_) => {
                            deleted = true;
                            signal_changed_data();
                        },
                        Err(err) => NOTIFICATIONS.write().push(format!("Failed to delete training session: {err}"))
                    }
                }
            }
            if deleted {
                close_dialog!(dialog, filter);
            }
        }
    };
    macro_rules! close {
        ($filter:expr) => {{
            let filter = $filter.to_string();
            move |_| {
                let filter = filter.to_string();
                *dialog.write() = ExerciseDialog::None;
                navigator().replace(Route::Exercises { add: false, filter });
            }
        }};
    }

    match &*dialog.read() {
        ExerciseDialog::None => rsx! {},
        ExerciseDialog::Options(exercise) => {
            let exercise = exercise.clone();
            let exercise_name_copy = exercise.name.clone();
            let exercise_name_edit = exercise.name.clone();
            rsx! {
                OptionsMenu {
                    options: vec![
                        rsx! {
                            MenuOption {
                                icon: "copy".to_string(),
                                text: "Copy exercise".to_string(),
                                onclick: move |_| {
                                    let exercise_name = exercise_name_copy.clone();
                                    async move {
                                        *dialog.write() = ExerciseDialog::Copy {
                                            name: FieldValue {
                                                input: exercise_name.to_string(),
                                                validated: DOMAIN_SERVICE.read().validate_exercise_name(&exercise_name.to_string(), domain::ExerciseID::nil()).await.map_err(|err| err.to_string()),
                                                orig: exercise_name.to_string(),
                                            },
                                            exercise_id: exercise.id,
                                        };
                                    }
                                }
                            },
                            MenuOption {
                                icon: "edit".to_string(),
                                text: "Rename exercise".to_string(),
                                onclick: move |_| {
                                    let exercise_name = exercise_name_edit.clone();
                                    *dialog.write() = ExerciseDialog::Rename {
                                        name: FieldValue::new(exercise_name),
                                        exercise_id: exercise.id,
                                    };
                                }
                            },
                            MenuOption {
                                icon: "times".to_string(),
                                text: "Delete exercise".to_string(),
                                onclick: move |_| { *dialog.write() = ExerciseDialog::Delete(exercise.clone()); }
                            },
                        },
                    ],
                    close_event: close!(filter)
                }
            }
        }
        ExerciseDialog::Add { name }
        | ExerciseDialog::Copy { name, .. }
        | ExerciseDialog::Rename { name, .. } => rsx! {
            Dialog {
                title: rsx! { match &*dialog.read() { ExerciseDialog::Add { .. } => { "Add exercise" }, ExerciseDialog::Copy { .. } =>  { "Copy exercise" }, ExerciseDialog::Rename { .. } =>  { "Rename exercise" }, _ => "" } },
                content: rsx! {
                    InputField {
                        label: "Name".to_string(),
                        value: name.input.clone(),
                        error: if let Err(err) = &name.validated { err.clone() },
                        has_changed: name.changed(),
                        oninput: move |event: FormEvent| {
                            async move {
                                match &mut *dialog.write() {
                                    ExerciseDialog::Add { name, .. } | ExerciseDialog::Copy { name, .. } => {
                                        name.input = event.value();
                                        name.validated = DOMAIN_SERVICE.read().validate_exercise_name(&name.input, domain::ExerciseID::nil()).await.map_err(|err| err.to_string());
                                    }
                                    ExerciseDialog::Rename { name, exercise_id } => {
                                        name.input = event.value();
                                        name.validated = DOMAIN_SERVICE.read().validate_exercise_name(&name.input, *exercise_id).await.map_err(|err| err.to_string());
                                    }
                                    _ => { }
                                }
                            }
                        }
                    }
                    div {
                        class: "field is-grouped is-grouped-centered",
                        div {
                            class: "control",
                            onclick: close!(filter),
                            button { class: "button is-light is-soft", "Cancel" }
                        }
                        div {
                            class: "control",
                            onclick: save,
                            button {
                                class: "button is-primary",
                                class: if is_loading() { "is-loading" },
                                disabled: !name.valid(),
                                "Save"
                            }
                        }
                    }
                },
                close_event: close!(filter),
            }
        },
        ExerciseDialog::Delete(exercise) => rsx! {
            DeleteConfirmationDialog {
                element_type: "exercise".to_string(),
                element_name: rsx! { "{exercise.name}" },
                delete_event: delete.clone(),
                cancel_event: close!(filter),
                is_loading: is_loading(),
            }
        },
    }
}

enum ExerciseDialog {
    None,
    Options(domain::Exercise),
    Add {
        name: FieldValue<domain::Name>,
    },
    Copy {
        name: FieldValue<domain::Name>,
        exercise_id: domain::ExerciseID,
    },
    Rename {
        name: FieldValue<domain::Name>,
        exercise_id: domain::ExerciseID,
    },
    Delete(domain::Exercise),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone, PartialEq)]
struct ExerciseFilter {
    pub name: String,
    pub muscles: HashSet<u8>,
    pub force: HashSet<u8>,
    pub mechanic: HashSet<u8>,
    pub laterality: HashSet<u8>,
    pub assistance: HashSet<u8>,
    pub equipment: HashSet<u8>,
    pub category: HashSet<u8>,
}

impl ExerciseFilter {
    fn to_base64(&self) -> String {
        match postcard::to_allocvec(self) {
            Ok(bytes) => URL_SAFE.encode(bytes),
            Err(err) => {
                error!("failed to encode exercise filter: {err}");
                String::new()
            }
        }
    }

    fn try_from_base64(value: &str) -> Result<Self, anyhow::Error> {
        if value.is_empty() {
            return Ok(Self::default());
        }
        match URL_SAFE.decode(value) {
            Ok(bytes) => match postcard::from_bytes(&bytes) {
                Ok(exercise_filter) => Ok(exercise_filter),
                Err(err) => {
                    error!("failed to decode exercise filter: {err}");
                    Err(err.into())
                }
            },
            Err(err) => {
                error!("failed to decode base64-encoded exercise filter: {err}");
                Err(err.into())
            }
        }
    }
}

impl From<domain::ExerciseFilter> for ExerciseFilter {
    fn from(value: domain::ExerciseFilter) -> Self {
        Self {
            name: value.name.clone(),
            muscles: value.muscles.iter().map(|v| *v as u8).collect(),
            force: value.force.iter().map(|v| *v as u8).collect(),
            mechanic: value.mechanic.iter().map(|v| *v as u8).collect(),
            laterality: value.laterality.iter().map(|v| *v as u8).collect(),
            assistance: value.assistance.iter().map(|v| *v as u8).collect(),
            equipment: value.equipment.iter().map(|v| *v as u8).collect(),
            category: value.category.iter().map(|v| *v as u8).collect(),
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct ExerciseFilterError;

impl TryFrom<ExerciseFilter> for domain::ExerciseFilter {
    type Error = ExerciseFilterError;

    fn try_from(value: ExerciseFilter) -> Result<Self, Self::Error> {
        Ok(domain::ExerciseFilter {
            name: value.name,
            muscles: value
                .muscles
                .into_iter()
                .filter_map(|v| domain::MuscleID::try_from(v).ok())
                .collect::<HashSet<_>>(),
            force: value
                .force
                .into_iter()
                .filter_map(|v| match v {
                    x if x == domain::Force::Push as u8 => Some(domain::Force::Push),
                    x if x == domain::Force::Pull as u8 => Some(domain::Force::Pull),
                    x if x == domain::Force::Static as u8 => Some(domain::Force::Static),
                    _ => None,
                })
                .collect::<HashSet<_>>(),
            mechanic: value
                .mechanic
                .into_iter()
                .filter_map(|v| match v {
                    x if x == domain::Mechanic::Compound as u8 => Some(domain::Mechanic::Compound),
                    x if x == domain::Mechanic::Isolation as u8 => {
                        Some(domain::Mechanic::Isolation)
                    }
                    _ => None,
                })
                .collect::<HashSet<_>>(),
            laterality: value
                .laterality
                .into_iter()
                .filter_map(|v| match v {
                    x if x == domain::Laterality::Bilateral as u8 => {
                        Some(domain::Laterality::Bilateral)
                    }
                    x if x == domain::Laterality::Unilateral as u8 => {
                        Some(domain::Laterality::Unilateral)
                    }
                    _ => None,
                })
                .collect::<HashSet<_>>(),
            assistance: value
                .assistance
                .into_iter()
                .filter_map(|v| match v {
                    x if x == domain::Assistance::Unassisted as u8 => {
                        Some(domain::Assistance::Unassisted)
                    }
                    x if x == domain::Assistance::Assisted as u8 => {
                        Some(domain::Assistance::Assisted)
                    }
                    _ => None,
                })
                .collect::<HashSet<_>>(),
            equipment: value
                .equipment
                .into_iter()
                .filter_map(|v| match v {
                    x if x == domain::Equipment::None as u8 => Some(domain::Equipment::None),
                    x if x == domain::Equipment::Barbell as u8 => Some(domain::Equipment::Barbell),
                    x if x == domain::Equipment::Box as u8 => Some(domain::Equipment::Box),
                    x if x == domain::Equipment::Cable as u8 => Some(domain::Equipment::Cable),
                    x if x == domain::Equipment::Dumbbell as u8 => {
                        Some(domain::Equipment::Dumbbell)
                    }
                    x if x == domain::Equipment::ExerciseBall as u8 => {
                        Some(domain::Equipment::ExerciseBall)
                    }
                    x if x == domain::Equipment::GymnasticRings as u8 => {
                        Some(domain::Equipment::GymnasticRings)
                    }
                    x if x == domain::Equipment::Kettlebell as u8 => {
                        Some(domain::Equipment::Kettlebell)
                    }
                    x if x == domain::Equipment::Machine as u8 => Some(domain::Equipment::Machine),
                    x if x == domain::Equipment::ParallelBars as u8 => {
                        Some(domain::Equipment::ParallelBars)
                    }
                    x if x == domain::Equipment::PullUpBar as u8 => {
                        Some(domain::Equipment::PullUpBar)
                    }
                    x if x == domain::Equipment::ResistanceBand as u8 => {
                        Some(domain::Equipment::ResistanceBand)
                    }
                    x if x == domain::Equipment::Sliders as u8 => Some(domain::Equipment::Sliders),
                    x if x == domain::Equipment::TrapBar as u8 => Some(domain::Equipment::TrapBar),
                    _ => None,
                })
                .collect::<HashSet<_>>(),
            category: value
                .category
                .into_iter()
                .filter_map(|v| match v {
                    x if x == domain::Category::Strength as u8 => Some(domain::Category::Strength),
                    x if x == domain::Category::Plyometrics as u8 => {
                        Some(domain::Category::Plyometrics)
                    }
                    _ => None,
                })
                .collect::<HashSet<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo() {
        let exercise_filter = domain::ExerciseFilter {
            name: "Exercise Name".to_string(),
            muscles: [domain::MuscleID::Lats, domain::MuscleID::Traps].into(),
            force: [domain::Force::Pull].into(),
            mechanic: [domain::Mechanic::Isolation].into(),
            laterality: [domain::Laterality::Unilateral].into(),
            assistance: [domain::Assistance::Assisted].into(),
            equipment: [
                domain::Equipment::GymnasticRings,
                domain::Equipment::ResistanceBand,
            ]
            .into(),
            category: [domain::Category::Plyometrics].into(),
        };
        let dto = ExerciseFilter::from(exercise_filter.clone());
        assert_eq!(
            domain::ExerciseFilter::try_from(
                ExerciseFilter::try_from_base64(&dto.to_base64()).unwrap()
            ),
            Ok(exercise_filter)
        );
    }
}
