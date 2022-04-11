use std::error::Error;

use isahc::prelude::*;
use isahc::Request;
use log::debug;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::device::{Device, DeviceType, get_device_from_guid};

/// A struct representing the data from SQLSprinkler zones.
#[derive(Deserialize)]
pub struct Zone {
    pub name: String,
    pub gpio: u8,
    pub time: u64,
    pub enabled: bool,
    pub auto_off: bool,
    pub system_order: i8,
    pub state: bool,
    pub id: i8,
}

/// Represents data for toggling a zone.
#[derive(Serialize, Deserialize, Debug)]
struct ZoneToggle {
    id: i64,
    state: bool,
}

/// Represents data for
#[derive(Serialize, Deserialize, Debug)]
struct SystemToggle {
    system_enabled: bool,
}

/// Sets the zone status to the given state
pub fn set_zone(ip: String, state: bool, id: i64) -> bool {
    let url = format!("http://{}:3030/zone", ip);

    let zone_toggle = ZoneToggle { id, state };

    let send_res = match Request::put(&url)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&zone_toggle).unwrap())
        .timeout(std::time::Duration::from_secs(3))
        .unwrap()
        .send()
    {
        Ok(res) => res.status().is_success(),
        Err(e) => {
            debug!("Error: {} with URL {}", e, &url);
            false
        }
    };
    send_res
}

/// Sets the sprinkler system on/off
pub fn set_system(ip: String, state: bool) -> bool {
    let url = format!("http://{}:3030/system/state", ip);

    let system_state = SystemToggle {
        system_enabled: state,
    };

    let status = match Request::put(url)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&system_state).unwrap())
        .timeout(std::time::Duration::from_secs(3))
        .unwrap()
        .send()
    {
        Ok(..) => true,
        Err(..) => false,
    };
    status
}

/// Gets the status from the SQLSprinkler host
/// # Params
/// * `ip` The IP Address of the SQLSprinkler host.
/// # Return
/// A boolean representing the state of the SQLSprinkler host, or an error if something happened.
pub(crate) fn get_status_from_sqlsprinkler(ip: &String) -> Result<bool, Box<dyn Error>> {
    let url = format!("http://{}:3030/system/state", ip);
    let response = match isahc::get(url).timeout(std::time::Duration::from_secs(3)).unwrap() {
        Ok(mut res) => res.text().unwrap(),
        Err(..) => "".to_string(),
    };

    match response.as_str() {
        "" => return Ok(false),
        _ => {
            let system_status: SystemToggle = serde_json::from_str(&response).unwrap();
            Ok(system_status.system_enabled)
        }
    }
}

/// Gets all the zones from the SQLSprinkler host.
/// # Params
/// * `ip` A string representing the IP address of the SQLSprinkler host.
/// # Returns
/// * A `Vec<Zone>` Representing all of the SQLSprinkler zones on the given host.  Or, if an
/// error occurs, we will get that error.
fn get_zones_from_sqlsprinkler(ip: &String) -> Result<Vec<Zone>, Box<dyn Error>> {
    let url = format!("http://{}:3030/zone/info", ip);

    //TODO: Make this less ugly.
    let response = isahc::get(url).timeout(std::time::Duration::from_secs(3)).unwrap().text().unwrap();;
    let zone_list: Vec<Zone> = serde_json::from_str(&response).unwrap();

    Ok(zone_list)
}

/// Checks to see if the given device is an SQLSprinkler Host.  If it is, push the zones that are
/// connected to that SQLSprinkler host.
/// # Params
/// * `dev` -> A mutable device representing the SQLSprinkler host
/// # Return
/// * True if the device is a sqlsprinkler host.
pub fn check_if_device_is_sqlsprinkler_host(dev: Device) -> Vec<Device> {
    let mut device_list = Vec::new();

    if dev.kind != DeviceType::SqlSprinklerHost {
        return device_list;
    }

    let ip = &dev.ip;
    let sprinkler_list = get_zones_from_sqlsprinkler(ip).unwrap();

    for zone in sprinkler_list {
        // Create a device from a sprinkler zone
        let mut sprinkler_device = Device::from(zone);

        // Make a new guid in the form of deviceguid-zoneid
        let new_guid = format!("{}-{}", dev.guid, sprinkler_device.guid);
        sprinkler_device.guid = new_guid;
        sprinkler_device.ip = dev.ip.to_string();

        device_list.push(sprinkler_device);
    }
    device_list
}

/// Checks to see if the given guid is a SQLSprinkler zone.
/// # Param
/// * `guid`  The GUID of the device we are checking.
/// # Return
/// True if there is a match to the pattern of a SQLSprinkler zone.
pub fn check_if_zone(guid: &String) -> bool {
    let re =
        Regex::new(r"(?im)^[0-9A-Fa-f]{8}[-]?(?:[0-9A-Fa-f]{4}[-]?){3}[0-9A-Fa-f]{12}[-][0-9].?$")
            .unwrap();
    re.is_match(guid.as_str())
}

/// Gets a Zone(as a Device) from the given GUID.
pub fn get_zone(guid: &String) -> Device {
    let host_guid = &guid[0..36];
    debug!("Host guid: {}", host_guid);
    let host_device = get_device_from_guid(&host_guid.to_string());
    let reg =
        Regex::new(r"(?im)^[0-9A-Fa-f]{8}[-]?(?:[0-9A-Fa-f]{4}[-]?){3}[0-9A-Fa-f]{12}[-]").unwrap();

    let id_vec: Vec<String> = reg.split(&guid).map(|x| x.to_string()).collect();

    let id = id_vec[1].parse::<i64>().unwrap() as i8;
    debug!("Got SQLSprinkler host device with IP: {}", &host_device.ip);
    let sprinkler_list = get_zones_from_sqlsprinkler(&host_device.ip).unwrap();
    for zone in sprinkler_list {
        if zone.id == id {
            let mut zone_device = Device::from(zone);
            zone_device.ip = host_device.ip.clone();
            return zone_device;
        }
    }
    Device::default()
}
