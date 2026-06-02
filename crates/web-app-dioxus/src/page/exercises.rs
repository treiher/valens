use std::collections::{BTreeSet, HashSet};

use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use chrono::{Duration, Local};
use dioxus::prelude::*;
use log::{error, warn};

use valens_domain::{self as domain, ExerciseService, Property};

use crate::{
    DOMAIN_SERVICE, Route,
    cache::{Cache, CacheState},
    eh,
    notification::notify,
    routing::NavigatorScrollExt,
    ui::{
        element::{
            Block, DeleteConfirmationDialog, Dialog, ErrorMessage, FloatingActionButton, Icon,
            ItemOptionsButton, LoadingPage, MenuOption, NoConnection, OptionsMenu, SaveDialog,
            SearchBox, Table, Title,
        },
        form::{FieldValue, FieldValueState, InputField, MultiToggle, MultiToggleTags},
    },
};

macro_rules! show_add_dialog {
    ($dialog:ident, $name:ident, $filter_string:ident, $exercises_page:ident) => {{
        let name = $name.clone();
        let filter_string = $filter_string.clone();
        move || async move {
            let validated_name = DOMAIN_SERVICE()
                .validate_exercise_name(&name, domain::ExerciseID::nil())
                .await
                .map_err(|err| err.to_string());
            *$dialog.write() = ExerciseDialog::Add {
                name: FieldValue {
                    input: name.clone(),
                    validated: validated_name,
                    orig: name.clone(),
                },
            };
            if $exercises_page {
                navigator().replace_preserving_scroll(Route::Exercises {
                    add: true,
                    filter: filter_string,
                });
            }
        }
    }
    ()};
}

#[component]
pub fn Exercises(add: bool, filter: String) -> Element {
    rsx! {
        ExerciseList {
            add,
            filter,
            on_exercise_click: move |(_, id)| { navigator().push(Route::Exercise { id }); },
            on_catalog_click: move |(_, name)| { navigator().push(Route::Catalog { name }); },
            exercises_page: true,
        }
    }
}

#[component]
pub fn ExerciseList(
    add: bool,
    filter: String,
    on_exercise_click: EventHandler<(MouseEvent, domain::ExerciseID)>,
    on_catalog_click: EventHandler<(MouseEvent, String)>,
    #[props(default)] exercises_page: bool,
) -> Element {
    let cache = consume_context::<Cache>();
    let mut dialog = use_signal(|| ExerciseDialog::None);
    let filter_dialog_shown = use_signal(|| false);

    let exercise_filter = use_signal(|| {
        domain::ExerciseFilter::try_from(ExerciseFilter::from_base64(&filter)).unwrap_or_default()
    });
    let name = exercise_filter.read().name.clone();

    use_future({
        let name = name.clone();
        let filter = filter.clone();
        move || {
            let name = name.clone();
            let filter = filter.clone();
            async move {
                if add {
                    show_add_dialog!(dialog, name, filter, exercises_page).await;
                }
            }
        }
    });

    match (&*cache.exercises.read(), &*cache.training_sessions.read()) {
        (CacheState::Ready(exercises), CacheState::Ready(training_sessions)) => {
            let filtered_exercises = exercise_filter.read().exercises(exercises.iter());
            rsx! {
                {view_search_box(exercise_filter, dialog, filter_dialog_shown, &filter, exercises_page)},
                {view_list(&filtered_exercises, training_sessions, exercise_filter, dialog, on_exercise_click, on_catalog_click)}
                {view_dialog(dialog, if exercises_page { Some(Route::Exercises { add: false, filter: filter.clone() }) } else { None })}
                {view_filter_dialog(exercise_filter, filter_dialog_shown, filtered_exercises.len())}
                if exercises_page {
                    FloatingActionButton {
                        icon: "plus".to_string(),
                        on_click: move |_| {
                            show_add_dialog!(dialog, name, filter, exercises_page)
                        },
                    }
                }
            }
        }
        (CacheState::Error(domain::ReadError::Storage(domain::StorageError::NoConnection)), _) => {
            rsx! { NoConnection {} }
        }
        (CacheState::Error(err), _) | (_, CacheState::Error(err)) => {
            rsx! { ErrorMessage { message: err } }
        }
        (CacheState::Loading, _) | (_, CacheState::Loading) => rsx! { LoadingPage {} },
    }
}

