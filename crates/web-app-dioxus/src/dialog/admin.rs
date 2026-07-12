use dioxus::prelude::*;

use valens_domain as domain;
use valens_domain::UserService;

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE,
    diagnostics::log_failure,
    notification::notify,
    session::{Session, SessionRefresh},
    signal_changed_data,
    ui::{
        element::{
            CenteredBlock, DeleteConfirmationDialog, Dialog, ErrorMessage, Icon, ItemOptionsButton,
            Loading, MenuOption, NoConnection, OptionsMenu, SaveDialog, Table, Title, value_or_dash,
        },
        form::{FieldValue, FieldValueState, InputField, SelectField, SelectOption},
    },
};

#[component]
pub fn AdminDialog(on_close: EventHandler<MouseEvent>) -> Element {
    rsx! {
        Dialog {
            title: rsx! { "Administration" },
            on_close,
            Users {}
        }
    }
}

#[component]
fn Users() -> Element {
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
                UserDialog::Add { name, sex, height, role } => {
                    if let (Ok(name), Ok(sex), Ok(height), Ok(role)) = (name.validated.clone(), sex.validated.clone(), height.validated.clone(), role.validated.clone()) {
                        match DOMAIN_SERVICE().create_user(name, sex, height, role).await {
                            Ok(_) => {
                                saved = true;
                                signal_changed_data();
                            },
                            Err(err) => {
                                notify("Failed to add user", &err);
                            }
                        }
                    }
                },
                UserDialog::Edit { id, name, sex, height, role } => {
                    if let (Ok(name), Ok(sex), Ok(height), Ok(role)) = (name.validated.clone(), sex.validated.clone(), height.validated.clone(), role.validated.clone()) {
                        let id = *id;
                        match DOMAIN_SERVICE().replace_user(domain::User { id, name, sex, height, role }).await {
                            Ok(_) => {
                                saved = true;
                                signal_changed_data();
                                if id == consume_context::<Session>().user().id {
                                    consume_context::<SessionRefresh>().refresh();
                                }
                            },
                            Err(err) => {
                                notify("Failed to edit user", &err);
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
                    Err(err) => notify("Failed to delete user", &err)

                }
            }
        }
        if deleted {
            close_dialog();
        }
    };
    let close = move |_| close_dialog();

    rsx! {
        match &*users.read() {
            Some(Ok(users)) => {
                rsx! {
                    Title {
                        actions: rsx! {
                            a {
                                "data-testid": "add-user",
                                onclick: move |_| {
                                    *dialog.write() = UserDialog::Add {
                                        name: FieldValue::default(),
                                        sex: FieldValue::new(domain::Sex::MALE),
                                        height: FieldValue::from_option(None),
                                        role: FieldValue::new(domain::Role::USER),
                                    };
                                },
                                Icon { name: "plus", is_small: true }
                            }
                        },
                        "Users"
                    }
                    Table {
                        head: vec![rsx! { "Name" }, rsx! { "Sex" }, rsx! { "Height (cm)" }, rsx! { "Role" }, rsx! {}],
                        body: users.iter().map(|user| {
                            let user = user.clone();
                            vec![
                                rsx! { "{user.name}" },
                                rsx! { "{user.sex}" },
                                rsx! { {value_or_dash(user.height)} },
                                rsx! { "{user.role}" },
                                rsx! {
                                    ItemOptionsButton { on_click: move |_| { *dialog.write() = UserDialog::Options(user.clone()); } }
                                }
                            ]
                        }).collect::<Vec<_>>()
                    }
                }
            }
            Some(Err(domain::ReadError::Forbidden(_))) => {
                rsx! {
                    CenteredBlock {
                        p {
                            "data-testid": "not-authorized",
                            "You are not authorized to manage users."
                        }
                    }
                }
            }
            Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))) => {
                rsx! {
                    NoConnection {}
                }
            }
            Some(Err(err)) => {
                log_failure("load users", err);
                rsx! {
                    ErrorMessage { message: err }
                }
            }
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
                                    "data-testid": "options-edit-user",
                                    on_click: move |_| {
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
                                            },
                                            height: FieldValue::from_option(user_edit.height),
                                            role: FieldValue {
                                                input: user_edit.role.to_string(),
                                                validated: Ok(user_edit.role),
                                                orig: user_edit.role.to_string()
                                            },
                                        };
                                    }
                                },
                                MenuOption {
                                    icon: "user-times".to_string(),
                                    text: "Delete user".to_string(),
                                    "data-testid": "options-delete-user",
                                    on_click: move |_| { *dialog.write() = UserDialog::Delete(user_delete.clone()); }
                                },
                            },
                        ],
                        on_close: move |_| *dialog.write() = UserDialog::None,
                    }
                }
            },
            UserDialog::Add { name, sex, height, role } | UserDialog::Edit { name, sex, height, role, .. } => {
                rsx! {
                SaveDialog {
                    title: rsx! { if let UserDialog::Add { .. } = &*dialog.read() { "Add user" } else { "Edit user" } },
                    on_close: close,
                    on_save: save,
                    is_loading: is_loading(),
                    disabled: (!name.changed() && !sex.changed() && !height.changed() && !role.changed()) || !name.valid() || !sex.valid() || !height.valid() || !role.valid(),
                    InputField {
                        label: "Name".to_string(),
                        "data-testid": "user-name",
                        value: name.input.clone(),
                        error: if let Err(err) = &name.validated { err.clone() },
                        has_changed: name.changed(),
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            match &mut *dialog.write() {
                                UserDialog::Add { name, .. }
                                | UserDialog::Edit { name, .. } => {
                                    name.input.clone_from(&input);
                                }
                                _ => {}
                            }
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
                                        UserDialog::Add { name, .. } | UserDialog::Edit { name, .. }
                                            if name.input != input => {
                                                return;
                                            }
                                        _ => {}
                                    }
                                }
                                let validated_name = DOMAIN_SERVICE().validate_user_name(&input, id).await.map_err(|err| err.to_string());
                                match &mut *dialog.write() {
                                    UserDialog::Add { name, .. } | UserDialog::Edit { name, .. }
                                        if name.input == input => {
                                            name.validated = validated_name;
                                        }
                                    _ => {}
                                }
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
                        on_change: move |event: FormEvent| {
                            if let UserDialog::Add { sex, .. } | UserDialog::Edit { sex, .. } = &mut *dialog.write() {
                                sex.input = event.value();
                                sex.validated = Ok(domain::Sex::from(sex.input.as_ref()));
                            }
                        }
                    }
                    InputField {
                        label: "Height".to_string(),
                        "data-testid": "user-height",
                        right_icon: rsx! { "cm" },
                        inputmode: "numeric".to_string(),
                        value: height.input.clone(),
                        error: if let Err(err) = &height.validated { err.clone() },
                        has_changed: height.changed(),
                        on_input: move |event: FormEvent| {
                            if let UserDialog::Add { height, .. } | UserDialog::Edit { height, .. } = &mut *dialog.write() {
                                height.input = event.value();
                                height.validated = DOMAIN_SERVICE()
                                    .validate_user_height(&height.input)
                                    .map_err(|err| err.to_string());
                            }
                        }
                    }
                    SelectField {
                        label: "Role".to_string(),
                        "data-testid": "user-role",
                        options: vec![
                            rsx! {
                                SelectOption {
                                    text: domain::Role::USER.to_string(),
                                    value: domain::Role::USER.to_string(),
                                    selected: matches!(role.validated, Ok(domain::Role::USER)),
                                }
                            },
                            rsx! {
                                SelectOption {
                                    text: domain::Role::ADMIN.to_string(),
                                    value: domain::Role::ADMIN.to_string(),
                                    selected: matches!(role.validated, Ok(domain::Role::ADMIN)),
                                }
                            },
                        ],
                        has_changed: role.changed(),
                        on_change: move |event: FormEvent| {
                            if let UserDialog::Add { role, .. } | UserDialog::Edit { role, .. } = &mut *dialog.write() {
                                role.input = event.value();
                                role.validated = Ok(domain::Role::from(role.input.as_ref()));
                            }
                        }
                    }
                }
            }},
            UserDialog::Delete(user) => {
                rsx! {
                    DeleteConfirmationDialog {
                        element_type: "user".to_string(),
                        element_name: rsx! { "{user.name}" },
                        on_delete: delete,
                        on_cancel: close,
                        is_loading: is_loading(),
                    }
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
        height: FieldValue<Option<u8>>,
        role: FieldValue<domain::Role>,
    },
    Edit {
        id: domain::UserID,
        name: FieldValue<domain::Name>,
        sex: FieldValue<domain::Sex>,
        height: FieldValue<Option<u8>>,
        role: FieldValue<domain::Role>,
    },
    Delete(domain::User),
}
