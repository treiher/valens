use std::collections::{BTreeMap, BTreeSet};

use chrono::{Duration, Local};
use seed::{prelude::*, *};
use valens_domain as domain;

use crate::{common, data};

const CURRENT_EXERCISE_CUTOFF_DAYS: i64 = 31;

// ------ ------
//     Model
// ------ ------

#[allow(clippy::struct_excessive_bools)]
pub struct Model {
    pub filter: domain::ExerciseFilter,
    view_filter_dialog: bool,
    view_create: bool,
    view_edit: bool,
    view_delete: bool,
    search_bar_padding: bool,
}

impl Model {
    #[allow(clippy::fn_params_excessive_bools)]
    pub fn new(
        view_create: bool,
        view_edit: bool,
        view_delete: bool,
        search_bar_padding: bool,
    ) -> Self {
        Self {
            filter: domain::ExerciseFilter::default(),
            view_filter_dialog: false,
            view_create,
            view_edit,
            view_delete,
            search_bar_padding,
        }
    }

    pub fn with_filter(self, filter: domain::ExerciseFilter) -> Self {
        Self { filter, ..self }
    }
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    SearchTermChanged(String),
    MuscleFilterChanged(domain::Muscle),
    ForceFilterChanged(domain::Force),
    MechanicFilterChanged(domain::Mechanic),
    LateralityFilterChanged(domain::Laterality),
    AssistanceFilterChanged(domain::Assistance),
    EquipmentFilterChanged(domain::Equipment),
    CategoryFilterChanged(domain::Category),

    Selected(u32),
    CreateClicked(String),
    EditClicked(u32),
    DeleteClicked(u32),

    ShowFilterDialog,
    CloseFilterDialog,

