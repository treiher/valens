use dioxus::prelude::*;

#[component]
pub fn Field(label: String, control: Element) -> Element {
    rsx! {
        div {
            class: "field",
            label { class: "label", "{label}" }
            div {
                class: "control",
                {control}
            }
        }
    }
}

#[component]
pub fn InputField(
    label: String,
    help: Option<String>,
    left_icon: Option<Element>,
    right_icon: Option<Element>,
    r#type: Option<String>,
    inputmode: Option<String>,
    max: Option<String>,
    value: String,
    error: Option<String>,
    has_changed: bool,
    is_disabled: Option<bool>,
    oninput: EventHandler<FormEvent>,
) -> Element {
    let error = error.and_then(|error| if error.is_empty() { None } else { Some(error) });
    let has_error = error.is_some();
    rsx! {
        div {
            class: "field",
            label { class: "label", "{label}" }
            div {
                class: "control",
                class: if left_icon.is_some() { "has-icons-left" },
                class: if right_icon.is_some() { "has-icons-right" },
                input {
                    class: "input",
                    class: if has_error { "is-danger" },
                    class: if has_changed { "is-info" },
                    disabled: if let Some(is_disabled) = is_disabled { is_disabled },
                    r#type: if let Some(r#type) = r#type { r#type } else { "text" },
                    inputmode: if let Some(inputmode) = inputmode { inputmode },
                    max: if let Some(max) = max { max },
                    value: "{value}",
                    oninput: move |evt| oninput.call(evt),
                }
                if let Some(ref left_icon) = left_icon {
                    span {
                        class: "icon is-left",
                        {left_icon}
                    }
                }
                if let Some(ref right_icon) = right_icon {
                    span {
                        class: "icon is-right",
                        {right_icon}
                    }
                }
            }
            if let Some(ref error) = error {
                p { class: "help is-danger", "{error}" }
            } else if let Some(ref help) = help {
                p { class: "help", "{help}" }
            }
        }
    }
}

#[component]
pub fn FieldSet(legend: String, fields: Element) -> Element {
    rsx! {
        fieldset { class: "fieldset mb-4",
            legend { class: "has-text-centered", {legend} }
            {fields}
        }
    }
}

#[component]
pub fn SelectField(
    label: String,
    options: Vec<Element>,
    has_changed: bool,
    onchange: EventHandler<FormEvent>,
) -> Element {
    rsx! {
        div {
            class: "field",
            label { class: "label", "{label}" }
            div {
                class: "control",
                div {
                    class: "select",
                    select {
                        class: if has_changed { "has-text-info" },
                        onchange,
                        for option in options {
                            {option}
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SelectOption(text: String, value: String, selected: bool) -> Element {
    rsx! {
        option {
            selected,
            value,
            "{text}"
        }
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct FieldValue<T> {
    pub input: String,
    pub validated: Result<T, String>,
    pub orig: String,
}

impl<T> Default for FieldValue<T> {
    fn default() -> Self {
        Self {
            input: String::new(),
            validated: Err(String::new()),
            orig: String::new(),
        }
    }
}

impl<T: ToString> FieldValue<T> {
    pub fn new(value: T) -> Self {
        let value_string = value.to_string();
        Self {
            input: value_string.clone(),
            validated: Ok(value),
            orig: value_string,
        }
    }
}

impl<T: ToString> FieldValue<Option<T>> {
    pub fn from_option(value: Option<T>) -> Self {
        if let Some(value) = value {
            let value_string = value.to_string();
            Self {
                input: value_string.clone(),
                validated: Ok(Some(value)),
                orig: value_string,
            }
        } else {
            Self {
                input: String::new(),
                validated: Ok(None),
                orig: String::new(),
            }
        }
    }
}

impl FieldValue<()> {
    pub fn has_valid_changes(values: &[&dyn FieldValueState]) -> bool {
        values.iter().any(|v| v.changed()) && values.iter().all(|v| v.valid())
    }
}

pub trait FieldValueState {
    fn valid(&self) -> bool;
    fn changed(&self) -> bool;
}

impl<T> FieldValueState for FieldValue<T> {
    fn valid(&self) -> bool {
        self.validated.is_ok()
    }

    fn changed(&self) -> bool {
        self.input.trim() != self.orig.trim()
    }
}
