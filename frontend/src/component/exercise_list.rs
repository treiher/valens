use seed::{prelude::*, *};

use crate::{catalog, common, data, domain};

// ------ ------
//     Model
// ------ ------

pub struct Model {
    search_term: String,
    filter: domain::ExerciseFilter,
    loading: bool, // TODO: Move loading into view()?
    view_create: bool,
    view_edit: bool,
    view_delete: bool,
}

impl Model {
    pub fn new(view_create: bool, view_edit: bool, view_delete: bool) -> Self {
        Self {
            search_term: String::new(),
            filter: domain::ExerciseFilter::default(),
            loading: false,
            view_create,
            view_edit,
            view_delete,
        }
    }
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    SearchTermChanged(String),
    FilterChanged(domain::Muscle),

    Selected(u32),
    CreateClicked(web_sys::Event),
    EditClicked(u32),
    DeleteClicked(u32),
}

pub enum OutMsg {
    None,
    Selected(u32),
    CreateClicked(web_sys::Event),
    EditClicked(u32),
    DeleteClicked(u32),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) -> OutMsg {
    match msg {
        Msg::SearchTermChanged(search_term) => {
            model.search_term = search_term;
            OutMsg::None
        }
        Msg::FilterChanged(muscle) => {
            if model.filter.muscles.contains(&muscle) {
                model.filter.muscles.remove(&muscle);
            } else {
                model.filter.muscles.insert(muscle);
            }
            OutMsg::None
        }

        Msg::Selected(exercise_id) => OutMsg::Selected(exercise_id),
        Msg::CreateClicked(exercise_id) => OutMsg::CreateClicked(exercise_id),
        Msg::EditClicked(exercise_id) => OutMsg::EditClicked(exercise_id),
        Msg::DeleteClicked(exercise_id) => OutMsg::DeleteClicked(exercise_id),
    }
}

// ------ ------
//     View
// ------ ------

//pub fn view_exercises_with_search<Ms>(
//    exercises: &BTreeMap<u32, data::Exercise>,
//    search_term: &str,
//    search_term_changed: impl FnOnce(String) -> Ms + 'static + Clone,
//    filter: &domain::ExerciseFilter,
//    filter_changed: impl FnOnce(domain::Muscle) -> Ms + 'static + Clone,
//    create_exercise: Option<impl FnOnce(web_sys::Event) -> Ms + 'static + Clone>,
//    loading: bool,
//    selected: impl FnOnce(u32) -> Ms + 'static + Clone,
//    edit: &Option<impl FnOnce(u32) -> Ms + 'static + Clone>,
//    delete: &Option<impl FnOnce(u32) -> Ms + 'static + Clone>,
//) -> Vec<Node<Ms>>
//where
//    Ms: 'static,
//{

pub fn view(model: &Model, data_model: &data::Model) -> Vec<Node<Msg>> {
    let muscle_filter = domain::Muscle::iter()
        .map(|m| (m, model.filter.muscles.contains(m)))
        .collect::<Vec<_>>();

    let exercises = data_model.exercises(&model.filter);
    let mut exercises = exercises
        .iter()
        .filter(|e| {
            e.name
                .to_lowercase()
                .contains(model.search_term.to_lowercase().trim())
        })
        .collect::<Vec<_>>();
    exercises.sort_by(|a, b| a.name.cmp(&b.name));

    let catalog_exercises = catalog::exercises(&model.filter);
    let mut catalog_exercises = catalog_exercises
        .iter()
        .filter(|e| {
            e.name
                .to_lowercase()
                .contains(model.search_term.to_lowercase().trim())
        })
        .collect::<Vec<_>>();
    catalog_exercises.sort_by(|a, b| a.name.cmp(b.name));

    nodes![
        div![
            C!["field"],
            C!["is-grouped"],
            //C![IF![edit.is_some() || delete.is_some() => "px-4"]],
            common::view_search_box(&model.search_term, Msg::SearchTermChanged),
            div![
                C!["control"],
                button![
                    C!["button"],
                    C![IF![!model.filter.is_empty() => "is-link"]],
                    //ev(Ev::Click, view_filter_dialog),
                    span![C!["icon"], i![C!["fas fa-filter"]]]
                ]
            ],
            if model.view_create {
                let disabled = model.loading
                    || model.search_term.is_empty()
                    || exercises
                        .iter()
                        .any(|e| e.name == *model.search_term.trim());
                div![
                    C!["control"],
                    button![
                        C!["button"],
                        C!["is-link"],
                        C![IF![model.loading => "is-loading"]],
                        attrs! {
                            At::Disabled => disabled.as_at_value()
                        },
                        ev(Ev::Click, Msg::CreateClicked),
                        span![C!["icon"], i![C!["fas fa-plus"]]]
                    ]
                ]
            } else {
                empty![]
            }
        ],
        div![
            C!["is-flex"],
            //C![IF![edit.is_some() || delete.is_some() => "px-4"]],
            div![C!["mr-1"], span![C!["icon"], i![C!["fas fa-filter"]]]],
            div![
                C!["tags"],
                C!["is-flex-wrap-nowrap"],
                C!["is-overflow-scroll"],
                C!["is-scrollbar-width-none"],
                muscle_filter.iter().map(|(muscle, enabled)| {
                    span![
                        C!["tag"],
                        C!["is-hoverable"],
                        IF![*enabled => C!["is-link"]],
                        ev(Ev::Click, {
                            let muscle = *muscle;
                            move |_| Msg::FilterChanged(*muscle)
                        }),
                        &domain::Muscle::name(**muscle)
                    ]
                })
            ],
        ],
        div![
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
                        a![
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
        ],
        IF![!catalog_exercises.is_empty() => common::view_title(&span!["Exercise catalog"], 3)],
        div![
            C!["table-container"],
            C!["mt-2"],
            table![
                C!["table"],
                C!["is-fullwidth"],
                C!["is-hoverable"],
                tbody![catalog_exercises.iter().map(|e| {
                    tr![td![
                        C!["is-flex"],
                        C!["is-justify-content-space-between"],
                        C!["has-text-link"],
                        span![
                            // ev(Ev::Click, {
                            //     let exercise_id = i;
                            //     let selected = selected.clone();
                            //     move |_| selected(exercise_id)
                            // }),
                            e.name.to_string(),
                        ],
                        p![
                            C!["is-flex is-flex-wrap-nowrap"],
                            span![
                                C!["icon"],
                                C!["mr-1"],
                                // ev(Ev::Click, {
                                //     let exercise_id = e.id;
                                //     let edit = edit.clone();
                                //     move |_| edit(exercise_id)
                                // }),
                                i![C!["fas fa-plus"]]
                            ]
                        ]
                    ]]
                })],
            ]
        ]
    ]
}
