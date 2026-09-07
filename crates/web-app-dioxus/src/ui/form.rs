//! Generic, domain-agnostic form components.

use dioxus::{prelude::*, web::WebEventExt};
use web_sys::wasm_bindgen::JsCast;

use crate::ui::capitalized;

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
    max: Option<String>,
    value: String,
    error: Option<String>,
    error_testid: Option<String>,
    has_changed: bool,
    #[props(default)] has_text_right: bool,
    #[props(default)] is_disabled: bool,
    #[props(default)] autofocus: bool,
    on_input: EventHandler<FormEvent>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let error = error.and_then(|error| {
        if error.is_empty() {
            None
        } else {
            Some(capitalized(&error))
        }
    });
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
                    class: if has_text_right { "has-text-right" },
                    disabled: is_disabled,
                    r#type: if let Some(r#type) = r#type { r#type } else { "text" },
                    inputmode: if let Some(inputmode) = inputmode { inputmode },
                    max: if let Some(max) = max { max },
                    value: "{value}",
                    oninput: on_input,
                    onmounted: move |event| async move {
                        if autofocus {
                            let _ = event.set_focus(true).await;
                            if let Some(element) = event.data().try_as_web_event()
                                && let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>()
                            {
                                input.select();
                            }
                        }
                    },
                    ..attributes,
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
                p {
                    class: "help is-danger",
                    "data-testid": if let Some(ref error_testid) = error_testid { error_testid.clone() },
                    "{error}"
                }
            } else if let Some(ref help) = help {
                p { class: "help", "{help}" }
            }
        }
    }
}

/// A row of numeric fields, one per phase of a tempo, with the unit inside the last field.
///
/// A trailing empty field means the phase is absent, an empty field before a filled one a phase of
/// zero seconds.
#[component]
pub fn PhaseFields(
    label: String,
    help: Option<String>,
    unit: String,
    phases: Vec<String>,
    /// Error of the field at the same index.
    field_errors: Vec<Option<String>>,
    /// Error of the row as a whole.
    error: Option<String>,
    has_changed: bool,
    field_testid: String,
    on_input: EventHandler<(usize, String)>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let last_index = phases.len().saturating_sub(1);
    // The row error stands for the tempo the fields make up, so a field error is shown only while
    // there is none.
    let has_row_error = error.as_ref().is_some_and(|error| !error.is_empty());
    let error = error
        .or_else(|| field_errors.iter().flatten().next().cloned())
        .filter(|error| !error.is_empty())
        .map(|error| capitalized(&error));
    rsx! {
        div {
            class: "field",
            ..attributes,
            label { class: "label", "{label}" }
            div {
                class: "field has-addons is-align-items-center mb-0",
                for (index, phase) in phases.iter().enumerate() {
                    div {
                        class: "control is-expanded",
                        class: if index == last_index { "has-icons-right" },
                        input {
                            class: "input has-text-right",
                            class: if field_errors.get(index).is_some_and(Option::is_some) || has_row_error { "is-danger" },
                            class: if has_changed { "is-info" },
                            inputmode: "numeric",
                            value: "{phase}",
                            oninput: move |event: FormEvent| on_input.call((index, event.value())),
                            "data-testid": "{field_testid}-{index}",
                        }
                        if index == last_index {
                            span { class: "icon is-right", "{unit}" }
                        }
                    }
                }
            }
            if let Some(ref error) = error {
                p {
                    class: "help is-danger",
                    "data-testid": "{field_testid}-error",
                    "{error}"
                }
            } else if let Some(ref help) = help {
                p { class: "help", "{help}" }
            }
        }
    }
}

