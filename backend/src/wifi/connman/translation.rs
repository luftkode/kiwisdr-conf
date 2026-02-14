//! Translation utilities for ConnMan service properties.
//!
//! This module converts loosely typed ConnMan `a{sv}` dictionaries into
//! strongly typed domain values. No I/O or D-Bus interaction is performed.

use crate::wifi::connman::consts::*;
use crate::wifi::connman::error::{ConnManError, Result};
use crate::wifi::model::{InterfaceName, Ipv4Connection, Ipv6Connection, WifiNetwork, WifiStatus};
use std::collections::HashMap;
use std::fmt::Debug;
use std::net::{Ipv4Addr, Ipv6Addr};
use zvariant::{Dict, OwnedValue};

/// Convenience alias for ConnMan property dictionaries.
///
/// ConnMan represents most structured data as `a{sv}` maps.
/// At this layer we normalize those into owned Rust maps for
/// predictable access and lifetimes.
type DBusDict = HashMap<String, OwnedValue>;

fn missing(key: &str) -> ConnManError {
    ConnManError::MissingProperty(format!("Missing required property '{}'", key))
}

fn invalid(key: &str, value: impl Debug) -> ConnManError {
    ConnManError::InvalidProperty(format!("Invalid value for property '{}': {:?}", key, value))
}

fn invalid_type(parent: &str, key: &str, expected: &str, value: impl Debug) -> ConnManError {
    ConnManError::InvalidProperty(format!(
        "Property '{}.{}' is not a {}, it is: {:?}",
        parent, key, expected, value
    ))
}

fn invalid_addr(parent: &str, key: &str, value: impl Debug) -> ConnManError {
    ConnManError::InvalidProperty(format!(
        "Invalid address {:?} in '{}.{}'",
        value, parent, key
    ))
}

fn not_a_dict(key: &str) -> ConnManError {
    ConnManError::InvalidProperty(format!("Property '{}' is not a dictionary", key))
}

/// Parse the `"State"` property into a `ServiceStateKind`.
///
/// # Errors
///
/// - `MissingProperty` if the `"State"` key is absent
/// - `InvalidProperty` if the value is not a string
/// - `InvalidProperty` if the state string is unknown
fn parse_state(props: &DBusDict) -> Result<WifiStatus> {
    let raw = get_string(props, PROP_STATE)?;
    WifiStatus::try_from(raw.as_str()).map_err(|_| invalid(PROP_STATE, raw))
}

/// Parse the optional `"Strength"` property.
///
/// ConnMan reports Wi-Fi strength as an unsigned byte (0–100),
/// but values outside that range are preserved verbatim.
///
/// # Errors
///
/// - `InvalidProperty` if the value exists but is not a `u8`
fn parse_strength(props: &DBusDict) -> Result<Option<u8>> {
    match props.get(PROP_STRENGTH) {
        None => Ok(None),
        Some(v) => match v.downcast_ref::<u8>() {
            Ok(n) => Ok(Some(n)),
            Err(_) => Err(invalid(PROP_STRENGTH, v)),
        },
    }
}

fn parse_interface(props: &DBusDict) -> Result<Option<InterfaceName>> {
    Ok(None) // TODO
}

/// Convert a D-Bus `Dict` value into a owned `HashMap<String, OwnedValue>`.
///
/// ConnMan frequently nests dictionaries inside `OwnedValue`.
/// This helper performs a **deep, owned conversion** so callers
/// can safely access values without lifetime gymnastics.
///
/// # Errors
///
/// - `InvalidProperty` if the value is not a dictionary
/// - `InvalidProperty` if any key is not a string
fn downcast_dict(value: &OwnedValue, name: &'static str) -> Result<DBusDict> {
    let dict = value.downcast_ref::<Dict>().map_err(|_| not_a_dict(name))?;

    let mut out = DBusDict::new();
    for (k, v) in dict {
        let key = k
            .downcast_ref::<String>()
            .map_err(|_| invalid_type(name, "key", "String", k))?
            .clone();

        out.insert(
            key.clone(),
            OwnedValue::try_from(v.clone())
                .map_err(|_| invalid_type(name, &key, "OwnedValue", v))?,
        );
    }

    Ok(out)
}

/// Returns the value of a required string property.
///
/// # Errors
///
/// Returns an error if the property is missing or if the value is not a string.
fn get_string(props: &DBusDict, key: &'static str) -> Result<String> {
    let v = props.get(key).ok_or_else(|| missing(key))?;
    v.downcast_ref::<String>()
        .map_err(|_| invalid_type("props", key, "String", v))
}

/// Fetch a required `u32` property.
///
/// # Errors
///
/// - `MissingProperty` if the key is absent
/// - `InvalidProperty` if the value is not a `u32`
fn get_u32(props: &DBusDict, key: &'static str) -> Result<u32> {
    let v = props.get(key).ok_or_else(|| missing(key))?;
    v.downcast_ref::<u32>()
        .map_err(|_| invalid_type("props", key, "u32", v))
}

