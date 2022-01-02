use seed::{prelude::*, *};

pub fn view_errors<Ms>(error_messages: &[String]) -> Vec<Node<Ms>> {
    error_messages
        .iter()
        .map(|message| {
            div![
                C!["message"],
                C!["is-danger"],
                C!["mx-2"],
                div![C!["message-body"], message],
            ]
        })
        .collect::<Vec<_>>()
}
