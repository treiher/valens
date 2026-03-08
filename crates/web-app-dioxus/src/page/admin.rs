use dioxus::prelude::*;

use valens_domain as domain;
use valens_domain::UserService;
use valens_web_app::log::Service;

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE, NOTIFICATIONS, WEB_APP_SERVICE, signal_changed_data,
    ui::{
        element::{
            Block, CenteredBlock, Color, DeleteConfirmationDialog, Dialog, Error, ErrorMessage,
            Icon, Loading, MenuOption, Message, NoConnection, OptionsMenu, Table, Title,
        },
        form::{FieldValue, FieldValueState, InputField, SelectField, SelectOption},
    },
    update::{UPDATE_STATUS, UpdateStatus, VersionInfo, check_for_updates},
};

#[component]
pub fn Admin() -> Element {
    rsx! {
        Users {}
        Version {}
        Log {}
    }
}

#[component]
pub fn Users() -> Element {
    let users = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE().get_users().await
    });
    let mut dialog = use_signal(|| UserDialog::None);
    let mut is_loading = use_signal(|| false);

    macro_rules! is_loading {
        ($block:expr) => {{
            is_loading.set(true);
            $block;
            is_loading.set(false);
        }};
    }

    let mut close_dialog = move || {
        dialog.set(UserDialog::None);
    };

    let save = move |_| async move {
        let mut saved = false;
        is_loading! {
            match &*dialog.read() {
                UserDialog::Add { name, sex } => {
                    if let (Ok(name), Ok(sex)) = (name.validated.clone(), sex.validated.clone()) {
                        match DOMAIN_SERVICE().create_user(name, sex).await {
                            Ok(_) => {
                                saved = true;
                                signal_changed_data();
                            },
                            Err(err) => {
                                NOTIFICATIONS.write().push(format!("Failed to add user: {err}"));
                            }
                        }
                    }
                },
                UserDialog::Edit { id, name, sex } => {
                    if let (Ok(name), Ok(sex)) = (name.validated.clone(), sex.validated.clone()) {
                        let id = *id;
                        match DOMAIN_SERVICE().replace_user(domain::User { id, name, sex }).await {
                            Ok(_) => {
                                saved = true;
                                signal_changed_data();
                            },
                            Err(err) => {
                                NOTIFICATIONS.write().push(format!("Failed to edit user: {err}"));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if saved {
            close_dialog();
        }
    };
    let delete = move |_| async move {
        let mut deleted = false;
        if let UserDialog::Delete(user) = &*dialog.read() {
            is_loading! {
                match DOMAIN_SERVICE().delete_user(user.id).await {
                    Ok(()) => {
                        deleted = true;
                        signal_changed_data();
                    },
                    Err(err) => NOTIFICATIONS.write().push(format!("Failed to delete user: {err}"))

                }
            }
        }
        if deleted {
            close_dialog();
        }
    };
    let close = move |_| close_dialog();

    rsx! {
        Title { "Users" }
        match &*users.read() {
            Some(Ok(users)) => {
                rsx! {
                    Table {
                        head: vec![rsx! { "Name" }, rsx! { "Sex" }, rsx! {}],
                        body: users.iter().map(|user| {
                            let user = user.clone();
                            vec![
                                rsx! { "{user.name}" },
                                rsx! { "{user.sex}" },
                                rsx! {
                                    a {
                                        class: "mx-2",
                                        onclick: move |_| { *dialog.write() = UserDialog::Options(user.clone()); },
                                        Icon { name: "ellipsis-vertical"}
                                    }
                                }
                            ]
                        }).collect::<Vec<_>>()
                    }
                    CenteredBlock {
                        button {
                            class: "button is-link",
                            onclick: move |_| {
                                *dialog.write() = UserDialog::Add {
                                    name: FieldValue::default(),
                                    sex: FieldValue::new(domain::Sex::MALE),
                                };
                            },
                            Icon { name: "user-plus" }
                        }
                    }
                }
            }
            Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))) => {
                rsx! {
                    NoConnection {}
                }
            }
            Some(Err(err)) => rsx! {
                ErrorMessage { message: err }
            },
            None => rsx! {
                Loading {}
            },
        }
        match &*dialog.read() {
            UserDialog::None => rsx! {},
            UserDialog::Options(user) => {
                let user_edit = user.clone();
                let user_delete = user.clone();
                rsx! {
                    OptionsMenu {
                        options: vec![
                            rsx! {
                                MenuOption {
                                    icon: "user-edit".to_string(),
                                    text: "Edit user".to_string(),
                                    onclick: move |_| {
                                        *dialog.write() = UserDialog::Edit {
                                            id: user_edit.id,
                                            name: FieldValue {
                                                input: user_edit.name.to_string(),
                                                validated: Ok(user_edit.name.clone()),
                                                orig: user_edit.name.to_string()
                                            },
                                            sex: FieldValue {
                                                input: user_edit.sex.to_string(),
                                                validated: Ok(user_edit.sex),
                                                orig: user_edit.sex.to_string()
                                            }
                                        };
                                    }
                                },
                                MenuOption {
                                    icon: "user-times".to_string(),
                                    text: "Delete user".to_string(),
                                    onclick: move |_| { *dialog.write() = UserDialog::Delete(user_delete.clone()); }
                                },
                            },
                        ],
                        close_event: move |_| *dialog.write() = UserDialog::None,
                    }
                }
            },
            UserDialog::Add { name, sex } | UserDialog::Edit { name, sex, .. } => rsx! {
                Dialog {
                    title: rsx! { if let UserDialog::Add { .. } = &*dialog.read() { "Add user" } else { "Edit user" } },
                    close_event: close,
                    InputField {
                        label: "Name".to_string(),
                        value: name.input.clone(),
                        error: if let Err(err) = &name.validated { err.clone() },
                        has_changed: name.changed(),
                        oninput: move |event: FormEvent| {
                            let input = event.value();
                            dialog.with_mut(|dlg| {
                                match dlg {
                                    UserDialog::Add { name, .. }
                                    | UserDialog::Edit { name, .. } => {
                                        name.input.clone_from(&input);
                                    }
                                    _ => {}
                                }
                            });
                            let id = {
                                match &*dialog.read() {
                                    UserDialog::Edit { id, .. } => *id,
                                    _ => domain::UserID::nil()
                                }
                            };
                            async move {
                                // Debounce the validation to prevent unexpected input field updates
                                // caused by rapid inputs
                                gloo_timers::future::sleep(std::time::Duration::from_millis(10)).await;
                                {
                                    match &*dialog.read() {
                                        UserDialog::Add { name, .. } | UserDialog::Edit { name, .. } => {
                                            if name.input != input {
                                                return;
                                            }
                                        }
                                        _ => { }
                                    }
                                }
                                let validated_name = DOMAIN_SERVICE().validate_user_name(&input, id).await.map_err(|err| err.to_string());
                                dialog.with_mut(|dialog|
                                    match dialog {
                                        UserDialog::Add { name, .. } | UserDialog::Edit { name, .. } => {
                                            if name.input == input {
                                                name.validated = validated_name;
                                            }
                                        }
                                        _ => { }
                                    }
                                );
                            }
                        }
                    }
                    SelectField {
                        label: "Sex".to_string(),
                        options: vec![
                            rsx! {
                                SelectOption {
                                    text: domain::Sex::FEMALE.to_string(),
                                    value: domain::Sex::FEMALE.to_string(),
                                    selected: matches!(sex.validated, Ok(domain::Sex::FEMALE)),
                                }
                            },
                            rsx! {
                                SelectOption {
                                    text: domain::Sex::MALE.to_string(),
                                    value: domain::Sex::MALE.to_string(),
                                    selected: matches!(sex.validated, Ok(domain::Sex::MALE)),
                                }
                            },
                        ],
                        has_changed: sex.changed(),
                        onchange: move |event: FormEvent| {
                            if let UserDialog::Add { sex, .. } | UserDialog::Edit { sex, .. } = &mut *dialog.write() {
                                sex.input = event.value();
                                sex.validated = Ok(domain::Sex::from(sex.input.as_ref()));
                            }
                        }
                    }
                    div {
                        class: "field is-grouped is-grouped-centered",
                        div {
                            class: "control",
                            onclick: close,
                            button { class: "button is-light is-soft", "Cancel" }
                        }
                        div {
                            class: "control",
                            onclick: save,
                            button {
                                class: "button is-primary",
                                class: if is_loading() { "is-loading" },
                                disabled: (!name.changed() && !sex.changed()) || !name.valid() || !sex.valid(),
                                "Save"
                            }
                        }
                    }
                }
            },
            UserDialog::Delete(user) => rsx! {
                DeleteConfirmationDialog {
                    element_type: "user".to_string(),
                    element_name: rsx! { "{user.name}" },
                    delete_event: delete,
                    cancel_event: close,
                    is_loading: is_loading(),
                }
            },
        }
    }
}

enum UserDialog {
    None,
    Options(domain::User),
    Add {
        name: FieldValue<domain::Name>,
        sex: FieldValue<domain::Sex>,
    },
    Edit {
        id: domain::UserID,
        name: FieldValue<domain::Name>,
        sex: FieldValue<domain::Sex>,
    },
    Delete(domain::User),
}

#[component]
pub fn Version() -> Element {
    use_effect(|| {
        spawn(check_for_updates());
    });
    rsx! {
        Block {
            class: "px-3",
            Title { "Version" }
            VersionInfo { }
            if let UpdateStatus::Deferred = UPDATE_STATUS() {
                CenteredBlock {
                    button {
                        class: "button is-link mt-5",
                        onclick: move |_| {
                            UPDATE_STATUS.with_mut(|s| *s = UpdateStatus::Available);
                        },
                        Icon { name: "download" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Log() -> Element {
    let entries = WEB_APP_SERVICE.read().get_log_entries();
    rsx! {
        Title { "Log" }
        Block {
            class: "px-3",
            match entries {
                Ok(entries) => rsx! {
                    for entry in entries {
                        Message {
                            color: match entry.level {
                                log::Level::Error => Color::Danger,
                                log::Level::Warn => Color::Warning,
                                log::Level::Info => Color::Primary,
                                log::Level::Debug => Color::Info,
                                log::Level::Trace => Color::Dark,
                            },
                            p { class: "is-size-7", {entry.time} }
                            p { "{entry.message}" }
                        }
                    }
                },
                Err(err) => rsx! {
                    Error { message: err }
                },
            }
        }
    }
}