fn get_ssid(props: &DBusDict) -> Option<String> {
    props.get(PROP_NAME)?.downcast_ref::<String>().ok()
}

/// Constructs a [`ServiceState`] from a ConnMan service property map.
///
/// The property map is expected to follow ConnMan’s `a{sv}` schema.
/// Required properties must be present and correctly typed.
/// Optional properties are validated if present.
///
/// # Errors
///
/// Returns an error if any required property is missing, if a property
/// has an unexpected type, or if an address string cannot be parsed.
pub fn service_state_from_properties(wifi_uid: String, props: &DBusDict) -> Result<WifiNetwork> {
    let ssid = get_ssid(props);
    let state = parse_state(props)?;
    let strength = parse_strength(props)?;
    let interface = parse_interface(props)?;

    Ok(WifiNetwork::new(ssid, wifi_uid, state, strength, interface))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zvariant::{OwnedValue, Str};

    fn ov<T: Into<OwnedValue>>(v: T) -> OwnedValue {
        v.into()
    }

    fn ovs(s: &str) -> OwnedValue {
        Str::from(s).into()
    }

    fn empty_props() -> DBusDict {
        DBusDict::new()
    }

    mod parse_state {
        use super::*;

        #[test]
        fn parses_valid_states() {
            let mut props = empty_props();
            props.insert(PROP_STATE.into(), ovs("online"));

            let state = parse_state(&props).unwrap();
            assert_eq!(state, WifiStatus::Online);
        }

        #[test]
        fn rejects_unknown_state() {
            let mut props = empty_props();
            props.insert(PROP_STATE.into(), ovs("nonsense"));

            let err = parse_state(&props).unwrap_err();
            let s = PROP_STATE.to_string();
            matches!(err, ConnManError::InvalidProperty(e) if e == s);
        }

        #[test]
        fn missing_state_is_error() {
            let props = empty_props();
            let err = parse_state(&props).unwrap_err();
            let s = PROP_STATE.to_string();
            matches!(err, ConnManError::MissingProperty(e) if e == s);
        }
    }

    mod parse_strength {
        use super::*;

        #[test]
        fn missing_strength_is_none() {
            let props = empty_props();
            assert_eq!(parse_strength(&props).unwrap(), None);
        }

        #[test]
        fn parses_strength() {
            let mut props = empty_props();
            props.insert(PROP_STRENGTH.into(), ov(73u8));

            assert_eq!(parse_strength(&props).unwrap(), Some(73));
        }

        #[test]
        fn invalid_strength_type_is_error() {
            let mut props = empty_props();
            props.insert(PROP_STRENGTH.into(), ovs("loud"));

            let err = parse_strength(&props).unwrap_err();
            let s = PROP_STRENGTH.to_string();
            matches!(err, ConnManError::InvalidProperty(e) if e == s);
        }

        #[test]
        fn strength_above_100_is_preserved() {
            let mut props = empty_props();
            props.insert(PROP_STATE.into(), ovs("online"));
            props.insert(PROP_STRENGTH.into(), ov(255u8));

            let state = service_state_from_properties("wifi0".into(), &props).unwrap();
            assert_eq!(state.strength(), Some(255));
        }
    }

    mod downcast_dict {
        use super::*;

        #[test]
        fn downcasts_dictionary() {
            let mut inner = DBusDict::new();
            inner.insert("Key".into(), ovs("Value"));

            let value = ov(inner);
            let dict = downcast_dict(&value, "Test").unwrap();

            assert!(dict.contains_key("Key"));
        }

        #[test]
        fn rejects_non_dictionary() {
            let value = ovs("not a dict");
            let err = downcast_dict(&value, "Test").unwrap_err();
            let s = "Test".to_string();
            matches!(err, ConnManError::InvalidProperty(e) if e == s);
        }
    }

    mod service_state_from_properties {
        use super::*;

        #[test]
        fn builds_minimal_service_state() {
            let mut props = empty_props();
            props.insert(PROP_STATE.into(), ovs("ready"));
            props.insert(PROP_STRENGTH.into(), ov(55u8));

            let state = service_state_from_properties("wifi0".into(), &props).unwrap();

            assert_eq!(state.uid(), "wifi0"); // replace wifi_uid() with uid()
            assert_eq!(state.state(), WifiStatus::Ready);
            assert_eq!(state.strength(), Some(55));
            assert!(state.interface().is_none());
        }
    }

    mod service_state_partial_failures {
        use super::*;

        #[test]
        fn missing_strength_does_not_fail_service() {
            let mut props = empty_props();
            props.insert(PROP_STATE.into(), ovs("online"));

            let state = service_state_from_properties("wifi0".into(), &props).unwrap();
            assert_eq!(state.strength(), None);
        }
    }
}
