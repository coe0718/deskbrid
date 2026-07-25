//! DDC/CI dispatch router — keeps DDC actions off the desktop-backend gate path.

use crate::protocol::Action;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Returns `Some(future)` if `action` is a DDC/CI action; otherwise `None`.
/// Lets `dispatch.rs` handle DDC before the backend gate fires.
pub fn dispatch_ddc(
    action: Action,
) -> Option<Pin<Box<dyn Future<Output = anyhow::Result<Value>> + Send>>> {
    match action {
        Action::DdcList => Some(Box::pin(super::ddc::ddc_list())),
        Action::DdcGetVcp { bus, vcp_code } => {
            Some(Box::pin(super::ddc::ddc_getvcp(bus, vcp_code)))
        }
        Action::DdcSetVcp {
            bus,
            vcp_code,
            value,
        } => Some(Box::pin(super::ddc::ddc_setvcp(bus, vcp_code, value))),
        Action::MonitorDdcBrightness { bus, percent } => {
            Some(Box::pin(super::ddc::ddc_brightness(bus, percent)))
        }
        Action::MonitorDdcContrast { bus, percent } => {
            Some(Box::pin(super::ddc::ddc_contrast(bus, percent)))
        }
        Action::MonitorDdcPower { bus, state } => Some(Box::pin(super::ddc::ddc_power(bus, state))),
        Action::MonitorDdcInput { bus, input } => Some(Box::pin(super::ddc::ddc_input(bus, input))),
        _ => None,
    }
}
