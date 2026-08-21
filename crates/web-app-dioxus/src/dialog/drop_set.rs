use dioxus::prelude::*;

use valens_domain as domain;

use crate::{
    DROP_SET_CALCULATOR,
    ui::{
        element::Dialog,
        form::{FieldValue, InputField, SelectField, SelectOption},
    },
};

#[component]
pub fn DropSetCalculator() -> Element {
    let state = DROP_SET_CALCULATOR.read().clone();
    let mut start_weight_input = use_signal(|| FieldValue::new(state.start_weight));
    let mut drop_percentage_input = use_signal(|| FieldValue::new(state.drop_percentage));

    let start_weight = state.start_weight;
    let drop_percentage = state.drop_percentage;
    let increment = state.increment;
    let weights = domain::drop_set_weights(start_weight, drop_percentage, increment);
    let dp = decimal_places(increment);

    rsx! {
        Dialog {
            on_close: move |_| {
                DROP_SET_CALCULATOR.write().visible = false;
            },
            div {
                class: "columns is-mobile",
                div {
                    class: "column",
                    InputField {
                        label: "Start".to_string(),
                        right_icon: rsx! { "kg" },
                        inputmode: "decimal",
                        value: start_weight_input.read().input.clone(),
                        error: if let Err(err) = &start_weight_input.read().validated { err.clone() },
                        has_changed: false,
                        "data-testid": "drop-set-start-weight",
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            let validated = domain::Weight::try_from(input.trim())
                                .map(f32::from)
                                .map_err(|err| err.to_string());
                            if let Ok(value) = &validated {
                                DROP_SET_CALCULATOR.write().start_weight = *value;
                            }
                            let mut fv = start_weight_input.write();
                            fv.input = input;
                            fv.validated = validated;
                        },
                    }
                }
                div {
                    class: "column",
                    InputField {
                        label: "Drop".to_string(),
                        right_icon: rsx! { "%" },
                        inputmode: "decimal",
                        value: drop_percentage_input.read().input.clone(),
                        error: if let Err(err) = &drop_percentage_input.read().validated { err.clone() },
                        has_changed: false,
                        "data-testid": "drop-set-drop-percentage",
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            let validated = parse_drop_percentage(input.trim());
                            if let Ok(value) = &validated {
                                DROP_SET_CALCULATOR.write().drop_percentage = *value;
                            }
                            let mut fv = drop_percentage_input.write();
                            fv.input = input;
                            fv.validated = validated;
                        },
                    }
                }
                div {
                    class: "column",
                    SelectField {
                        label: "Increment".to_string(),
                        options: DROP_SET_INCREMENT_PRESETS.iter().map(|preset| {
                            rsx! {
                                SelectOption {
                                    text: format!("{preset} kg"),
                                    value: preset.to_string(),
                                    selected: (preset - increment).abs() < 1e-4,
                                }
                            }
                        }).collect::<Vec<_>>(),
                        has_changed: false,
                        is_fullwidth: true,
                        "data-testid": "drop-set-increment",
                        on_change: move |event: FormEvent| {
                            if let Ok(value) = event.value().parse::<f32>() {
                                DROP_SET_CALCULATOR.write().increment = value;
                            }
                        },
                    }
                }
            }
            table {
                class: "table is-striped is-fullwidth",
                style: "white-space: nowrap",
                thead {
                    tr {
                        th { class: "has-text-right", "Nominal %" }
                        th { class: "has-text-right", "Actual %" }
                        th { class: "has-text-right", "Weight (kg)" }
                    }
                }
                tbody {
                    tr {
                        td { class: "has-text-right", "100.0" }
                        td { class: "has-text-right", "100.0" }
                        td { class: "has-text-right", { format!("{start_weight:.dp$}") } }
                    }
                    for (index, w) in weights.iter().enumerate() {
                        {
                            let drop_index = i32::try_from(index + 1).unwrap_or(i32::MAX);
                            let nominal: f32 =
                                100.0 * (1.0f32 - drop_percentage / 100.0).powi(drop_index);
                            let actual: f32 = if start_weight > 0.0 {
                                100.0 * w / start_weight
                            } else {
                                0.0
                            };
                            rsx! {
                                tr {
                                    td { class: "has-text-right", { format!("{nominal:.1}") } }
                                    td { class: "has-text-right", { format!("{actual:.1}") } }
                                    td { class: "has-text-right", { format!("{w:.dp$}") } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn parse_drop_percentage(input: &str) -> Result<f32, String> {
    let value: f32 = input
        .replace(',', ".")
        .parse()
        .map_err(|_| "drop must be a decimal".to_string())?;
    if !value.is_finite() || value <= 0.0 || value >= 100.0 {
        return Err("drop must be greater than 0 and less than 100 %".to_string());
    }
    Ok(value)
}

const DROP_SET_INCREMENT_PRESETS: &[f32] = &[0.25, 0.5, 1.0, 1.25, 2.0, 2.5, 3.75, 5.0, 10.0];

#[derive(Clone)]
pub struct DropSetCalculatorState {
    pub visible: bool,
    pub start_weight: f32,
    pub drop_percentage: f32,
    pub increment: f32,
}

impl DropSetCalculatorState {
    #[must_use]
    pub fn new(start_weight: f32, drop_percentage: f32, increment: f32) -> Self {
        Self {
            visible: false,
            start_weight,
            drop_percentage,
            increment,
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn decimal_places(increment: f32) -> usize {
    let hundredths = (increment * 100.0).round() as i64;
    if hundredths % 100 == 0 {
        0
    } else if hundredths % 10 == 0 {
        1
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::{decimal_places, parse_drop_percentage};

    #[test]
    fn decimal_places_matches_preset_precision() {
        assert_eq!(decimal_places(1.0), 0);
        assert_eq!(decimal_places(2.0), 0);
        assert_eq!(decimal_places(5.0), 0);
        assert_eq!(decimal_places(10.0), 0);
        assert_eq!(decimal_places(0.5), 1);
        assert_eq!(decimal_places(2.5), 1);
        assert_eq!(decimal_places(0.25), 2);
        assert_eq!(decimal_places(1.25), 2);
        assert_eq!(decimal_places(3.75), 2);
    }

    #[test]
    fn parse_drop_percentage_accepts_valid_values() {
        assert_eq!(parse_drop_percentage("20"), Ok(20.0));
        assert_eq!(parse_drop_percentage("12.5"), Ok(12.5));
        assert_eq!(parse_drop_percentage("0.1"), Ok(0.1));
        assert_eq!(parse_drop_percentage("99.9"), Ok(99.9));
    }

    #[test]
    fn parse_drop_percentage_accepts_comma_decimal() {
        assert_eq!(parse_drop_percentage("12,5"), Ok(12.5));
    }

    #[test]
    fn parse_drop_percentage_rejects_non_numeric() {
        assert!(parse_drop_percentage("").is_err());
        assert!(parse_drop_percentage("abc").is_err());
    }

    #[test]
    fn parse_drop_percentage_rejects_out_of_range() {
        assert!(parse_drop_percentage("0").is_err());
        assert!(parse_drop_percentage("-1").is_err());
        assert!(parse_drop_percentage("100").is_err());
        assert!(parse_drop_percentage("150").is_err());
    }

    #[test]
    fn parse_drop_percentage_rejects_non_finite() {
        assert!(parse_drop_percentage("nan").is_err());
        assert!(parse_drop_percentage("inf").is_err());
        assert!(parse_drop_percentage("-inf").is_err());
    }
}