#[component]
pub fn TextAreaField(
    value: String,
    has_changed: bool,
    /// Focus the text area on mount and place the caret at the end of the text.
    #[props(default)]
    autofocus: bool,
    on_input: EventHandler<FormEvent>,
    on_mounted: Option<EventHandler<web_sys::HtmlTextAreaElement>>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    rsx! {
        div {
            class: "field",
            div {
                class: "control",
                textarea {
                    class: "textarea",
                    class: if has_changed { "is-info" },
                    oninput: on_input,
                    onmounted: move |event: MountedEvent| async move {
                        if autofocus {
                            let _ = event.set_focus(true).await;
                        }
                        if let Some(element) = event.data().try_as_web_event()
                            && let Some(textarea) = element.dyn_ref::<web_sys::HtmlTextAreaElement>()
                        {
                            if autofocus
                                && let Ok(len) = u32::try_from(textarea.value().encode_utf16().count())
                            {
                                let _ = textarea.set_selection_range(len, len);
                            }
                            if let Some(on_mounted) = on_mounted {
                                on_mounted.call(textarea.clone());
                            }
                        }
                    },
                    ..attributes,
                    { value }
                }
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
    #[props(default)] is_fullwidth: bool,
    on_change: EventHandler<FormEvent>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    rsx! {
        div {
            class: "field",
            label { class: "label", "{label}" }
            div {
                class: "control",
                div {
                    class: "select",
                    class: if is_fullwidth { "is-fullwidth" },
                    select {
                        class: if has_changed { "has-text-info" },
                        onchange: on_change,
                        ..attributes,
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
    /// Distribute the options over the full width, wrapping labels that do not fit.
    #[props(default)]
    is_expanded: bool,
    on_click: EventHandler<(MouseEvent, T)>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let error = error.and_then(|error| {
        if error.is_empty() {
            None
        } else {
            Some(capitalized(&error))
        }
    });
    let has_error = error.is_some();
    rsx! {
        div {
            class: "field",
            ..attributes,
            label { class: "label", "{label}" }
            div {
                class: "field has-addons",
                for option in options {
                    div {
                        class: "control",
                        class: if is_expanded { "is-expanded" },
                        div {
                            class: "button",
                            class: if is_expanded { "is-fullwidth is-wrapping" },
                            class: if option.value == selected && has_error { "is-danger" },
                            class: if option.value == selected && !has_error { "is-link" },
                            class: if option.value != selected && has_changed { "is-link is-outlined" },
                            onclick: {
                                let value = option.value.clone();
                                move |event| {
                                    let value = value.clone();
                                    on_click((event, value));
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
    pub states: Vec<(String, usize)>,
    /// Tag class of every state but the zeroth, in the order the states are cycled through.
    pub classes: Vec<&'static str>,
}

impl MultiToggle {
    /// Cycles the state of the given entry, wrapping around to the zeroth state.
    fn advance(&mut self, index: usize) {
        let num_states = self.classes.len() + 1;
        self.states[index].1 = (self.states[index].1 + 1) % num_states;
    }

    fn class(&self, state: usize) -> &'static str {
        state
            .checked_sub(1)
            .and_then(|i| self.classes.get(i))
            .copied()
            .unwrap_or_default()
    }
}

#[component]
pub fn MultiToggleTags(multi_toggle: Signal<MultiToggle>) -> Element {
    let tags = &*multi_toggle
        .read()
        .states
        .iter()
        .enumerate()
        .map(|(i, (name, state))| {
            let class = multi_toggle.read().class(*state);
            rsx! {
                span {
                    class: "tag is-hoverable {class}",
                    "data-testid": "multi-toggle-tag",
                    onclick: move |_| {
                        multi_toggle.write().advance(i);
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{attribute_of, contains, render, text_of};

    use super::*;

    fn render_phase_fields(field_errors: Vec<Option<String>>, error: Option<String>) -> String {
        render(move || {
            let field_errors = field_errors.clone();
            let error = error.clone();
            rsx! {
                PhaseFields {
                    label: "Tempo",
                    help: "Optionally split into the phases of a tempo",
                    unit: "s",
                    phases: vec!["1000".to_string(), String::new()],
                    field_errors,
                    error,
                    has_changed: false,
                    field_testid: "input-tempo",
                    on_input: move |_| {},
                }
            }
        })
    }

    #[test]
    fn test_phase_fields_show_the_error_of_a_field() {
        let html = render_phase_fields(
            vec![Some("time must not be longer than 999 s".into()), None],
            None,
        );

        assert_eq!(
            text_of(&html, "input-tempo-error"),
            "Time must not be longer than 999 s"
        );
    }

    #[test]
    fn test_phase_fields_show_the_error_of_the_row_before_the_error_of_a_field() {
        let html = render_phase_fields(
            vec![Some("time must not be longer than 999 s".into()), None],
            Some("tempo must not be longer than 999 s".into()),
        );

        assert_eq!(
            text_of(&html, "input-tempo-error"),
            "Tempo must not be longer than 999 s"
        );
    }

    #[test]
    fn test_phase_fields_mark_only_the_field_its_error_belongs_to() {
        let html = render_phase_fields(
            vec![Some("time must not be longer than 999 s".into()), None],
            None,
        );

        assert!(
            attribute_of(&html, "input-tempo-0", "class").contains("is-danger"),
            "{html}"
        );
        assert!(
            !attribute_of(&html, "input-tempo-1", "class").contains("is-danger"),
            "{html}"
        );
    }

    #[test]
    fn test_phase_fields_mark_every_field_of_a_row_in_error() {
        let html = render_phase_fields(
            vec![None, None],
            Some("tempo must not be longer than 999 s".into()),
        );

        assert!(
            attribute_of(&html, "input-tempo-1", "class").contains("is-danger"),
            "{html}"
        );
    }

    #[test]
    fn test_phase_fields_show_no_error_without_one() {
        let html = render_phase_fields(vec![None, None], None);

        assert!(!contains(&html, "input-tempo-error"), "{html}");
    }

    #[test]
    fn test_phase_fields_show_the_help_without_an_error() {
        let html = render_phase_fields(vec![None, None], None);

        assert!(
            html.contains("Optionally split into the phases of a tempo"),
            "{html}"
        );
    }

    #[test]
    fn test_phase_fields_replace_the_help_by_an_error() {
        let html = render_phase_fields(
            vec![None, None],
            Some("tempo must not be longer than 999 s".into()),
        );

        assert!(
            !html.contains("Optionally split into the phases of a tempo"),
            "{html}"
        );
    }

    #[test]
    fn test_field_value_new() {
        let value = FieldValue::new(1);

        assert!(value.valid());
        assert!(!value.changed());
        assert_eq!(value.input, "1");
    }

    #[test]
    fn test_field_value_new_with_empty_default() {
        let value = FieldValue::new_with_empty_default(0);

        assert!(value.valid());
        assert!(!value.changed());
        assert_eq!(value.input, "");
    }

    #[test]
    fn test_field_value_new_with_empty_default_of_non_default() {
        assert_eq!(FieldValue::new_with_empty_default(1).input, "1");
    }

    #[test]
    fn test_field_value_from_option() {
        let value = FieldValue::from_option(Some(1));

        assert!(value.valid());
        assert!(!value.changed());
        assert_eq!(value.input, "1");
    }

    #[test]
    fn test_field_value_from_option_of_none() {
        let value = FieldValue::from_option(None::<i32>);

        assert!(value.valid());
        assert!(!value.changed());
        assert_eq!(value.input, "");
    }

    #[test]
    fn test_field_value_default_is_invalid() {
        assert!(!FieldValue::<i32>::default().valid());
    }

    #[test]
    fn test_field_value_changed_ignores_surrounding_whitespace() {
        let value = FieldValue::<i32> {
            input: " 1 ".to_string(),
            validated: Ok(1),
            orig: "1".to_string(),
        };

        assert!(!value.changed());
    }

    #[test]
    fn test_has_valid_changes_without_changes() {
        assert!(!FieldValue::has_valid_changes(&[
            &FieldValue::new(1),
            &FieldValue::new(2)
        ]));
    }

    #[test]
    fn test_has_valid_changes_with_a_valid_change() {
        assert!(FieldValue::has_valid_changes(&[
            &changed(1),
            &FieldValue::new(2)
        ]));
    }

    #[test]
    fn test_has_valid_changes_with_an_invalid_field() {
        assert!(!FieldValue::has_valid_changes(&[
            &changed(1),
            &FieldValue::<i32>::default()
        ]));
    }

    #[test]
    fn test_multi_toggle_advance_cycles_through_all_states() {
        let mut multi_toggle = multi_toggle();

        for expected in [1, 2, 0, 1] {
            multi_toggle.advance(0);
            assert_eq!(multi_toggle.states[0].1, expected);
        }
    }

    #[test]
    fn test_multi_toggle_advance_affects_only_the_given_entry() {
        let mut multi_toggle = multi_toggle();

        multi_toggle.advance(1);

        assert_eq!(
            multi_toggle.states,
            vec![("A".to_string(), 0), ("B".to_string(), 1)]
        );
    }

    #[test]
    fn test_multi_toggle_class() {
        let multi_toggle = multi_toggle();

        assert_eq!(multi_toggle.class(0), "");
        assert_eq!(multi_toggle.class(1), "is-dark");
        assert_eq!(multi_toggle.class(2), "is-link");
        assert_eq!(multi_toggle.class(3), "");
    }

    fn changed(value: i32) -> FieldValue<i32> {
        FieldValue {
            input: value.to_string(),
            validated: Ok(value),
            orig: String::new(),
        }
    }

    fn multi_toggle() -> MultiToggle {
        MultiToggle {
            states: vec![("A".to_string(), 0), ("B".to_string(), 0)],
            classes: vec!["is-dark", "is-link"],
        }
    }
}
