use dioxus::prelude::*;

use valens_domain as domain;

use crate::{
    ONE_REP_MAX_CALCULATOR,
    ui::{
        element::Dialog,
        form::{FieldValue, InputField},
    },
};

#[component]
pub fn OneRepMaxCalculator() -> Element {
    let initial = ONE_REP_MAX_CALCULATOR.read().clone();
    let mut reps_input = use_signal(|| FieldValue::new(initial.reps));
    let mut weight_input = use_signal(|| FieldValue::new(initial.weight));

    let reps = ONE_REP_MAX_CALCULATOR.read().reps;
    let weight = ONE_REP_MAX_CALCULATOR.read().weight;
    #[allow(clippy::cast_precision_loss)]
    let one_rep_max = domain::one_rep_max(reps as f32, weight);
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let table_rows: Vec<(u32, u32, f32)> = (50u32..=100)
        .rev()
        .step_by(5)
        .map(|p| {
            (
                p,
                domain::reps_for_percentage(p as f32).round() as u32,
                p as f32 / 100.0 * one_rep_max,
            )
        })
        .collect();

    rsx! {
        Dialog {
            on_close: move |_| {
                ONE_REP_MAX_CALCULATOR.write().visible = false;
            },
            div {
                class: "columns is-mobile",
                div {
                    class: "column",
                    InputField {
                        label: "Reps".to_string(),
                        inputmode: "numeric",
                        value: reps_input.read().input.clone(),
                        error: if let Err(err) = &reps_input.read().validated { err.clone() },
                        has_changed: false,
                        "data-testid": "1rm-reps",
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            let validated = domain::Reps::try_from(input.trim())
                                .map(u32::from)
                                .map_err(|err| err.to_string());
                            if let Ok(value) = &validated {
                                ONE_REP_MAX_CALCULATOR.write().reps = *value;
                            }
                            let mut fv = reps_input.write();
                            fv.input = input;
                            fv.validated = validated;
                        },
                    }
                }
                div {
                    class: "column",
                    InputField {
                        label: "Weight".to_string(),
                        right_icon: rsx! { "kg" },
                        inputmode: "decimal",
                        value: weight_input.read().input.clone(),
                        error: if let Err(err) = &weight_input.read().validated { err.clone() },
                        has_changed: false,
                        "data-testid": "1rm-weight",
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            let validated = domain::Weight::try_from(input.trim())
                                .map(f32::from)
                                .map_err(|err| err.to_string());
                            if let Ok(value) = &validated {
                                ONE_REP_MAX_CALCULATOR.write().weight = *value;
                            }
                            let mut fv = weight_input.write();
                            fv.input = input;
                            fv.validated = validated;
                        },
                    }
                }
            }
            table {
                class: "table is-striped is-fullwidth",
                style: "white-space: nowrap",
                thead {
                    tr {
                        th { class: "has-text-right", "% 1RM" }
                        th { class: "has-text-right", "Reps" }
                        th { class: "has-text-right", "Weight (kg)" }
                    }
                }
                tbody {
                    for (percentage, row_reps, row_weight) in &table_rows {
                        tr {
                            td { class: "has-text-right", "{percentage}" }
                            td { class: "has-text-right", "{row_reps}" }
                            td { class: "has-text-right", "{row_weight:.2}" }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct OneRepMaxCalculatorState {
    pub visible: bool,
    pub reps: u32,
    pub weight: f32,
}

impl OneRepMaxCalculatorState {
    #[must_use]
    pub fn new(reps: u32, weight: f32) -> Self {
        Self {
            visible: false,
            reps,
            weight,
        }
    }
}
