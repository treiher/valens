use seed::{prelude::*, *};
use valens_domain::{self as domain, Property};

use crate::{common, data};

// ------ ------
//     Init
// ------ ------

pub fn init(
    mut url: Url,
    _orders: &mut impl Orders<Msg>,
    _data_model: &data::Model,
    navbar: &mut crate::Navbar,
) -> Model {
    let id = url.next_hash_path_part().unwrap_or_default().to_string();

    navbar.title = String::from("Catalog Exercise");

    Model { name: id }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    name: String,
}

// ------ ------
//    Update
// ------ ------

#[derive(Copy, Clone)]
pub enum Msg {}

pub fn update(
    _msg: Msg,
    _model: &mut Model,
    _data_model: &data::Model,
    _orders: &mut impl Orders<Msg>,
) {
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model, _data_model: &data::Model) -> Node<Msg> {
    let exercises = domain::ExerciseFilter::default().catalog();
    if let Some(exercise) = exercises.get(&*model.name) {
        div![
            div![
                C!["mx-2"],
                C!["mb-5"],
                common::view_title(&span![&exercise.name], 0),
            ],
            view_exercise_tags(
                exercise.force,
                exercise.mechanic,
                exercise.laterality,
                exercise.assistance,
                exercise.equipment,
                exercise.muscles,
                exercise.category
            )
        ]
    } else {
        common::view_error_not_found("Exercise")
    }
}

fn view_exercise_tags(
    force: domain::Force,
    mechanic: domain::Mechanic,
    laterality: domain::Laterality,
    assistance: domain::Assistance,
    equipment: &[domain::Equipment],
    muscles: &[(domain::Muscle, domain::MuscleStimulus)],
    category: domain::Category,
) -> Vec<Node<Msg>> {
    nodes![
        div![
            C!["tags"],
            C!["is-centered"],
            C!["m-2"],
            span![C!["tag"], force.name()],
            span![C!["tag"], mechanic.name()],
            span![C!["tag"], laterality.name()],
            span![C!["tag"], assistance.name()],
            span![C!["tag"], category.name()],
        ],
        div![
            C!["tags"],
            C!["is-centered"],
            C!["m-2"],
            muscles.iter().map(|(m, stimulus)| {
                common::view_element_with_description(
                    span![
                        C!["tag"],
                        C!["is-link"],
                        C![IF![*stimulus == domain::MuscleStimulus::Secondary => "is-light"]],
                        m.name()
                    ],
                    m.description(),
                )
            }),
        ],
        div![
            C!["tags"],
            C!["is-centered"],
            C!["m-2"],
            equipment.iter().map(|e| { span![C!["tag"], e.name()] }),
        ],
    ]
}