    AddExerciseFromCatalog(&'static domain::catalog::Exercise),
    CatalogExerciseSelected(&'static str),
}

pub enum OutMsg {
    None,
    Selected(u32),
    CreateClicked(String),
    EditClicked(u32),
    DeleteClicked(u32),
    CatalogExerciseSelected(&'static str),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) -> OutMsg {
    match msg {
        Msg::SearchTermChanged(search_term) => {
            model.filter.name = search_term;
            OutMsg::None
        }
        Msg::MuscleFilterChanged(muscle) => {
            model.filter.toggle_muscle(muscle);
            OutMsg::None
        }
        Msg::ForceFilterChanged(force) => {
            model.filter.toggle_force(force);
            OutMsg::None
        }
        Msg::MechanicFilterChanged(mechanic) => {
            model.filter.toggle_mechanic(mechanic);
            OutMsg::None
        }
        Msg::LateralityFilterChanged(laterality) => {
            model.filter.toggle_laterality(laterality);
            OutMsg::None
        }
        Msg::AssistanceFilterChanged(assistance) => {
            model.filter.toggle_assistance(assistance);
            OutMsg::None
        }
        Msg::EquipmentFilterChanged(equipment) => {
            model.filter.toggle_equipment(equipment);
            OutMsg::None
        }
        Msg::CategoryFilterChanged(category) => {
            model.filter.toggle_category(category);
            OutMsg::None
        }

        Msg::Selected(exercise_id) => OutMsg::Selected(exercise_id),
        Msg::CreateClicked(exercise_id) => OutMsg::CreateClicked(exercise_id),
        Msg::EditClicked(exercise_id) => OutMsg::EditClicked(exercise_id),
        Msg::DeleteClicked(exercise_id) => OutMsg::DeleteClicked(exercise_id),

        Msg::ShowFilterDialog => {
            model.view_filter_dialog = true;
            OutMsg::None
        }
        Msg::CloseFilterDialog => {
            model.view_filter_dialog = false;
            OutMsg::None
        }

        Msg::AddExerciseFromCatalog(catalog_exercise) => {
            let mut muscles = vec![];
            for (m, s) in catalog_exercise.muscles {
                muscles.push(domain::ExerciseMuscle {
                    muscle_id: domain::Muscle::id(*m),
                    stimulus: *s as u8,
                });
            }
            orders.notify(data::Msg::CreateExercise(
                catalog_exercise.name.to_string(),
                muscles,
            ));
            OutMsg::None
        }
        Msg::CatalogExerciseSelected(idx) => OutMsg::CatalogExerciseSelected(idx),
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model, loading: bool, data_model: &data::Model) -> Vec<Node<Msg>> {
    let cutoff = Local::now().date_naive() - Duration::days(CURRENT_EXERCISE_CUTOFF_DAYS);

    let current_exercise_ids = data_model
        .training_sessions
        .iter()
        .filter(|(_, s)| s.date >= cutoff)
        .flat_map(|(_, session)| session.exercises())
        .collect::<BTreeSet<_>>();

    let previous_exercise_ids = data_model
        .training_sessions
        .iter()
        .filter(|(_, s)| s.date < cutoff)
        .flat_map(|(_, session)| session.exercises())
        .collect::<BTreeSet<_>>();

    let exercises = data_model.exercises(&model.filter);

    let mut current_exercises = exercises
        .iter()
        .filter(|e| current_exercise_ids.contains(&e.id) || !previous_exercise_ids.contains(&e.id))
        .collect::<Vec<_>>();
    current_exercises.sort_by(|a, b| a.name.cmp(&b.name));

    let mut previous_exercises = exercises
        .iter()
        .filter(|e| !current_exercise_ids.contains(&e.id) && previous_exercise_ids.contains(&e.id))
        .collect::<Vec<_>>();
    previous_exercises.sort_by(|a, b| a.name.cmp(&b.name));

    let catalog_exercises = model.filter.catalog();

    nodes![
        IF![model.view_filter_dialog => view_filter_dialog(&model.filter, exercises.len(), catalog_exercises.len())],
        div![
            C!["field"],
            C!["is-grouped"],
            C![IF![model.search_bar_padding => "px-4"]],
            common::view_search_box(&model.filter.name, Msg::SearchTermChanged),
            div![
                C!["control"],
                button![
                    C!["button"],
                    C![IF![!model.filter.is_empty() => "is-link"]],
                    ev(Ev::Click, |_| Msg::ShowFilterDialog),
                    span![C!["icon"], i![C!["fas fa-filter"]]]
                ]
            ],
            if model.view_create {
                let disabled = loading
                    || model.filter.name.trim().is_empty()
                    || current_exercises
                        .iter()
                        .any(|e| e.name == *model.filter.name.trim());
                div![
                    C!["control"],
                    button![
                        C!["button"],
                        C!["is-link"],
                        C![IF![loading => "is-loading"]],
                        attrs! {
                            At::Disabled => disabled.as_at_value()
                        },
                        ev(Ev::Click, {
                            let search_term = model.filter.name.trim().to_string();
                            move |_| Msg::CreateClicked(search_term)
                        }),
                        span![C!["icon"], i![C!["fas fa-plus"]]]
                    ]
                ]
            } else {
                empty![]
            }
        ],
        div![
            C!["is-flex"],
            C![IF![model.search_bar_padding => "px-4"]],
            div![
                C!["tags"],
                C!["is-flex-wrap-nowrap"],
                C!["is-overflow-scroll"],
                C!["is-scrollbar-width-none"],
                view_filter_tags(
                    &model.filter.muscle_list(),
                    |_, e| Msg::MuscleFilterChanged(e),
                    true
                ),
                view_filter_tags(
                    &model.filter.force_list(),
                    |_, e| Msg::ForceFilterChanged(e),
                    true
                ),
                view_filter_tags(
                    &model.filter.mechanic_list(),
                    |_, e| Msg::MechanicFilterChanged(e),
                    true
                ),
                view_filter_tags(
                    &model.filter.laterality_list(),
                    |_, e| Msg::LateralityFilterChanged(e),
                    true
                ),
                view_filter_tags(
                    &model.filter.assistance_list(),
                    |_, e| Msg::AssistanceFilterChanged(e),
                    true
                ),
                view_filter_tags(
                    &model.filter.equipment_list(),
                    |_, e| Msg::EquipmentFilterChanged(e),
                    true
                ),
                view_filter_tags(
                    &model.filter.category_list(),
                    |_, e| Msg::CategoryFilterChanged(e),
                    true
                ),
            ],
        ],
        view_exercises(model, &current_exercises),
        IF![
            !previous_exercises.is_empty() =>
                nodes![
                    div![
                        C!["container"],
                        C!["has-text-centered"],
                        C!["my-3"],
                        common::view_element_with_description(
                            common::view_title(&span!["Previous exercises"], 0),
                            &format!("Exercises not performed within the last {CURRENT_EXERCISE_CUTOFF_DAYS} days")
                        ),
                    ]
                    view_exercises(model, &previous_exercises)
                ]
        ],
        IF![!catalog_exercises.is_empty() => common::view_title(&span!["Catalog"], 3)],
        view_catalog_exercises(
            catalog_exercises,
            &data_model.exercises(&domain::ExerciseFilter::default())
        ),
    ]
}

fn view_exercises(model: &Model, exercises: &[&&domain::Exercise]) -> Vec<Node<Msg>> {
    if exercises.is_empty() {
        return vec![];
    }
    nodes![div![
        C!["table-container"],
        C!["mt-2"],
        table![
            C!["table"],
            C!["is-fullwidth"],
            C!["is-hoverable"],
            tbody![exercises.iter().map(|e| {
                tr![td![
                    C!["is-flex"],
                    C!["is-justify-content-space-between"],
                    C!["has-text-link"],
                    span![
                        ev(Ev::Click, {
                            let exercise_id = e.id;
                            move |_| Msg::Selected(exercise_id)
                        }),
                        e.name.to_string(),
                    ],
                    p![
                        C!["is-flex is-flex-wrap-nowrap"],
                        if model.view_edit {
                            a![
                                C!["icon"],
                                C!["mr-1"],
                                ev(Ev::Click, {
                                    let exercise_id = e.id;
                                    move |_| Msg::EditClicked(exercise_id)
                                }),
                                i![C!["fas fa-edit"]]
                            ]
                        } else {
                            empty![]
                        },
                        if model.view_delete {
                            a![
                                C!["icon"],
                                C!["ml-1"],
                                ev(Ev::Click, {
                                    let exercise_id = e.id;
                                    move |_| Msg::DeleteClicked(exercise_id)
                                }),
                                i![C!["fas fa-times"]]
                            ]
                        } else {
                            empty![]
                        }
                    ]
                ]]
            })],
        ]
    ]]
}

fn view_catalog_exercises(
    catalog_exercises: BTreeMap<&'static str, &'static domain::catalog::Exercise>,
    exercises: &[&domain::Exercise],
) -> Node<Msg> {
    div![
        C!["table-container"],
        C!["mt-2"],
        table![
            C!["table"],
            C!["is-fullwidth"],
            C!["is-hoverable"],
            tbody![catalog_exercises.into_values().map(|e| {
                tr![td![
                    C!["is-flex"],
                    C!["is-justify-content-space-between"],
                    C!["has-text-link"],
                    span![
                        ev(Ev::Click, move |_| Msg::CatalogExerciseSelected(e.name)),
                        e.name.to_string(),
                    ],
                    IF![
                        !exercises.iter().any(|x| x.name == e.name) =>
                        p![
                            C!["is-flex is-flex-wrap-nowrap"],
                            a![
                                C!["icon"],
                                ev(Ev::Click, move |_| Msg::AddExerciseFromCatalog(e)),
                                i![C!["fas fa-plus"]]
                            ]
                        ]
                    ]
                ]]
            })],
        ]
    ]
}

fn view_filter_dialog(
    filter: &domain::ExerciseFilter,
    exercise_count: usize,
    catalog_exercise_count: usize,
) -> Node<Msg> {
    common::view_dialog(
        "primary",
        span!["Filter exercises"],
        nodes![
            view_filter_section("Muscles", &filter.muscle_list(), |_, e| {
                Msg::MuscleFilterChanged(e)
            }),
            view_filter_section("Force", &filter.force_list(), |_, e| {
                Msg::ForceFilterChanged(e)
            }),
            view_filter_section("Mechanic", &filter.mechanic_list(), |_, e| {
                Msg::MechanicFilterChanged(e)
            }),
            view_filter_section("Laterality", &filter.laterality_list(), |_, e| {
                Msg::LateralityFilterChanged(e)
            }),
            view_filter_section("Assistance", &filter.assistance_list(), |_, e| {
                Msg::AssistanceFilterChanged(e)
            }),
            view_filter_section("Equipment", &filter.equipment_list(), |_, e| {
                Msg::EquipmentFilterChanged(e)
            }),
            view_filter_section("Category", &filter.category_list(), |_, e| {
                Msg::CategoryFilterChanged(e)
            }),
            div![
                C!["field"],
                C!["is-grouped"],
                C!["is-grouped-centered"],
                div![
                    C!["control"],
                    button![
                        C!["button"],
                        C!["is-primary"],
                        &ev(Ev::Click, |_| Msg::CloseFilterDialog),
                        format!(
                            "Show {exercise_count} custom and {catalog_exercise_count} catalog exercises"
                        )
                    ]
                ],
            ],
        ],
        &ev(Ev::Click, |_| Msg::CloseFilterDialog),
    )
}

fn view_filter_section<T: domain::Property + 'static>(
    title: &str,
    filter: &[(T, bool)],
    filter_changed: impl FnOnce(web_sys::Event, T) -> Msg + 'static + Clone,
) -> Node<Msg> {
    div![
        C!["block"],
        label![C!["subtitle"], title],
        div![
            C!["container"],
            C!["py-3"],
            div![C!["tags"], view_filter_tags(filter, filter_changed, false)]
        ],
    ]
}

fn view_filter_tags<T: domain::Property + 'static>(
    filter: &[(T, bool)],
    filter_changed: impl FnOnce(web_sys::Event, T) -> Msg + 'static + Clone,
    show_enabled_only: bool,
) -> Vec<Node<Msg>> {
    filter
        .iter()
        .filter(|(_, enabled)| !show_enabled_only || *enabled)
        .map(|(element, enabled)| {
            let e = *element;
            let f = filter_changed.clone();
            span![
                C!["tag"],
                C!["is-hoverable"],
                IF![*enabled => C!["is-link"]],
                ev(Ev::Click, move |x| f(x, e)),
                &T::name(*element)
            ]
        })
        .collect::<Vec<_>>()
}
