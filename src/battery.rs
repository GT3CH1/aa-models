use isahc::ReadResponseExt;
use serde_json::Value;

use crate::device::Device;

/// Checks to see if the given device is a battery, if it is, get the data from the UPS status page.
/// # Param
/// * dev : The Device we want to check to see if it is a battery/UPS
/// # Return
/// True if the device is a TV, false otherwise.
pub fn parse_device(mut dev: Device) -> Device {
    if dev.kind == crate::device::DeviceType::BATTERY {
        let battery_status: Value = serde_json::from_str(
            isahc::get(format!("http://{}/ups_status.php", dev.ip))
                .unwrap()
                .text()
                .unwrap()
                .as_str(),
        )
            .unwrap();
        dev.last_state = battery_status;
        dev.database_update();
        return dev.clone();
    }
    dev.clone()
}
