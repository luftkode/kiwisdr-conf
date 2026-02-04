//! Translation utilities for ConnMan service properties.
//!
//! This module converts loosely typed ConnMan `a{sv}` dictionaries into
//! strongly typed domain values. No I/O or D-Bus interaction is performed.

use crate::wifi::connman::consts::*;
use crate::wifi::connman::error::{ConnManError, Result};
use crate::wifi::model::{Ipv4Connection, Ipv6Connection, ServiceState, ServiceStateKind};
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
fn parse_state(props: &DBusDict) -> Result<ServiceStateKind> {
    let raw = get_string(props, PROP_STATE)?;
    ServiceStateKind::try_from(raw.as_str()).map_err(|_| invalid(PROP_STATE, raw))
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

/// Parse the optional `"IPv4"` configuration block.
///
/// # Errors
///
/// - `InvalidProperty` if the block exists but is not a dictionary
/// - Any error returned by `parse_ipv4_dict`
fn parse_ipv4(props: &DBusDict) -> Result<Option<Ipv4Connection>> {
    let value = match props.get(PROP_IPV4) {
        None => return Ok(None),
        Some(v) => v,
    };

    let dict = downcast_dict(value, PROP_IPV4)?;
    Ok(Some(parse_ipv4_dict(&dict)?))
}

/// Parse the optional `"IPv6"` configuration block.
///
/// # Errors
///
/// - `InvalidProperty` if the block exists but is not a dictionary
/// - Any error returned by `parse_ipv6_dict`
fn parse_ipv6(props: &DBusDict) -> Result<Option<Ipv6Connection>> {
    let value = match props.get(PROP_IPV6) {
        None => return Ok(None),
        Some(v) => v,
    };

    let dict = downcast_dict(value, PROP_IPV6)?;
    Ok(Some(parse_ipv6_dict(&dict)?))
}

/// Parse an IPv4 configuration dictionary.
///
/// Expected keys:
/// - `"Address"` → IPv4 address string
/// - `"Gateway"` → IPv4 address string
/// - `"Prefix"` → CIDR prefix length
///
/// # Errors
///
/// - `MissingProperty` for any required key
/// - `InvalidProperty` for incorrect value types
/// - `InvalidAddress` for malformed IP strings
fn parse_ipv4_dict(dict: &DBusDict) -> Result<Ipv4Connection> {
    let address: Ipv4Addr = get_string(dict, IP_ADDRESS)?
        .parse()
        .map_err(|_| invalid_addr("IPv4", IP_ADDRESS, dict.get(IP_ADDRESS)))?;

    let gateway: Ipv4Addr = get_string(dict, IP_GATEWAY)?
        .parse()
        .map_err(|_| invalid_addr("IPv4", IP_GATEWAY, dict.get(IP_GATEWAY)))?;

    let prefix = get_u32(dict, IP_PREFIX)? as u8;

    Ok(Ipv4Connection::new(address, prefix, gateway))
}

/// Parse an IPv6 configuration dictionary.
///
/// Expected keys:
/// - `"Address"` → IPv6 address string
/// - `"Prefix"` → CIDR prefix length
/// - `"Gateway"` → IPv6 address string (optional)
///
/// # Errors
///
/// - `MissingProperty` for required keys
/// - `InvalidProperty` for incorrect value types
/// - `InvalidAddress` for malformed IP strings
fn parse_ipv6_dict(dict: &DBusDict) -> Result<Ipv6Connection> {
    let address: Ipv6Addr = get_string(dict, IP_ADDRESS)?
        .parse()
        .map_err(|_| invalid_addr("IPv6", IP_ADDRESS, dict.get(IP_ADDRESS)))?;

    let prefix = get_u32(dict, IP_PREFIX)? as u8;

    let gateway = match dict.get(IP_GATEWAY) {
        None => None,
        Some(v) => {
            let s = v
                .downcast_ref::<String>()
                .map_err(|_| invalid_type("IPv6", IP_GATEWAY, "String", v))?;

            if s.is_empty() {
                return Err(invalid_addr("IPv6", IP_GATEWAY, s));
            }

            Some(s.parse().map_err(|_| invalid_addr("IPv6", IP_GATEWAY, s))?)
        }
    };

    Ok(Ipv6Connection::new(address, prefix, gateway))
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
///
/// # Examples
///
/// ```ignore
/// # use std::collections::HashMap;
/// # use zvariant::{OwnedValue, Str};
/// # use backend::wifi::model::ServiceStateKind;
/// # use backend::wifi::connman::translation::service_state_from_properties;
/// #
/// let mut props = HashMap::new();
/// props.insert("State".into(), OwnedValue::from(Str::from("online")));
///
/// let state = service_state_from_properties("wifi0".into(), &props).unwrap();
/// assert_eq!(state.state(), ServiceStateKind::Online);
/// ```
pub fn service_state_from_properties(wifi_uid: String, props: &DBusDict) -> Result<ServiceState> {
    let ssid = get_ssid(props);
    let state = parse_state(props)?;
    let strength = parse_strength(props)?;
    let ipv4 = parse_ipv4(props)?;
    let ipv6 = parse_ipv6(props)?;

    Ok(ServiceState::new(
        ssid, wifi_uid, state, strength, ipv4, ipv6,
    ))
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
            assert_eq!(state, ServiceStateKind::Online);
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

    mod parse_ip_blocks {
        use super::*;

        #[test]
        fn missing_ipv4_block_is_none() {
            let props = empty_props();
            assert!(parse_ipv4(&props).unwrap().is_none());
        }

        #[test]
        fn invalid_ipv4_block_type_is_error() {
            let mut props = empty_props();
            props.insert(PROP_IPV4.into(), ovs("not a dict"));

            let err = parse_ipv4(&props).unwrap_err();
            let s = PROP_IPV4.to_string();
            matches!(err, ConnManError::InvalidProperty(e) if e == s);
        }

        #[test]
        fn invalid_ipv6_block_type_is_error() {
            let mut props = empty_props();
            props.insert(PROP_IPV6.into(), ov(123u32));

            let err = parse_ipv6(&props).unwrap_err();
            let s = PROP_IPV6.to_string();
            matches!(err, ConnManError::InvalidProperty(e) if e == s);
        }
    }

    mod parse_ipv4_dict {
        use super::*;

        #[test]
        fn parses_valid_ipv4_dict() {
            let mut dict = DBusDict::new();
            dict.insert(IP_ADDRESS.into(), ovs("192.168.1.10"));
            dict.insert(IP_GATEWAY.into(), ovs("192.168.1.1"));
            dict.insert(IP_PREFIX.into(), ov(24u32));

            let ipv4 = parse_ipv4_dict(&dict).unwrap();
            assert_eq!(ipv4.cidr(), "192.168.1.10/24");
            assert_eq!(ipv4.gateway(), Ipv4Addr::new(192, 168, 1, 1));
        }

        #[test]
        fn missing_address_is_error() {
            let mut dict = DBusDict::new();
            dict.insert(IP_PREFIX.into(), ov(24u32));

            let err = parse_ipv4_dict(&dict).unwrap_err();
            let s = IP_ADDRESS.to_string();
            matches!(err, ConnManError::MissingProperty(e) if e == s);
        }
    }

    mod parse_ipv4_edge_cases {
        use super::*;

        #[test]
        fn rejects_invalid_ipv4_address() {
            let mut dict = DBusDict::new();
            dict.insert(IP_ADDRESS.into(), ovs("999.999.999.999"));
            dict.insert(IP_GATEWAY.into(), ovs("192.168.1.1"));
            dict.insert(IP_PREFIX.into(), ov(24u32));

            let err = parse_ipv4_dict(&dict).unwrap_err();
            let s = IP_ADDRESS.to_string();
            matches!(err, ConnManError::InvalidAddress(e) if e == s);
        }

        #[test]
        fn rejects_invalid_gateway_address() {
            let mut dict = DBusDict::new();
            dict.insert(IP_ADDRESS.into(), ovs("192.168.1.10"));
            dict.insert(IP_GATEWAY.into(), ovs("nope"));
            dict.insert(IP_PREFIX.into(), ov(24u32));

            let err = parse_ipv4_dict(&dict).unwrap_err();
            let s = IP_GATEWAY.to_string();
            matches!(err, ConnManError::InvalidAddress(e) if e == s);
        }

        #[test]
        fn prefix_zero_is_allowed() {
            let mut dict = DBusDict::new();
            dict.insert(IP_ADDRESS.into(), ovs("0.0.0.0"));
            dict.insert(IP_GATEWAY.into(), ovs("0.0.0.0"));
            dict.insert(IP_PREFIX.into(), ov(0u32));

            let ipv4 = parse_ipv4_dict(&dict).unwrap();
            assert_eq!(ipv4.cidr(), "0.0.0.0/0");
        }
    }

    mod parse_ipv6_dict {
        use super::*;

        #[test]
        fn parses_ipv6_without_gateway() {
            let mut dict = DBusDict::new();
            dict.insert(IP_ADDRESS.into(), ovs("fe80::1"));
            dict.insert(IP_PREFIX.into(), ov(64u32));

            let ipv6 = parse_ipv6_dict(&dict).unwrap();
            assert_eq!(ipv6.cidr(), "fe80::1/64");
            assert!(ipv6.gateway().is_none());
        }

        #[test]
        fn parses_ipv6_with_gateway() {
            let mut dict = DBusDict::new();
            dict.insert(IP_ADDRESS.into(), ovs("fe80::1"));
            dict.insert(IP_GATEWAY.into(), ovs("fe80::ff"));
            dict.insert(IP_PREFIX.into(), ov(64u32));

            let ipv6 = parse_ipv6_dict(&dict).unwrap();
            assert_eq!(
                ipv6.gateway().unwrap(),
                Ipv6Addr::from([0xfe80, 0, 0, 0, 0, 0, 0, 0xff])
            );
        }
    }

    mod parse_ipv6_edge_cases {
        use super::*;

        #[test]
        fn rejects_invalid_ipv6_address() {
            let mut dict = DBusDict::new();
            dict.insert(IP_ADDRESS.into(), ovs("this:is:garbage"));
            dict.insert(IP_PREFIX.into(), ov(64u32));

            let err = parse_ipv6_dict(&dict).unwrap_err();
            let s = IP_ADDRESS.to_string();
            matches!(err, ConnManError::InvalidAddress(e) if e == s);
        }

        #[test]
        fn empty_gateway_is_error() {
            let mut dict = DBusDict::new();
            dict.insert(IP_ADDRESS.into(), ovs("fe80::1"));
            dict.insert(IP_GATEWAY.into(), ovs(""));
            dict.insert(IP_PREFIX.into(), ov(64u32));

            let err = parse_ipv6_dict(&dict).unwrap_err();
            let s = IP_GATEWAY.to_string();
            matches!(err, ConnManError::InvalidAddress(e) if e == s);
        }

        #[test]
        fn missing_prefix_is_error() {
            let mut dict = DBusDict::new();
            dict.insert(IP_ADDRESS.into(), ovs("fe80::1"));

            let err = parse_ipv6_dict(&dict).unwrap_err();
            let s = IP_PREFIX.to_string();
            matches!(err, ConnManError::MissingProperty(e) if e == s);
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
            assert_eq!(state.state(), ServiceStateKind::Ready);
            assert_eq!(state.strength(), Some(55));
            assert!(state.ipv4().is_none());
            assert!(state.ipv6().is_none());
        }
    }

    mod service_state_partial_failures {
        use super::*;

        #[test]
        fn invalid_ipv4_does_not_hide_state_error() {
            let mut ipv4 = DBusDict::new();
            ipv4.insert(IP_ADDRESS.into(), ovs("broken"));
            ipv4.insert(IP_PREFIX.into(), ov(24u32));

            let mut props = empty_props();
            props.insert(PROP_STATE.into(), ovs("online"));
            props.insert(PROP_IPV4.into(), ov(ipv4));

            let err = service_state_from_properties("wifi0".into(), &props).unwrap_err();
            let s = IP_ADDRESS.to_string();
            matches!(err, ConnManError::InvalidAddress(e) if e == s);
        }

        #[test]
        fn missing_strength_does_not_fail_service() {
            let mut props = empty_props();
            props.insert(PROP_STATE.into(), ovs("online"));

            let state = service_state_from_properties("wifi0".into(), &props).unwrap();
            assert_eq!(state.strength(), None);
        }
    }
}