macro_rules! view_filter_tags {
    ($list:ident, $toggle:ident, $exercise_filter:ident, $show_enabled_only:expr) => {{
        let filter = $exercise_filter.read().$list();
        let elements = filter
            .iter()
            .filter(|(_, enabled)| !$show_enabled_only || *enabled)
            .map(|(element, enabled)| {
                let e = *element;
                let n = (*element).name();
                rsx! {
                    span {
                        class: "tag is-hoverable",
                        class: if *enabled { "is-link" },
                        onclick: move |_| $exercise_filter.write().$toggle(e),
                        {n}
                    }
                }
            })
            .collect::<Vec<_>>();
        rsx! {
            for element in elements {
                {element}
            }
        }
    }};
}

fn view_search_box(
    mut exercise_filter: Signal<domain::ExerciseFilter>,
    mut filter_dialog: Signal<ExerciseDialog>,
    mut filter_dialog_shown: Signal<bool>,
    filter_string: &str,
    exercises_page: bool,
) -> Element {
    let filter_string = filter_string.to_string();
    let name = exercise_filter.read().name.clone();
    let muscle_tags = view_filter_tags!(muscle_list, toggle_muscle, exercise_filter, true);
    let force_tags = view_filter_tags!(force_list, toggle_force, exercise_filter, true);
    let mechanic_tags = view_filter_tags!(mechanic_list, toggle_mechanic, exercise_filter, true);
    let laterality_tags =
        view_filter_tags!(laterality_list, toggle_laterality, exercise_filter, true);
    let assistance_tags =
        view_filter_tags!(assistance_list, toggle_assistance, exercise_filter, true);
    let equipment_tags = view_filter_tags!(equipment_list, toggle_equipment, exercise_filter, true);
    let category_tags = view_filter_tags!(category_list, toggle_category, exercise_filter, true);
    rsx! {
        Block {
            div {
                class: "field is-grouped px-3",
                SearchBox {
                    search_term: &name,
                    on_input: move |event: FormEvent| {
                        exercise_filter.write().name = event.value();
                        let filter_string = ExerciseFilter::from(exercise_filter.read().clone()).to_base64();
                        let filter_string = filter_string.clone();
                        if exercises_page {
                            navigator().replace_preserving_scroll(Route::Exercises {
                                add: false,
                                filter: filter_string,
                            });
                        }
                    }
                }
                button {
                    class: "button",
                    class: if !exercise_filter.read().is_empty() { "is-link" },
                    "data-testid": "filter-exercises",
                    onclick: move |_| *filter_dialog_shown.write() = true,
                    Icon { name: "filter" }
                }
                if !exercises_page {
                    button {
                        class: "button is-link",
                        "data-testid": "create-exercise",
                        onclick: move |_| {
                            show_add_dialog!(filter_dialog, name, filter_string, exercises_page)
                        },
                        Icon { name: "plus" }
                    }
                }
            }
            div {
                class: "is-flex px-4",
                div {
                    class: "tags is-flex-wrap-nowrap is-overflow-scroll is-scrollbar-width-none",
                    {muscle_tags}
                    {force_tags}
                    {mechanic_tags}
                    {laterality_tags}
                    {assistance_tags}
                    {equipment_tags}
                    {category_tags}
                }
            }
        }
    }
}

