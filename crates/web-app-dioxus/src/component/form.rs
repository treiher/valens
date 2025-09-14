use dioxus::prelude::*;

#[component]
pub fn Field(children: Element, label: String) -> Element {
    rsx! {
        div {
            class: "field",
            label { class: "label", "{label}" }
            div {
                class: "control",
                {children}
            }
        }
    }
}

#[component]
pub fn InputField(
    label: Option<String>,
    help: Option<String>,
    left_icon: Option<Element>,
    right_icon: Option<Element>,
    r#type: Option<String>,
    inputmode: Option<String>,
    size: Option<usize>,
    min: Option<String>,
    max: Option<String>,
    step: Option<usize>,
    value: String,
    error: Option<String>,
    has_changed: bool,
    has_text_right: Option<bool>,
    is_disabled: Option<bool>,
    oninput: EventHandler<FormEvent>,
) -> Element {
    let error = error.and_then(|error| if error.is_empty() { None } else { Some(error) });
    let has_error = error.is_some();
    rsx! {
        div {
            class: "field",
            if let Some(label) = label { label { class: "label", "{label}" } }
            div {
                class: "control",
                class: if left_icon.is_some() { "has-icons-left" },
                class: if right_icon.is_some() { "has-icons-right" },
                input {
                    class: "input",
                    class: if has_error { "is-danger" },
                    class: if has_changed { "is-info" },
                    class: if has_text_right.unwrap_or_default() { "has-text-right" },
                    disabled: if let Some(is_disabled) = is_disabled { is_disabled },
                    r#type: if let Some(r#type) = r#type { r#type } else { "text" },
                    inputmode: if let Some(inputmode) = inputmode { inputmode },
                    size: if let Some(size) = size { size },
                    min: if let Some(min) = min { min },
                    max: if let Some(max) = max { max },
                    step: if let Some(step) = step { step },
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
pub fn FieldSet(children: Element, legend: String) -> Element {
    rsx! {
        fieldset { class: "fieldset mb-4",
            legend { class: "has-text-centered", {legend} }
            {children}
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

#[component]
pub fn ButtonSelectField<T: Clone + PartialEq + 'static>(
    label: String,
    options: Vec<ButtonSelectOption<T>>,
    selected: T,
    error: Option<String>,
    has_changed: bool,
    onclick: EventHandler<(MouseEvent, T)>,
) -> Element {
    let error = error.and_then(|error| if error.is_empty() { None } else { Some(error) });
    let has_error = error.is_some();
    rsx! {
        div {
            class: "field",
            label { class: "label", "{label}" }
            div {
                class: "field has-addons",
                for option in options {
                    div {
                        class: "control",
                        div {
                            class: "button",
                            class: if option.value == selected && has_error { "is-danger" },
                            class: if option.value == selected && !has_error { "is-link" },
                            class: if option.value != selected && has_changed { "is-link is-outlined" },
                            onclick: {
                                let value = option.value.clone();
                                move |event| {
                                    let value = value.clone();
                                    onclick((event, value));
                                }
                            },
                            {option.text}
                        }
                    }
                }
            }
            if let Some(ref error) = error {
                p { class: "help is-danger", "{error}" }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct ButtonSelectOption<T> {
    pub text: String,
    pub value: T,
}

#[derive(Debug, Clone, PartialEq)]
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

impl<T: Default + PartialEq + ToString> FieldValue<T> {
    pub fn new_with_empty_default(value: T) -> Self {
        let value_string = if value == T::default() {
            String::new()
        } else {
            value.to_string()
        };
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

pub struct MultiToggle {
    pub states: Vec<(String, u8)>,
    pub num_states: u8,
}

#[component]
pub fn MultiToggleTags(multi_toggle: Signal<MultiToggle>) -> Element {
    let tags = &*multi_toggle
        .read()
        .states
        .iter()
        .enumerate()
        .map(|(i, (name, state))| {
            rsx! {
                span {
                    class: "tag is-hoverable",
                    class: if *state == 1 { "is-link" },
                    class: if *state == 2 { "is-dark" },
                    onclick: move |_| {
                        let m = multi_toggle.read().num_states;
                        let s = multi_toggle.read().states[i].1;
                        multi_toggle.write().states[i].1 = (s + 1) % m;
                    },
                    "{name}"
                }
            }
        })
        .collect::<Vec<_>>();
    rsx! {
        div {
            class: "tags",
            for tag in tags {
                {tag}
            }
        }
    }
}
