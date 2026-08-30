use std::collections::{BTreeSet, HashSet};

use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use chrono::Duration;
use dioxus::prelude::*;
use log::{error, warn};

use valens_domain::{self as domain, ExerciseService, Property};

use crate::{
    DOMAIN_SERVICE, Route,
    cache::{Cache, CacheState},
    current_date::current_date,
    eh,
    notification::notify,
    page,
    routing::NavigatorScrollExt,
    ui::{
        element::{
            Block, Color, DeleteConfirmationDialog, Dialog, ErrorPage, FloatingActionButton, Icon,
            ItemOptionsButton, LoadingPage, MenuOption, OptionsMenu, SaveDialog, SearchBox, Table,
            Title,
        },
        form::{
            ButtonSelectField, ButtonSelectOption, FieldValue, FieldValueState, InputField,
            MultiToggle, MultiToggleTags, TextAreaField,
        },
    },
};

macro_rules! show_add_dialog {
    ($dialog:ident, $name:ident, $properties:ident $(, $filter_string:ident, $exercises_page:ident)?) => {{
        let name = $name.clone();
        let properties = $properties.clone();
        $(let $filter_string = $filter_string.clone();)?
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
                properties,
            };
            $(
                if $exercises_page {
                    navigator().replace_preserving_scroll(Route::Exercises {
                        add: true,
                        filter: $filter_string,
                    });
                }
            )?
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
    let mut catalog_update_dialog_shown = use_signal(|| false);

    let exercise_filter = use_signal(|| {
        domain::ExerciseFilter::try_from(ExerciseFilter::from_base64(&filter)).unwrap_or_default()
    });
    let name = exercise_filter.read().name.clone();
    let properties = exercise_filter.read().exercise_properties();

    use_future({
        let name = name.clone();
        let properties = properties.clone();
        let filter = filter.clone();
        move || {
            let name = name.clone();
            let properties = properties.clone();
            let filter = filter.clone();
            async move {
                if add {
                    show_add_dialog!(dialog, name, properties, filter, exercises_page).await;
                }
            }
        }
    });

    match (&*cache.exercises.read(), &*cache.training_sessions.read()) {
        (CacheState::Ready(exercises), CacheState::Ready(training_sessions)) => {
            let filtered_exercises = exercise_filter.read().exercises(exercises.iter());
            rsx! {
                {view_search_box(exercise_filter, dialog, filter_dialog_shown, catalog_update_dialog_shown, properties.clone(), exercises_page)},
                {view_list(&filtered_exercises, exercises, training_sessions, exercise_filter, dialog, on_exercise_click, on_catalog_click)}
                {view_dialog(dialog, if exercises_page { Some(Route::Exercises { add: false, filter: filter.clone() }) } else { None })}
                {view_filter_dialog(exercise_filter, filter_dialog_shown, filtered_exercises.len())}
                if catalog_update_dialog_shown() {
                    CatalogUpdateDialog { on_close: move |()| catalog_update_dialog_shown.set(false) }
                }
                if exercises_page {
                    FloatingActionButton {
                        icon: "plus".to_string(),
                        on_click: move |_| {
                            show_add_dialog!(dialog, name, properties, filter, exercises_page)
                        },
                    }
                }
            }
        }
        (CacheState::Error(err), _) | (_, CacheState::Error(err)) => {
            rsx! { ErrorPage { message: "{err}" } }
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
                let n = domain::name_or_none(*element);
                rsx! {
                    span {
                        class: "tag is-hoverable",
                        class: if *enabled { "is-link" },
                        "data-testid": "filter-tag",
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

/// Show the muscles as chips colored by the level at which they are filtered.
///
/// With `selected_only`, only the selected muscles are shown and clicking a chip clears the muscle.
/// Otherwise all muscles are shown and clicking a chip cycles the level.
fn view_muscle_filter_tags(
    mut exercise_filter: Signal<domain::ExerciseFilter>,
    selected_only: bool,
) -> Element {
    let mut chips = exercise_filter
        .read()
        .muscle_list()
        .into_iter()
        .filter(|(_, level)| !selected_only || level.is_some())
        .map(|(muscle, level)| {
            let class = match level {
                Some(level) => {
                    format!(
                        "tag is-hoverable {}",
                        page::exercise::stimulus_level_class(level)
                    )
                }
                None => "tag is-hoverable".to_string(),
            };
            rsx! {
                span {
                    class: "{class}",
                    "data-testid": "filter-tag",
                    onclick: move |_| {
                        if selected_only {
                            exercise_filter.write().clear_muscle(Some(muscle));
                        } else {
                            exercise_filter.write().toggle_muscle(Some(muscle));
                        }
                    },
                    {muscle.name()}
                }
            }
        })
        .collect::<Vec<_>>();
    let not_set = exercise_filter.read().muscles_not_set();
    if !selected_only || not_set {
        chips.push(rsx! {
            span {
                class: "tag is-hoverable",
                class: if not_set { "is-link" },
                "data-testid": "filter-tag",
                onclick: move |_| exercise_filter.write().toggle_muscle(None),
                {domain::MuscleID::none_name()}
            }
        });
    }
    rsx! {
        for chip in chips {
            {chip}
        }
    }
}

fn view_search_box(
    mut exercise_filter: Signal<domain::ExerciseFilter>,
    mut filter_dialog: Signal<ExerciseDialog>,
    mut filter_dialog_shown: Signal<bool>,
    mut catalog_update_dialog_shown: Signal<bool>,
    properties: domain::ExerciseProperties,
    exercises_page: bool,
) -> Element {
    let name = exercise_filter.read().name.clone();
    let tags = domain::ExerciseProperty::iter()
        .map(|property| match property {
            domain::ExerciseProperty::Muscles => view_muscle_filter_tags(exercise_filter, true),
            domain::ExerciseProperty::Force => {
                view_filter_tags!(force_list, toggle_force, exercise_filter, true)
            }
            domain::ExerciseProperty::Mechanic => {
                view_filter_tags!(mechanic_list, toggle_mechanic, exercise_filter, true)
            }
            domain::ExerciseProperty::Laterality => {
                view_filter_tags!(laterality_list, toggle_laterality, exercise_filter, true)
            }
            domain::ExerciseProperty::Assistance => {
                view_filter_tags!(assistance_list, toggle_assistance, exercise_filter, true)
            }
            domain::ExerciseProperty::Category => {
                view_filter_tags!(category_list, toggle_category, exercise_filter, true)
            }
            domain::ExerciseProperty::Equipment => {
                view_filter_tags!(equipment_list, toggle_equipment, exercise_filter, true)
            }
        })
        .collect::<Vec<_>>();
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
                if exercises_page {
                    button {
                        class: "button",
                        "data-testid": "update-exercises-from-catalog",
                        onclick: move |_| *catalog_update_dialog_shown.write() = true,
                        Icon { name: "book-open" }
                    }
                } else {
                    button {
                        class: "button is-link",
                        "data-testid": "create-exercise",
                        onclick: move |_| {
                            show_add_dialog!(filter_dialog, name, properties)
                        },
                        Icon { name: "plus" }
                    }
                }
            }
            div {
                class: "is-flex px-4",
                div {
                    class: "tags is-flex-wrap-nowrap is-overflow-scroll is-scrollbar-width-none",
                    for tags in tags {
                        {tags}
                    }
                }
            }
        }
    }
}

fn view_list(
    exercises: &[&domain::Exercise],
    all_exercises: &[domain::Exercise],
    training_sessions: &[domain::TrainingSession],
    exercise_filter: Signal<domain::ExerciseFilter>,
    mut dialog: Signal<ExerciseDialog>,
    on_exercise_click: EventHandler<(MouseEvent, domain::ExerciseID)>,
    on_catalog_click: EventHandler<(MouseEvent, String)>,
) -> Element {
    const CURRENT_EXERCISE_CUTOFF_DAYS: i64 = 31;

    let cutoff = current_date() - Duration::days(CURRENT_EXERCISE_CUTOFF_DAYS);

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
                        "data-testid": "exercise-item",
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
                        "data-testid": "exercise-item",
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

    let existing_names = all_exercises
        .iter()
        .map(|e| &e.name)
        .collect::<BTreeSet<_>>();

    let catalog_exercises_body = exercise_filter
        .read()
        .catalog()
        .values()
        .map(|e| {
            let e = (*e).clone();
            let name = e.name.to_string();
            vec![
                rsx! {
                    span {
                        class: "has-text-link",
                        "data-testid": "catalog-item",
                        onclick: move |event| on_catalog_click((event, name.clone())),
                        "{e.name}"
                    }
                },
                rsx! {
                    if !existing_names.contains(&e.name) {
                        div {
                            class: "has-text-link has-text-right",
                            a {
                                class: "mx-2",
                                "data-testid": "add-catalog-exercise",
                                onclick: move |_| {
                                    let name = e.name.clone();
                                    let domain::ExerciseProperties {
                                        muscles,
                                        force,
                                        mechanic,
                                        laterality,
                                        assistance,
                                        equipment,
                                        category,
                                    } = domain::ExerciseProperties::from(&e);
                                    async move {
                                            match DOMAIN_SERVICE()
                                                .create_exercise(
                                                    name,
                                                    String::new(),
                                                    muscles,
                                                    force,
                                                    mechanic,
                                                    laterality,
                                                    assistance,
                                                    equipment,
                                                    category,
                                                )
                                                .await
                                            {
                                                Ok(_) => {
                                                    consume_context::<Cache>().refresh_exercises();
                                                }
                                                Err(err) => {
                                                    notify("add exercise from catalog", &err);
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
                    if let ExerciseDialog::Copy { name, .. } | ExerciseDialog::Rename { name, .. } = &*dialog.read()
                        && let Ok(name) = name.validated.clone() {
                            match &*dialog.read() {
                                ExerciseDialog::Copy { exercise, .. } => {
                                    match DOMAIN_SERVICE()
                                        .create_exercise(
                                            name,
                                            exercise.notes.clone(),
                                            exercise.muscles.clone(),
                                            exercise.force,
                                            exercise.mechanic,
                                            exercise.laterality,
                                            exercise.assistance,
                                            exercise.equipment.clone(),
                                            exercise.category,
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            saved = true;
                                            consume_context::<Cache>().refresh_exercises();
                                        }
                                        Err(err) => {
                                            notify("copy exercise", &err);
                                        }
                                    }
                                }
                                ExerciseDialog::Rename { exercise, .. } => {
                                    match DOMAIN_SERVICE()
                                        .replace_exercise(domain::Exercise {
                                            name,
                                            ..exercise.clone()
                                        })
                                        .await
                                    {
                                        Ok(_) => {
                                            saved = true;
                                            consume_context::<Cache>().refresh_exercises();
                                        }
                                        Err(err) => {
                                            notify("rename exercise", &err);
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
                            Err(err) => notify("delete exercise", &err)
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
            rsx! {
                OptionsMenu {
                    options: vec![
                        rsx! {
                            MenuOption {
                                icon: "copy".to_string(),
                                text: "Copy exercise".to_string(),
                                "data-testid": "options-copy",
                                on_click: eh!(exercise_name, exercise; {
                                    async move {
                                        let validated_name = DOMAIN_SERVICE().validate_exercise_name(&exercise_name.to_string(), domain::ExerciseID::nil()).await.map_err(|err| err.to_string());
                                        *dialog.write() = ExerciseDialog::Copy {
                                            name: FieldValue {
                                                input: exercise_name.to_string(),
                                                validated: validated_name,
                                                orig: exercise_name.to_string(),
                                            },
                                            exercise,
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
                                icon: "note-sticky".to_string(),
                                text: "Edit exercise notes".to_string(),
                                "data-testid": "options-edit-exercise-notes",
                                on_click: eh!(exercise; {
                                    *dialog.write() = ExerciseDialog::EditNotes {
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
        ExerciseDialog::Add { name, properties } => rsx! {
            AddExerciseDialog {
                dialog,
                disabled: !name.valid(),
                properties: properties.clone(),
                on_close: eh!(close_dialog; { close_dialog(); }),
            }
        },
        ExerciseDialog::Copy { name, .. } | ExerciseDialog::Rename { name, .. } => rsx! {
            SaveDialog {
                title: rsx! { match &*dialog.read() { ExerciseDialog::Copy { .. } =>  { "Copy exercise" }, ExerciseDialog::Rename { .. } =>  { "Rename exercise" }, _ => "" } },
                on_close: eh!(close_dialog; { close_dialog(); }),
                on_save: save,
                is_loading: is_loading(),
                disabled: !name.valid(),
                {view_name_field(dialog)}
            }
        },
        ExerciseDialog::EditNotes { exercise } => rsx! {
            ExerciseNotesDialog { exercise: exercise.clone(), on_close: eh!(close_dialog; { close_dialog(); }) }
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

#[component]
fn AddExerciseDialog(
    dialog: Signal<ExerciseDialog>,
    disabled: bool,
    properties: domain::ExerciseProperties,
    on_close: EventHandler<()>,
) -> Element {
    let fields = PropertyFields::new(&properties);
    let mut is_loading = use_signal(|| false);

    let save = move |_| async move {
        let name = match &*dialog.read() {
            ExerciseDialog::Add { name, .. } => name.validated.clone(),
            _ => return,
        };
        let Ok(name) = name else {
            return;
        };
        let domain::ExerciseProperties {
            muscles,
            force,
            mechanic,
            laterality,
            assistance,
            equipment,
            category,
        } = fields.properties();
        let mut saved = false;
        is_loading.set(true);
        match DOMAIN_SERVICE()
            .create_exercise(
                name,
                String::new(),
                muscles,
                force,
                mechanic,
                laterality,
                assistance,
                equipment,
                category,
            )
            .await
        {
            Ok(_) => {
                saved = true;
                consume_context::<Cache>().load_exercises().await;
            }
            Err(err) => {
                notify("add exercise", &err);
            }
        }
        is_loading.set(false);
        if saved {
            on_close(());
        }
    };

    let sections = fields.sections();

    rsx! {
        SaveDialog {
            title: rsx! { "Add exercise" },
            on_close: eh!(on_close; { on_close(()); }),
            on_save: save,
            is_loading: is_loading(),
            disabled,
            {view_name_field(dialog)}
            for section in sections {
                {section}
            }
        }
    }
}

/// Show the name of an exercise, validated while it is entered.
fn view_name_field(mut dialog: Signal<ExerciseDialog>) -> Element {
    let (input, validated, changed) = match &*dialog.read() {
        ExerciseDialog::Add { name, .. }
        | ExerciseDialog::Copy { name, .. }
        | ExerciseDialog::Rename { name, .. } => {
            (name.input.clone(), name.validated.clone(), name.changed())
        }
        _ => return rsx! {},
    };
    rsx! {
        InputField {
            label: "Name".to_string(),
            "data-testid": "dialog-name",
            value: input,
            error: if let Err(err) = &validated { err.clone() },
            has_changed: changed,
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
}

macro_rules! view_filter_section {
    ($property:expr, $list:ident, $toggle:ident, $exercise_filter:ident) => {{
        let tags = view_filter_tags!($list, $toggle, $exercise_filter, false);
        let name = $property.name();
        view_filter_block(rsx! { {name} }, name, tags)
    }};
}

fn view_filter_block(label: Element, name: &str, tags: Element) -> Element {
    let test_id = format!("filter-section-{}", name.to_lowercase());
    rsx! {
        Block {
            label {
                class: "subtitle",
                {label}
            }
            div {
                class: "container py-3",
                "data-testid": test_id,
                div {
                    class: "tags",
                    {tags}
                }
            }
        }
    }
}

/// The stimulus levels in the order in which the muscle toggles cycle through them.
const STIMULUS_LEVELS: [domain::StimulusLevel; 2] = [
    domain::StimulusLevel::Primary,
    domain::StimulusLevel::Secondary,
];

/// Show the stimulus levels with the colors used for the muscle tags.
fn view_stimulus_level_legend() -> Element {
    let tags = STIMULUS_LEVELS
        .into_iter()
        .enumerate()
        .map(|(i, level)| {
            let class = page::exercise::stimulus_level_class(level);
            rsx! {
                if i > 0 {
                    " "
                }
                span {
                    class: "tag {class}",
                    {level.name()}
                }
            }
        })
        .collect::<Vec<_>>();
    rsx! {
        " ("
        for tag in tags {
            {tag}
        }
        ")"
    }
}

/// Show the values of a property as chips, of which at most one is selected.
///
/// Clicking the selected chip clears the property.
fn view_property_section<T: Property + PartialEq + 'static>(
    property: domain::ExerciseProperty,
    mut selected: Signal<Option<T>>,
) -> Element {
    let chips = T::iter()
        .map(|value| {
            let value = *value;
            let enabled = *selected.read() == Some(value);
            rsx! {
                span {
                    class: "tag is-hoverable",
                    class: if enabled { "is-link" },
                    "data-testid": "property-chip",
                    onclick: move |_| selected.set(if enabled { None } else { Some(value) }),
                    {value.name()}
                }
            }
        })
        .collect::<Vec<_>>();
    view_dialog_section(property.name(), chips)
}

/// Show the equipment as chips, of which any number is selected.
fn view_equipment_section(mut selected: Signal<Vec<domain::Equipment>>) -> Element {
    let chips = domain::Equipment::iter()
        .map(|value| {
            let value = *value;
            let enabled = selected.read().contains(&value);
            rsx! {
                span {
                    class: "tag is-hoverable",
                    class: if enabled { "is-link" },
                    "data-testid": "property-chip",
                    onclick: move |_| {
                        if enabled {
                            selected.write().retain(|e| *e != value);
                        } else {
                            let mut selected = selected.write();
                            selected.push(value);
                            selected.sort_by_key(|e| {
                                domain::Equipment::iter().position(|v| v == e)
                            });
                        }
                    },
                    {value.name()}
                }
            }
        })
        .collect::<Vec<_>>();
    view_dialog_section(domain::ExerciseProperty::Equipment.name(), chips)
}

/// Show the muscles as toggles that cycle through the stimulus.
fn view_muscles_section(multi_toggle: Signal<MultiToggle>) -> Element {
    rsx! {
        Block {
            label {
                class: "subtitle",
                {domain::ExerciseProperty::Muscles.name()}
                {view_stimulus_level_legend()}
            }
            div {
                class: "container py-3",
                MultiToggleTags { multi_toggle }
            }
        }
    }
}

fn view_dialog_section(title: &str, chips: Vec<Element>) -> Element {
    let title = title.to_string();
    rsx! {
        Block {
            label {
                class: "subtitle",
                {title}
            }
            div {
                class: "container py-3",
                div {
                    class: "tags",
                    for chip in chips {
                        {chip}
                    }
                }
            }
        }
    }
}

fn multi_toggle_state(level: Option<domain::StimulusLevel>) -> usize {
    level.map_or(0, |level| {
        STIMULUS_LEVELS
            .into_iter()
            .position(|l| l == level)
            .map_or(0, |i| i + 1)
    })
}

fn stimulus_level(state: usize) -> Option<domain::StimulusLevel> {
    state
        .checked_sub(1)
        .and_then(|i| STIMULUS_LEVELS.get(i))
        .copied()
}

/// The editable state of the properties of an exercise.
#[derive(Clone, Copy)]
struct PropertyFields {
    muscles: Signal<MultiToggle>,
    force: Signal<Option<domain::Force>>,
    mechanic: Signal<Option<domain::Mechanic>>,
    laterality: Signal<Option<domain::Laterality>>,
    assistance: Signal<Option<domain::Assistance>>,
    equipment: Signal<Vec<domain::Equipment>>,
    category: Signal<Option<domain::Category>>,
}

impl PropertyFields {
    fn new(properties: &domain::ExerciseProperties) -> Self {
        PropertyFields {
            muscles: use_signal(|| MultiToggle {
                states: domain::MuscleID::iter()
                    .map(|m| {
                        (
                            m.name().to_string(),
                            properties
                                .muscles
                                .iter()
                                .find(|em| em.muscle_id == *m)
                                .map(|em| {
                                    multi_toggle_state(domain::StimulusLevel::from_stimulus(
                                        em.stimulus,
                                    ))
                                })
                                .unwrap_or_default(),
                        )
                    })
                    .collect::<Vec<_>>(),
                classes: STIMULUS_LEVELS
                    .into_iter()
                    .map(page::exercise::stimulus_level_class)
                    .collect(),
            }),
            force: use_signal(|| properties.force),
            mechanic: use_signal(|| properties.mechanic),
            laterality: use_signal(|| properties.laterality),
            assistance: use_signal(|| properties.assistance),
            equipment: use_signal(|| properties.equipment.clone()),
            category: use_signal(|| properties.category),
        }
    }

    fn properties(&self) -> domain::ExerciseProperties {
        domain::ExerciseProperties {
            muscles: self
                .muscles
                .read()
                .states
                .iter()
                .enumerate()
                .filter_map(|(i, (_, state))| {
                    let level = stimulus_level(*state)?;
                    domain::MuscleID::iter()
                        .nth(i)
                        .map(|muscle_id| domain::ExerciseMuscle {
                            muscle_id: *muscle_id,
                            stimulus: level.stimulus(),
                        })
                })
                .collect(),
            force: (self.force)(),
            mechanic: (self.mechanic)(),
            laterality: (self.laterality)(),
            assistance: (self.assistance)(),
            equipment: (self.equipment)(),
            category: (self.category)(),
        }
    }

    fn sections(&self) -> Vec<Element> {
        domain::ExerciseProperty::iter()
            .map(|property| match property {
                domain::ExerciseProperty::Muscles => view_muscles_section(self.muscles),
                domain::ExerciseProperty::Force => view_property_section(*property, self.force),
                domain::ExerciseProperty::Mechanic => {
                    view_property_section(*property, self.mechanic)
                }
                domain::ExerciseProperty::Laterality => {
                    view_property_section(*property, self.laterality)
                }
                domain::ExerciseProperty::Assistance => {
                    view_property_section(*property, self.assistance)
                }
                domain::ExerciseProperty::Category => {
                    view_property_section(*property, self.category)
                }
                domain::ExerciseProperty::Equipment => view_equipment_section(self.equipment),
            })
            .collect()
    }
}

#[component]
fn ExerciseNotesDialog(exercise: domain::Exercise, on_close: EventHandler<()>) -> Element {
    let orig_notes = exercise.notes.clone();
    let mut notes = use_signal(|| orig_notes.clone());
    let mut is_loading = use_signal(|| false);
    let changed = notes().trim() != orig_notes.trim();

    let save = move |_| {
        let exercise = exercise.clone();
        async move {
            let mut saved = false;
            is_loading.set(true);
            match DOMAIN_SERVICE()
                .replace_exercise(domain::Exercise {
                    notes: notes().trim().to_string(),
                    ..exercise
                })
                .await
            {
                Ok(_) => {
                    saved = true;
                    consume_context::<Cache>().load_exercises().await;
                }
                Err(err) => {
                    notify("edit notes of exercise", &err);
                }
            }
            is_loading.set(false);
            if saved {
                on_close(());
            }
        }
    };

    rsx! {
        SaveDialog {
            title: rsx! { "Exercise notes" },
            on_close: eh!(on_close; { on_close(()); }),
            on_save: save,
            is_loading: is_loading(),
            disabled: !changed,
            TextAreaField {
                value: orig_notes.clone(),
                has_changed: changed,
                autofocus: true,
                "data-testid": "exercise-notes-input",
                on_input: move |event: FormEvent| {
                    notes.set(event.value());
                },
            }
        }
    }
}

#[component]
fn ExercisePropertiesDialog(
    exercise: domain::Exercise,
    on_save: EventHandler<MouseEvent>,
    on_close: EventHandler<()>,
) -> Element {
    let fields = PropertyFields::new(&domain::ExerciseProperties::from(&exercise));
    let mut is_loading = use_signal(|| false);

    macro_rules! is_loading {
        ($block:expr) => {
            is_loading.set(true);
            $block;
            is_loading.set(false);
        };
    }

    let save = move |_| {
        let exercise = exercise.clone();
        async move {
            let domain::ExerciseProperties {
                muscles,
                force,
                mechanic,
                laterality,
                assistance,
                equipment,
                category,
            } = fields.properties();
            let mut saved = false;
            is_loading! {
                match DOMAIN_SERVICE()
                    .replace_exercise(domain::Exercise {
                        muscles,
                        force,
                        mechanic,
                        laterality,
                        assistance,
                        equipment,
                        category,
                        ..exercise
                    })
                    .await
                {
                    Ok(_) => {
                        saved = true;
                        consume_context::<Cache>().load_exercises().await;
                    }
                    Err(err) => {
                        notify("change properties of exercise", &err);
                    }
                }
            }
            if saved {
                on_close(());
            }
        }
    };

    let sections = fields.sections();

    rsx! {
        SaveDialog {
            title: rsx! { "Change properties" },
            on_close: eh!(on_close; { on_close(()); }),
            on_save: save,
            is_loading: is_loading(),
            disabled: false,
            for section in sections {
                {section}
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

    let sections = domain::ExerciseProperty::iter()
        .map(|property| match property {
            domain::ExerciseProperty::Muscles => {
                let name = property.name();
                view_filter_block(
                    rsx! { {name} {view_stimulus_level_legend()} },
                    name,
                    view_muscle_filter_tags(exercise_filter, false),
                )
            }
            domain::ExerciseProperty::Force => {
                view_filter_section!(property, force_list, toggle_force, exercise_filter)
            }
            domain::ExerciseProperty::Mechanic => {
                view_filter_section!(property, mechanic_list, toggle_mechanic, exercise_filter)
            }
            domain::ExerciseProperty::Laterality => {
                view_filter_section!(
                    property,
                    laterality_list,
                    toggle_laterality,
                    exercise_filter
                )
            }
            domain::ExerciseProperty::Assistance => {
                view_filter_section!(
                    property,
                    assistance_list,
                    toggle_assistance,
                    exercise_filter
                )
            }
            domain::ExerciseProperty::Category => {
                view_filter_section!(property, category_list, toggle_category, exercise_filter)
            }
            domain::ExerciseProperty::Equipment => {
                view_filter_section!(property, equipment_list, toggle_equipment, exercise_filter)
            }
        })
        .collect::<Vec<_>>();
    let catalog_count = exercise_filter.read().catalog().len();
    rsx! {
        Dialog {
            title: rsx! { "Filter exercises" },
            on_close: move |_| *filter_dialog_shown.write() = false,
            for section in sections {
                {section}
            }
            div {
                class: "control",
                onclick: move |_| *filter_dialog_shown.write() = false,
                button {
                    class: "button is-primary",
                    "data-testid": "filter-show",
                    "Show {exercise_count} custom and {catalog_count} catalog exercises"
                }
            }
        }
    }
}

#[component]
fn CatalogUpdateDialog(on_close: EventHandler<()>) -> Element {
    let cache = consume_context::<Cache>();
    let mode = use_signal(|| domain::CatalogUpdateMode::FillMissing);
    let updates = use_memo(move || match &*cache.exercises.read() {
        CacheState::Ready(exercises) => domain::catalog_updates(exercises, mode()),
        CacheState::Loading | CacheState::Error(_) => vec![],
    });
    let mut selected = use_signal(|| default_selection(&updates.read()));
    // A manually changed selection must survive a refresh of the exercises.
    let touched = use_signal(|| false);
    let mut confirmation_shown = use_signal(|| false);
    let mut is_updating = use_signal(|| false);
    let mut updated = use_signal(|| 0_usize);

    use_effect(move || {
        let selection = default_selection(&updates.read());
        if !*touched.peek() {
            selected.set(selection);
        }
    });

    let update = move || async move {
        confirmation_shown.set(false);
        is_updating.set(true);
        updated.set(0);
        let exercises = updates
            .read()
            .iter()
            .filter(|update| selected.read().contains(&update.exercise.id))
            .map(|update| update.exercise.clone())
            .collect::<Vec<_>>();
        let attempted = exercises.len();
        let mut errors = vec![];
        for exercise in exercises {
            if let Err(err) = DOMAIN_SERVICE().replace_exercise(exercise).await {
                errors.push(err);
            }
            *updated.write() += 1;
        }
        consume_context::<Cache>().load_exercises().await;
        is_updating.set(false);
        if let Some(err) = errors.first() {
            notify(
                format!(
                    "update {} of {attempted} exercises from catalog",
                    errors.len()
                ),
                err,
            );
        }
        if updates.read().is_empty() {
            on_close(());
        }
    };

    let rows = updates
        .read()
        .iter()
        .map(|update| view_catalog_update(update, selected, touched, is_updating))
        .collect::<Vec<_>>();
    let selected_count = updates
        .read()
        .iter()
        .filter(|update| selected.read().contains(&update.exercise.id))
        .count();

    rsx! {
        Dialog {
            title: rsx! { "Update from catalog" },
            on_close: move |_| {
                if !is_updating() {
                    on_close(());
                }
            },
            {view_mode_selection(mode, touched, is_updating)},
            if rows.is_empty() {
                div {
                    class: "block has-text-centered",
                    "data-testid": "no-catalog-updates",
                    "No exercise can be updated from a catalog exercise with a matching name."
                }
            } else {
                for row in rows {
                    {row}
                }
                div {
                    class: "field is-grouped is-grouped-centered",
                    div {
                        class: "control",
                        button {
                            class: "button is-primary",
                            "data-testid": "update-from-catalog",
                            disabled: selected_count == 0 || is_updating(),
                            onclick: move |_| async move {
                                if mode() == domain::CatalogUpdateMode::ReplaceAll {
                                    confirmation_shown.set(true);
                                } else {
                                    update().await;
                                }
                            },
                            if is_updating() {
                                "Update {updated} of {selected_count} exercises"
                            } else {
                                "Update {selected_count} exercises"
                            }
                        }
                    }
                }
            }
            if confirmation_shown() {
                Dialog {
                    title: rsx! { "Replace all values of {selected_count} exercises?" },
                    on_close: move |_| confirmation_shown.set(false),
                    color: Color::Danger,
                    div {
                        class: "block",
                        "Values that the catalog exercises do not have will be cleared."
                    }
                    div {
                        class: "field is-grouped is-grouped-centered",
                        div {
                            class: "control",
                            onclick: move |_| confirmation_shown.set(false),
                            button {
                                class: "button is-light is-soft",
                                "data-testid": "dialog-no",
                                "No"
                            }
                        }
                        div {
                            class: "control",
                            button {
                                class: "button is-danger",
                                "data-testid": "dialog-yes",
                                onclick: move |_| update(),
                                "Yes, replace all values"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Show the update modes as a group of buttons, of which exactly one is selected.
fn view_mode_selection(
    mut mode: Signal<domain::CatalogUpdateMode>,
    mut touched: Signal<bool>,
    is_updating: Signal<bool>,
) -> Element {
    rsx! {
        Block {
            ButtonSelectField {
                label: "Mode".to_string(),
                options: vec![
                    ButtonSelectOption {
                        text: "Fill in missing values".to_string(),
                        value: domain::CatalogUpdateMode::FillMissing,
                    },
                    ButtonSelectOption {
                        text: "Replace all values".to_string(),
                        value: domain::CatalogUpdateMode::ReplaceAll,
                    },
                ],
                selected: mode(),
                has_changed: false,
                is_expanded: true,
                "data-testid": "mode-selection",
                on_click: move |(_, value)| {
                    if !is_updating() {
                        mode.set(value);
                        touched.set(false);
                    }
                },
            }
        }
    }
}

fn view_catalog_update(
    update: &domain::CatalogUpdate,
    mut selected: Signal<HashSet<domain::ExerciseID>>,
    mut touched: Signal<bool>,
    is_updating: Signal<bool>,
) -> Element {
    let id = update.exercise.id;
    let name = update.exercise.name.to_string();
    let is_selected = selected.read().contains(&id);
    let catalog_name = match update.catalog_match {
        domain::CatalogMatch::Exact => None,
        domain::CatalogMatch::Prefix => Some(update.catalog_name.to_string()),
    };
    let changes = update
        .changes
        .iter()
        .map(view_property_change)
        .collect::<Vec<_>>();
    rsx! {
        div {
            class: "block",
            "data-testid": "catalog-update",
            div {
                class: "is-flex is-align-items-center is-clickable",
                "data-testid": "catalog-update-toggle",
                "data-selected": "{is_selected}",
                onclick: move |_| {
                    if !is_updating() {
                        touched.set(true);
                        let mut selection = selected.write();
                        if !selection.remove(&id) {
                            selection.insert(id);
                        }
                    }
                },
                Icon {
                    name: if is_selected { "square-check".to_string() } else { "square".to_string() },
                    class: if is_selected { "has-text-link".to_string() } else { "has-text-muted".to_string() },
                }
                span { class: "has-text-weight-bold", {name} }
            }
            div {
                class: "ml-5",
                if let Some(catalog_name) = catalog_name {
                    div {
                        class: "is-italic",
                        "data-testid": "catalog-update-source",
                        "from {catalog_name}"
                    }
                }
                for change in changes {
                    {change}
                }
            }
        }
    }
}

fn view_property_change(change: &domain::PropertyChange) -> Element {
    let property = change.property.name();
    let before = change
        .before
        .iter()
        .map(view_property_value)
        .collect::<Vec<_>>();
    let after = change
        .after
        .iter()
        .map(view_property_value)
        .collect::<Vec<_>>();
    rsx! {
        div {
            class: "mb-2",
            "data-testid": "catalog-update-change",
            div { class: "is-size-7", {property} }
            div {
                class: "is-flex is-align-items-center",
                div {
                    class: "tags mb-0",
                    for tag in before {
                        {tag}
                    }
                }
                Icon { name: "arrow-right", is_small: true, px: 3 }
                div {
                    class: "tags mb-0",
                    for tag in after {
                        {tag}
                    }
                }
            }
        }
    }
}

fn view_property_value(value: &domain::PropertyValue) -> Element {
    let name = value.name;
    let stimulus_class = value
        .stimulus
        .and_then(domain::StimulusLevel::from_stimulus)
        .map_or("", page::exercise::stimulus_level_class);
    rsx! {
        span {
            class: "tag {stimulus_class}",
            {name}
        }
    }
}

fn default_selection(updates: &[domain::CatalogUpdate]) -> HashSet<domain::ExerciseID> {
    updates
        .iter()
        .filter(|update| update.catalog_match == domain::CatalogMatch::Exact)
        .map(|update| update.exercise.id)
        .collect()
}

pub enum ExerciseDialog {
    None,
    Options(domain::Exercise),
    Add {
        name: FieldValue<domain::Name>,
        properties: domain::ExerciseProperties,
    },
    Copy {
        name: FieldValue<domain::Name>,
        exercise: domain::Exercise,
    },
    Rename {
        name: FieldValue<domain::Name>,
        exercise: domain::Exercise,
    },
    EditNotes {
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
    pub muscles: HashSet<(u8, u8)>,
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

/// Encoding of a filter value that stands for the absence of the property.
const NONE: u8 = 255;

/// Encoding of the stimulus levels of the muscles filter.
const SECONDARY: u8 = 1;
const PRIMARY: u8 = 2;

impl From<domain::ExerciseFilter> for ExerciseFilter {
    fn from(value: domain::ExerciseFilter) -> Self {
        Self {
            name: value.name.clone(),
            muscles: value
                .muscles
                .iter()
                .map(|v| match v {
                    Some((muscle, domain::StimulusLevel::Secondary)) => (*muscle as u8, SECONDARY),
                    Some((muscle, domain::StimulusLevel::Primary)) => (*muscle as u8, PRIMARY),
                    None => (NONE, 0),
                })
                .collect(),
            force: value
                .force
                .iter()
                .map(|v| v.map_or(NONE, |v| v as u8))
                .collect(),
            mechanic: value
                .mechanic
                .iter()
                .map(|v| v.map_or(NONE, |v| v as u8))
                .collect(),
            laterality: value
                .laterality
                .iter()
                .map(|v| v.map_or(NONE, |v| v as u8))
                .collect(),
            assistance: value
                .assistance
                .iter()
                .map(|v| v.map_or(NONE, |v| v as u8))
                .collect(),
            equipment: value
                .equipment
                .iter()
                .map(|v| v.map_or(NONE, |v| v as u8))
                .collect(),
            category: value
                .category
                .iter()
                .map(|v| v.map_or(NONE, |v| v as u8))
                .collect(),
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
            muscles: decode_muscles(value.muscles),
            force: decode_values(value.force),
            mechanic: decode_values(value.mechanic),
            laterality: decode_values(value.laterality),
            assistance: decode_values(value.assistance),
            equipment: decode_values(value.equipment),
            category: decode_values(value.category),
        })
    }
}

/// Decode the muscles filter, dropping pairs with an unknown muscle or level.
fn decode_muscles(
    values: HashSet<(u8, u8)>,
) -> HashSet<Option<(domain::MuscleID, domain::StimulusLevel)>> {
    values
        .into_iter()
        .filter_map(|(muscle, level)| {
            if (muscle, level) == (NONE, 0) {
                return Some(None);
            }
            let muscle = domain::MuscleID::try_from(muscle).ok()?;
            let level = match level {
                SECONDARY => domain::StimulusLevel::Secondary,
                PRIMARY => domain::StimulusLevel::Primary,
                _ => return None,
            };
            Some(Some((muscle, level)))
        })
        .collect()
}

/// Decode a filter section, dropping values that are not valid for the property.
fn decode_values<T: TryFrom<u8> + Eq + std::hash::Hash>(values: HashSet<u8>) -> HashSet<Option<T>> {
    values
        .into_iter()
        .filter_map(|value| {
            if value == NONE {
                Some(None)
            } else {
                T::try_from(value).ok().map(Some)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exercise_filter_base64_round_trip() {
        let exercise_filter = domain::ExerciseFilter {
            name: "Exercise Name".to_string(),
            muscles: [
                Some((domain::MuscleID::Lats, domain::StimulusLevel::Primary)),
                Some((domain::MuscleID::Traps, domain::StimulusLevel::Secondary)),
                None,
            ]
            .into(),
            force: [Some(domain::Force::Pull)].into(),
            mechanic: [Some(domain::Mechanic::Isolation)].into(),
            laterality: [Some(domain::Laterality::Unilateral)].into(),
            assistance: [Some(domain::Assistance::Assisted)].into(),
            equipment: [
                Some(domain::Equipment::GymnasticRings),
                Some(domain::Equipment::ResistanceBand),
                None,
            ]
            .into(),
            category: [Some(domain::Category::Plyometrics), None].into(),
        };
        let dto = ExerciseFilter::from(exercise_filter.clone());

        assert_eq!(
            domain::ExerciseFilter::try_from(ExerciseFilter::from_base64(&dto.to_base64())),
            Ok(exercise_filter)
        );
    }

    #[test]
    fn test_exercise_filter_invalid_values_dropped() {
        let dto = ExerciseFilter {
            muscles: [
                (0, PRIMARY),
                (domain::MuscleID::Lats as u8, 3),
                (domain::MuscleID::Traps as u8, SECONDARY),
            ]
            .into(),
            equipment: [0, domain::Equipment::Cable as u8].into(),
            ..ExerciseFilter::default()
        };

        assert_eq!(
            domain::ExerciseFilter::try_from(dto),
            Ok(domain::ExerciseFilter {
                muscles: [Some((
                    domain::MuscleID::Traps,
                    domain::StimulusLevel::Secondary
                ))]
                .into(),
                equipment: [Some(domain::Equipment::Cable)].into(),
                ..domain::ExerciseFilter::default()
            })
        );
    }
}