fn view_list(
    exercises: &[&domain::Exercise],
    training_sessions: &[domain::TrainingSession],
    exercise_filter: Signal<domain::ExerciseFilter>,
    mut dialog: Signal<ExerciseDialog>,
    on_exercise_click: EventHandler<(MouseEvent, domain::ExerciseID)>,
    on_catalog_click: EventHandler<(MouseEvent, String)>,
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

    let mut current_exercises = exercises
        .iter()
        .copied()
        .filter(|e| current_exercise_ids.contains(&e.id) || !previous_exercise_ids.contains(&e.id))
        .cloned()
        .collect::<Vec<_>>();
    current_exercises.sort_by(|a, b| a.name.cmp(&b.name));

    let mut previous_exercises = exercises
        .iter()
        .copied()
        .filter(|e| !current_exercise_ids.contains(&e.id) && previous_exercise_ids.contains(&e.id))
        .cloned()
        .collect::<Vec<_>>();
    previous_exercises.sort_by(|a, b| a.name.cmp(&b.name));

    let current_exercises_body = current_exercises
        .into_iter()
        .map(|e| {
            vec![
                rsx! {
                    span {
                        class: "has-text-link",
                        onclick: move |event| on_exercise_click((event, e.id)),
                        "{e.name}"
                    }
                },
                rsx! {
                    div {
                        class: "has-text-link has-text-right",
                        ItemOptionsButton { on_click: move |_| { *dialog.write() = ExerciseDialog::Options(e.clone()); } }
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
                    span {
                        class: "has-text-link",
                        onclick: move |event| on_exercise_click((event, e.id)),
                        "{e.name}"
                    }
                },
                rsx! {
                    div {
                        class: "has-text-link has-text-right",
                        ItemOptionsButton { on_click: move |_| { *dialog.write() = ExerciseDialog::Options(e.clone()); } }
                    }
                },
            ]
        })
        .collect::<Vec<_>>();

    let catalog_exercises_body = exercise_filter.read()
        .catalog()
        .values()
        .map(|e| {
            let e = (*e).clone();
            let name = e.name.to_string();
            vec![
                rsx! {
                    span {
                        class: "has-text-link",
                        onclick: move |event| on_catalog_click((event, name.clone())),
                        "{e.name}"
                    }
                },
                rsx! {
                    if exercises.iter().all(|x| x.name != e.name) {
                        div {
                            class: "has-text-link has-text-right",
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
                                            match DOMAIN_SERVICE()
                                                .create_exercise(name, muscles)
                                                .await
                                            {
                                                Ok(_) => {
                                                    consume_context::<Cache>().refresh_exercises();
                                                }
                                                Err(err) => {
                                                    notify("Failed to add exercise from catalog", &err);
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
        if !current_exercises_body.is_empty() {
            Table { body: current_exercises_body }
        }
        if !previous_exercises_body.is_empty() {
            Title { "Previous exercises" }
            Table { body: previous_exercises_body }
        }
        if !catalog_exercises_body.is_empty() {
            Title { "Catalog exercises" }
            Table { body: catalog_exercises_body }
        }
    }
}

pub fn view_dialog(
    mut dialog: Signal<ExerciseDialog>,
    closed_dialog_route: Option<Route>,
) -> Element {
    let mut is_loading = use_signal(|| false);

    macro_rules! is_loading {
        ($block:expr) => {
            is_loading.set(true);
            $block;
            is_loading.set(false);
        };
    }

    let close_dialog = move || {
        dialog.set(ExerciseDialog::None);
        if let Some(route) = closed_dialog_route {
            navigator().replace_preserving_scroll(route);
        }
    };

    let save = {
        let close_dialog = close_dialog.clone();
        move |_| {
            let close_dialog = close_dialog.clone();
            async move {
                let mut saved = false;
                is_loading! {
                    if let ExerciseDialog::Add { name } | ExerciseDialog::Copy { name, .. } | ExerciseDialog::Rename { name, .. } = &*dialog.read()
                        && let Ok(name) = name.validated.clone() {
                            match &*dialog.read() {
                                ExerciseDialog::Add { .. } => {
                                    match DOMAIN_SERVICE()
                                        .create_exercise(name, vec![])
                                        .await
                                    {
                                        Ok(_) => {
                                            saved = true;
                                            consume_context::<Cache>().refresh_exercises();
                                        }
                                        Err(err) => {
                                            notify("Failed to add exercise", &err);
                                        }
                                    }
                                }
                                ExerciseDialog::Copy { muscles, .. } => {
                                    match DOMAIN_SERVICE()
                                        .create_exercise(name, muscles.clone())
                                        .await
                                    {
                                        Ok(_) => {
                                            saved = true;
                                            consume_context::<Cache>().refresh_exercises();
                                        }
                                        Err(err) => {
                                            notify("Failed to copy exercise", &err);
                                        }
                                    }
                                }
                                ExerciseDialog::Rename { exercise, .. } => {
                                    match DOMAIN_SERVICE()
                                        .replace_exercise(domain::Exercise {
                                            id: exercise.id,
                                            name,
                                            muscles: exercise.muscles.clone(),
                                        })
                                        .await
                                    {
                                        Ok(_) => {
                                            saved = true;
                                            consume_context::<Cache>().refresh_exercises();
                                        }
                                        Err(err) => {
                                            notify("Failed to rename exercise", &err);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                }
                if saved {
                    close_dialog();
                }
            }
        }
    };
    let delete = {
        let close_dialog = close_dialog.clone();
        move |_| {
            let close_dialog = close_dialog.clone();
            async move {
                let mut deleted = false;
                is_loading! {
                    if let ExerciseDialog::Delete(exercise) = &*dialog.read() {
                        match DOMAIN_SERVICE().delete_exercise(exercise.id).await {
                            Ok(()) => {
                                deleted = true;
                                consume_context::<Cache>().refresh_exercises();
                            },
                            Err(err) => notify("Failed to delete exercise", &err)
                        }
                    }
                }
                if deleted {
                    close_dialog();
                }
            }
        }
    };

    match &*dialog.read() {
        ExerciseDialog::None => rsx! {},
        ExerciseDialog::Options(exercise) => {
            let exercise = exercise.clone();
            let exercise_name = exercise.name.clone();
            let exercise_muscles = exercise.muscles.clone();
            rsx! {
                OptionsMenu {
                    options: vec![
                        rsx! {
                            MenuOption {
                                icon: "copy".to_string(),
                                text: "Copy exercise".to_string(),
                                "data-testid": "options-copy",
                                on_click: eh!(exercise_name, exercise_muscles; {
                                    async move {
                                        let validated_name = DOMAIN_SERVICE().validate_exercise_name(&exercise_name.to_string(), domain::ExerciseID::nil()).await.map_err(|err| err.to_string());
                                        *dialog.write() = ExerciseDialog::Copy {
                                            name: FieldValue {
                                                input: exercise_name.to_string(),
                                                validated: validated_name,
                                                orig: exercise_name.to_string(),
                                            },
                                            muscles: exercise_muscles,
                                        };
                                    }
                                })
                            },
                            MenuOption {
                                icon: "edit".to_string(),
                                text: "Rename exercise".to_string(),
                                "data-testid": "options-rename",
                                on_click: eh!(exercise; {
                                    *dialog.write() = ExerciseDialog::Rename {
                                        name: FieldValue::new(exercise.name.clone()),
                                        exercise,
                                    };
                                })
                            },
                            MenuOption {
                                icon: "tags".to_string(),
                                text: "Change properties".to_string(),
                                "data-testid": "options-properties",
                                on_click: eh!(exercise; {
                                    *dialog.write() = ExerciseDialog::ChangeProperties {
                                        exercise,
                                    };
                                })
                            },
                            MenuOption {
                                icon: "times".to_string(),
                                text: "Delete exercise".to_string(),
                                "data-testid": "options-delete",
                                on_click: move |_| { *dialog.write() = ExerciseDialog::Delete(exercise.clone()); }
                            },
                        },
                    ],
                    on_close: eh!(close_dialog; { close_dialog(); })
                }
            }
        }
        ExerciseDialog::Add { name }
        | ExerciseDialog::Copy { name, .. }
        | ExerciseDialog::Rename { name, .. } => rsx! {
            SaveDialog {
                title: rsx! { match &*dialog.read() { ExerciseDialog::Add { .. } => { "Add exercise" }, ExerciseDialog::Copy { .. } =>  { "Copy exercise" }, ExerciseDialog::Rename { .. } =>  { "Rename exercise" }, _ => "" } },
                on_close: eh!(close_dialog; { close_dialog(); }),
                on_save: save,
                is_loading: is_loading(),
                disabled: !name.valid(),
                InputField {
                    label: "Name".to_string(),
                    value: name.input.clone(),
                    error: if let Err(err) = &name.validated { err.clone() },
                    has_changed: name.changed(),
                    autofocus: true,
                    on_input: move |event: FormEvent| {
                        let input = event.value();
                        match &mut *dialog.write() {
                            ExerciseDialog::Add { name, .. }
                            | ExerciseDialog::Copy { name, .. }
                            | ExerciseDialog::Rename { name, .. } => {
                                name.input.clone_from(&input);
                            }
                            _ => {}
                        }
                        let exercise_id = {
                            match &*dialog.read() {
                                ExerciseDialog::Rename { exercise, .. } => exercise.id,
                                _ => domain::ExerciseID::nil()
                            }
                        };
                        async move {
                            // Debounce the validation to prevent unexpected input field updates
                            // caused by rapid inputs
                            gloo_timers::future::sleep(std::time::Duration::from_millis(10)).await;
                            {
                                match &*dialog.read() {
                                    ExerciseDialog::Add { name, .. } | ExerciseDialog::Copy { name, .. } | ExerciseDialog::Rename { name, .. }
                                        if name.input != input => {
                                            return;
                                        }
                                    _ => {}
                                }
                            }
                            let validated_name = DOMAIN_SERVICE().validate_exercise_name(&input, exercise_id).await.map_err(|err| err.to_string());
                            match &mut *dialog.write() {
                                ExerciseDialog::Add { name, .. } | ExerciseDialog::Copy { name, .. } | ExerciseDialog::Rename { name, .. }
                                    if name.input == input => {
                                        name.validated = validated_name;
                                    }
                                _ => {}
                            }
                        }
                    }
                }
            }
        },
        ExerciseDialog::ChangeProperties { exercise } => rsx! {
            ExercisePropertiesDialog { exercise: exercise.clone(), on_save: save, on_close: eh!(close_dialog; { close_dialog(); }) }
        },
        ExerciseDialog::Delete(exercise) => rsx! {
            DeleteConfirmationDialog {
                element_type: "exercise".to_string(),
                element_name: rsx! { "{exercise.name}" },
                on_delete: delete.clone(),
                on_cancel: eh!(close_dialog; { close_dialog(); }),
                is_loading: is_loading(),
            }
        },
    }
}

macro_rules! view_filter_section {
    ($title:expr, $list:ident, $toggle:ident, $exercise_filter:ident, $show_enabled_only:expr) => {{
        let tags = view_filter_tags!($list, $toggle, $exercise_filter, false);
        rsx! {
            Block {
                label {
                    class: "subtitle",
                    $title
                }
                div {
                    class: "container py-3",
                    div {
                        class: "tags",
                        {tags}
                    }
                }
            }
        }
    }};
}

#[component]
fn ExercisePropertiesDialog(
    exercise: domain::Exercise,
    on_save: EventHandler<MouseEvent>,
    on_close: EventHandler<()>,
) -> Element {
    let multi_toggle = use_signal(|| MultiToggle {
        states: domain::MuscleID::iter()
            .map(|m| {
                (
                    m.name().to_string(),
                    exercise
                        .muscles
                        .iter()
                        .find(|em| em.muscle_id == *m)
                        .map(|em| {
                            if em.stimulus == domain::Stimulus::PRIMARY {
                                2
                            } else {
                                1
                            }
                        })
                        .unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>(),
        num_states: 3,
    });
    let mut is_loading = use_signal(|| false);

    macro_rules! is_loading {
        ($block:expr) => {
            is_loading.set(true);
            $block;
            is_loading.set(false);
        };
    }

    let exercise_name = exercise.name.clone();

    let save = move |_| {
        let exercise_name = exercise_name.clone();
        async move {
            let muscles = multi_toggle
                .read()
                .states
                .iter()
                .enumerate()
                .filter_map(|(i, (_, value))| {
                    if *value > 0 {
                        domain::MuscleID::iter()
                            .nth(i)
                            .map(|muscle_id| domain::ExerciseMuscle {
                                muscle_id: *muscle_id,
                                stimulus: if *value == 1 {
                                    domain::Stimulus::SECONDARY
                                } else if *value == 2 {
                                    domain::Stimulus::PRIMARY
                                } else {
                                    domain::Stimulus::NONE
                                },
                            })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let mut saved = false;
            is_loading! {
                match DOMAIN_SERVICE()
                    .replace_exercise(domain::Exercise {
                        id: exercise.id,
                        name: exercise_name,
                        muscles,
                    })
                    .await
                {
                    Ok(_) => {
                        saved = true;
                        consume_context::<Cache>().refresh_exercises();
                    }
                    Err(err) => {
                        notify("Failed to change properties of exercise", &err);
                    }
                }
            }
            if saved {
                on_close(());
            }
        }
    };

    rsx! {
        SaveDialog {
            title: rsx! { "Change properties" },
            on_close: eh!(on_close; { on_close(()); }),
            on_save: save,
            is_loading: is_loading(),
            disabled: false,
            label {
                class: "subtitle",
                "Muscles ("
                    span {
                        class: "tag is-dark",
                        "Primary"
                    }
                " "
                    span {
                        class: "tag is-link",
                        "Secondary"
                    }
                ")"
            }
            div {
                class: "container py-3",
                MultiToggleTags { multi_toggle }
            }
        }
    }
}

fn view_filter_dialog(
    mut exercise_filter: Signal<domain::ExerciseFilter>,
    mut filter_dialog_shown: Signal<bool>,
    exercise_count: usize,
) -> Element {
    if !*filter_dialog_shown.read() {
        return rsx! {};
    }

    let muscles =
        view_filter_section!("Muscles", muscle_list, toggle_muscle, exercise_filter, true);
    let force = view_filter_section!("Force", force_list, toggle_force, exercise_filter, true);
    let mechanic = view_filter_section!(
        "Mechanic",
        mechanic_list,
        toggle_mechanic,
        exercise_filter,
        true
    );
    let laterality = view_filter_section!(
        "Laterality",
        laterality_list,
        toggle_laterality,
        exercise_filter,
        true
    );
    let assistance = view_filter_section!(
        "Assistance",
        assistance_list,
        toggle_assistance,
        exercise_filter,
        true
    );
    let equipment = view_filter_section!(
        "Equipment",
        equipment_list,
        toggle_equipment,
        exercise_filter,
        true
    );
    let category = view_filter_section!(
        "Category",
        category_list,
        toggle_category,
        exercise_filter,
        true
    );
    let catalog_count = exercise_filter.read().catalog().len();
    rsx! {
        Dialog {
            title: rsx! { "Filter exercises" },
            on_close: move |_| *filter_dialog_shown.write() = false,
            {muscles},
            {force},
            {mechanic},
            {laterality},
            {assistance},
            {equipment},
            {category},
            div {
                class: "control",
                onclick: move |_| *filter_dialog_shown.write() = false,
                button {
                    class: "button is-primary",
                    "Show {exercise_count} custom and {catalog_count} catalog exercises"
                }
            }
        }
    }
}

pub enum ExerciseDialog {
    None,
    Options(domain::Exercise),
    Add {
        name: FieldValue<domain::Name>,
    },
    Copy {
        name: FieldValue<domain::Name>,
        muscles: Vec<domain::ExerciseMuscle>,
    },
    Rename {
        name: FieldValue<domain::Name>,
        exercise: domain::Exercise,
    },
    ChangeProperties {
        exercise: domain::Exercise,
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

    fn from_base64(value: &str) -> Self {
        if value.is_empty() {
            return Self::default();
        }
        match URL_SAFE.decode(value) {
            Ok(bytes) => match postcard::from_bytes(&bytes) {
                Ok(exercise_filter) => exercise_filter,
                Err(err) => {
                    warn!("failed to decode exercise filter: {err}");
                    Self::default()
                }
            },
            Err(err) => {
                warn!("failed to decode base64-encoded exercise filter: {err}");
                Self::default()
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
            domain::ExerciseFilter::try_from(ExerciseFilter::from_base64(&dto.to_base64())),
            Ok(exercise_filter)
        );
    }
}
