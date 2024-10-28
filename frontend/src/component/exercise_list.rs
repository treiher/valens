use seed::{prelude::*, *};

use crate::{common, data, domain};

// ------ ------
//     Model
// ------ ------

#[allow(clippy::struct_excessive_bools)]
pub struct Model {
    search_term: String,
    filter: domain::ExerciseFilter,
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
            search_term: String::new(),
            filter: domain::ExerciseFilter::default(),
            view_create,
            view_edit,
            view_delete,
            search_bar_padding,
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
    CreateClicked(String),
    EditClicked(u32),
    DeleteClicked(u32),
}

pub enum OutMsg {
    None,
    Selected(u32),
    CreateClicked(String),
    EditClicked(u32),
    DeleteClicked(u32),
}

pub fn update(msg: Msg, model: &mut Model, _orders: &mut impl Orders<Msg>) -> OutMsg {
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

pub fn view(model: &Model, loading: bool, data_model: &data::Model) -> Vec<Node<Msg>> {
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

    nodes![
        div![
            C!["field"],
            C!["is-grouped"],
            C![IF![model.search_bar_padding => "px-4"]],
            common::view_search_box(&model.search_term, Msg::SearchTermChanged),
            if model.view_create {
                let disabled = loading
                    || model.search_term.is_empty()
                    || exercises
                        .iter()
                        .any(|e| e.name == *model.search_term.trim());
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
                            let search_term = model.search_term.clone();
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
        ],
    ]
}
