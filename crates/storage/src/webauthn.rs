//! `WebAuthn` ceremonies via the browser credentials API.
//!
//! The options returned by the server and the credentials expected by it use the `WebAuthn`
//! JSON representation. The conversion between JSON and the binary fields of the browser
//! API is done by `PublicKeyCredential.parseCreationOptionsFromJSON`,
//! `PublicKeyCredential.parseRequestOptionsFromJSON` and `PublicKeyCredential.toJSON`.
//! These methods are bound manually as their `web-sys` bindings are still unstable.

#![allow(clippy::missing_errors_doc)]

use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
unsafe extern "C" {
    #[wasm_bindgen(catch, js_namespace = PublicKeyCredential, js_name = parseCreationOptionsFromJSON)]
    fn parse_creation_options_from_json(options: &JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_namespace = PublicKeyCredential, js_name = parseRequestOptionsFromJSON)]
    fn parse_request_options_from_json(options: &JsValue) -> Result<JsValue, JsValue>;

    type Credential;

    #[wasm_bindgen(catch, method, js_name = toJSON)]
    fn to_json(this: &Credential) -> Result<JsValue, JsValue>;
}

/// Create a new credential and return it in JSON representation.
pub async fn create_credential(options: &serde_json::Value) -> Result<serde_json::Value, Error> {
    let parsed = parse_creation_options_from_json(&to_js(options)?)
        .map_err(|err| js_error("failed to parse creation options", &err))?;
    let credential_options = web_sys::CredentialCreationOptions::new();
    set_public_key(&credential_options, &parsed)?;
    let promise = credentials()?
        .create_with_options(&credential_options)
        .map_err(|err| js_error("failed to create credential", &err))?;
    credential_to_json(
        &JsFuture::from(promise)
            .await
            .map_err(|err| js_error("failed to create credential", &err))?,
    )
}

/// Request an existing credential and return it in JSON representation.
pub async fn get_credential(options: &serde_json::Value) -> Result<serde_json::Value, Error> {
    let parsed = parse_request_options_from_json(&to_js(options)?)
        .map_err(|err| js_error("failed to parse request options", &err))?;
    let credential_options = web_sys::CredentialRequestOptions::new();
    set_public_key(&credential_options, &parsed)?;
    let promise = credentials()?
        .get_with_options(&credential_options)
        .map_err(|err| js_error("failed to get credential", &err))?;
    credential_to_json(
        &JsFuture::from(promise)
            .await
            .map_err(|err| js_error("failed to get credential", &err))?,
    )
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// The ceremony was cancelled or blocked by the browser or the user.
    #[error("the passkey operation was cancelled or not allowed")]
    NotAllowed,
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Whether `err` wraps a ceremony that was cancelled or blocked (`NotAllowed`).
    #[must_use]
    pub fn is_cancellation(err: &(dyn std::error::Error + 'static)) -> bool {
        matches!(err.downcast_ref::<Self>(), Some(Self::NotAllowed))
    }
}

fn credentials() -> Result<web_sys::CredentialsContainer, Error> {
    Ok(web_sys::window()
        .ok_or_else(|| Error::Other("failed to access window".to_string()))?
        .navigator()
        .credentials())
}

fn set_public_key(options: &JsValue, public_key: &JsValue) -> Result<(), Error> {
    js_sys::Reflect::set(options, &JsValue::from_str("publicKey"), public_key)
        .map_err(|err| js_error("failed to set public key options", &err))?;
    Ok(())
}

fn credential_to_json(credential: &JsValue) -> Result<serde_json::Value, Error> {
    let json = credential
        .unchecked_ref::<Credential>()
        .to_json()
        .map_err(|err| js_error("failed to serialize credential", &err))?;
    let text = js_sys::JSON::stringify(&json)
        .map_err(|err| js_error("failed to serialize credential", &err))?;
    serde_json::from_str(&String::from(text))
        .map_err(|err| Error::Other(format!("failed to deserialize credential: {err}")))
}

fn to_js(value: &serde_json::Value) -> Result<JsValue, Error> {
    js_sys::JSON::parse(&value.to_string())
        .map_err(|err| js_error("failed to convert options", &err))
}

fn js_error(context: &str, err: &JsValue) -> Error {
    let name = js_sys::Reflect::get(err, &JsValue::from_str("name"))
        .ok()
        .and_then(|name| name.as_string());
    if name.as_deref() == Some("NotAllowedError") {
        return Error::NotAllowed;
    }
    let message = js_sys::Reflect::get(err, &JsValue::from_str("message"))
        .ok()
        .and_then(|message| message.as_string());
    Error::Other(match (name, message) {
        (Some(name), Some(message)) => format!("{context}: {name}: {message}"),
        _ => format!("{context}: {err:?}"),
    })
}
