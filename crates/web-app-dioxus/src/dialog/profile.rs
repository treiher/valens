use dioxus::prelude::*;

use valens_domain as domain;
use valens_domain::{SessionService, UserService};

use crate::{
    DOMAIN_SERVICE,
    diagnostics::log_failure,
    notification::notify,
    session::SessionRefresh,
    signal_changed_data,
    ui::{
        element::{Dialog, ErrorMessage, Loading, NoConnection, SaveDialog},
        form::{FieldValue, FieldValueState, InputField, SelectField, SelectOption},
    },
};

#[component]
pub fn ProfileDialog(on_close: EventHandler<MouseEvent>) -> Element {
    let user = use_resource(|| async { DOMAIN_SERVICE().get_session().await });
    match &*user.read() {
        Some(Ok(user)) => rsx! {
            ProfileForm { user: user.clone(), on_close }
        },
        Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))) => rsx! {
            Dialog {
                title: rsx! { "Profile" },
                on_close,
                NoConnection {}
            }
        },
        Some(Err(err)) => {
            log_failure("load profile", err);
            rsx! {
                Dialog {
                    title: rsx! { "Profile" },
                    on_close,
                    ErrorMessage { message: err }
                }
            }
        }
        None => rsx! {
            Dialog {
                title: rsx! { "Profile" },
                on_close,
                Loading {}
            }
        },
    }
}

#[component]
fn ProfileForm(user: domain::User, on_close: EventHandler<MouseEvent>) -> Element {
    let mut name = use_signal(|| FieldValue {
        input: user.name.to_string(),
        validated: Ok(user.name.clone()),
        orig: user.name.to_string(),
    });
    let mut sex = use_signal(|| FieldValue {
        input: user.sex.to_string(),
        validated: Ok(user.sex),
        orig: user.sex.to_string(),
    });
    let mut height = use_signal(|| FieldValue::from_option(user.height));
    let mut is_loading = use_signal(|| false);

    let user_id = user.id;
    let save = move |event: MouseEvent| async move {
        let mut saved = false;
        is_loading.set(true);
        if let (Ok(name), Ok(sex), Ok(height)) = (
            name.read().validated.clone(),
            sex.read().validated.clone(),
            height.read().validated.clone(),
        ) {
            match DOMAIN_SERVICE()
                .update_user(user_id, name, sex, height)
                .await
            {
                Ok(_) => {
                    saved = true;
                    signal_changed_data();
                    consume_context::<SessionRefresh>().refresh();
                }
                Err(err) => {
                    notify("Failed to edit profile", &err);
                }
            }
        }
        is_loading.set(false);
        if saved {
            on_close.call(event);
        }
    };

    rsx! {
        SaveDialog {
            title: rsx! { "Profile" },
            on_close,
            on_save: save,
            is_loading: is_loading(),
            disabled: (!name.read().changed() && !sex.read().changed() && !height.read().changed())
                || !name.read().valid() || !sex.read().valid() || !height.read().valid(),
            InputField {
                label: "Name".to_string(),
                "data-testid": "profile-name",
                value: name.read().input.clone(),
                error: if let Err(err) = &name.read().validated { err.clone() },
                has_changed: name.read().changed(),
                on_input: move |event: FormEvent| {
                    let mut name = name.write();
                    name.input = event.value();
                    name.validated = domain::Name::new(&name.input)
                        .map_err(|err| err.to_string());
                }
            }
            SelectField {
                label: "Sex".to_string(),
                options: vec![
                    rsx! {
                        SelectOption {
                            text: domain::Sex::FEMALE.to_string(),
                            value: domain::Sex::FEMALE.to_string(),
                            selected: matches!(sex.read().validated, Ok(domain::Sex::FEMALE)),
                        }
                    },
                    rsx! {
                        SelectOption {
                            text: domain::Sex::MALE.to_string(),
                            value: domain::Sex::MALE.to_string(),
                            selected: matches!(sex.read().validated, Ok(domain::Sex::MALE)),
                        }
                    },
                ],
                has_changed: sex.read().changed(),
                on_change: move |event: FormEvent| {
                    let mut sex = sex.write();
                    sex.input = event.value();
                    sex.validated = Ok(domain::Sex::from(sex.input.as_ref()));
                }
            }
            InputField {
                label: "Height".to_string(),
                "data-testid": "profile-height",
                right_icon: rsx! { "cm" },
                inputmode: "numeric".to_string(),
                value: height.read().input.clone(),
                error: if let Err(err) = &height.read().validated { err.clone() },
                has_changed: height.read().changed(),
                on_input: move |event: FormEvent| {
                    let mut height = height.write();
                    height.input = event.value();
                    height.validated = DOMAIN_SERVICE()
                        .validate_user_height(&height.input)
                        .map_err(|err| err.to_string());
                }
            }
        }
    }
}
