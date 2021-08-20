use std::fmt;
use std::process::Command;
use std::str::FromStr;

use isahc::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::sqlsprinkler::*;
use crate::tv;
use aa_consts::*;

/// Data representing a device that can be automated/remotely controlled.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    /// The IP of the device (sometimes used)
    pub ip: String,

    /// The GUID of the device
    pub guid: String,

    /// What kind the device is
    pub kind: DeviceType,

    /// The hardware used on the device
    pub hardware: HardwareType,

    /// The last state of the device (can be changed)
    pub last_state: Value,

    /// The current software version on the device.
    pub sw_version: String,

    /// The user this device belongs to.
    pub useruuid: String,

    /// The name of the device
    pub name: String,

    /// A list of nicknames for the device
    pub nicknames: Vec<String>,
}


/// Represents hardware types
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone)]
pub enum HardwareType {
    ARDUINO,
    PI,
    OTHER,
    LG,
}


/// Represents all the different types of devices we can have
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone)]
pub enum DeviceType {
    LIGHT,
    SWITCH,
    GARAGE,
    SPRINKLER,
    ROUTER,
    SqlSprinklerHost,
    TV,
}

/// Gets all the traits that belong to a TV.
fn tv_traits() -> Vec<&'static str> {
    vec!["action.devices.traits.OnOff", "action.devices.traits.Volume"]
}

/// Gets all the traits that belong to opening/closing doors
fn open_close_traits() -> Vec<&'static str> {
    vec!["action.devices.traits.OpenClose"]
}

/// Gets all traits that belong to turning things on/off
fn on_off_traits() -> Vec<&'static str> {
    vec!["action.devices.traits.OnOff"]
}

/// Gets all traits that belong to things that can be rebooted
fn reboot_traits() -> Vec<&'static str> {
    vec!["action.devices.traits.Reboot"]
}

/// Gets attributes for garage doors
/// # Return
/// The attributes needed for garage doors.
fn garage_attribute() -> Value {
    serde_json::json!({
        "discreteOnlyOpenClose": true
    })
}

/// Gets the attributes for on/off devices (switches, outlets, some lights)
/// # Return
/// The attributes needed for on/off devices
fn on_off_attribute() -> Value {
    serde_json::json!({
        "commandOnlyOnOff": false,
        "queryOnlyOnOff": false
    })
}

/// Gets all the attributes needed for TV's
/// # Return
/// The attributes needed for TV's
fn tv_attribute() -> Value {
    let _tv = crate::models::tv::get_tv_state();
    serde_json::json!({
        "commandOnlyOnOff": false,
        "queryOnlyOnOff": false,
        "volumeMaxLevel": _tv.volumeMax,
        "volumeCanMuteAndUnmute": true,
        "commandOnlyVolume": false,
        "volumeDefaultPercentage": 10
    })
}

impl Device {
    /// Gets the API Url of the device, with the endpoint.
    /// # Return
    /// A formatted string we can use to send requests to.
    fn get_api_url(&self, endpoint: String) -> String {
        match self.hardware {
            HardwareType::ARDUINO => format!("http://{}/{}", self.ip, endpoint),
            _ => "".to_string(),
        }
    }

    /// Get the attributes of this device.
    /// # Return
    /// The attributes for this device.
    pub fn get_attributes(&self) -> Value {
        match self.kind {
            DeviceType::GARAGE => garage_attribute(),
            DeviceType::LIGHT | DeviceType::SWITCH | DeviceType::SPRINKLER | DeviceType::ROUTER | DeviceType::SqlSprinklerHost => on_off_attribute(),
            DeviceType::TV => tv_attribute(),
        }
    }

    /// Gets a URL to use for turning on/off relays on arduinos
    /// # Params
    /// * endpoint : The UUID of the device we want to control.
    /// * param :   The state we want to set this device to.
    /// # Return
    /// A formatted URL we can send a request to.
    pub fn get_api_url_with_param(&self, endpoint: String, param: String) -> String {
        match self.kind {
            DeviceType::SqlSprinklerHost => format!("https://api.peasenet.com/sprinkler/systems/{}/state", self.guid),
            _ => format!("{}?param={}", self.get_api_url(endpoint), param)
        }
    }

    /// Updates the device in the backend database
    /// # Return
    /// A bool representing if the update was successful.
    pub fn database_update(&self) -> bool {
        get_firebase_devices()
            .at(&self.guid)
            .unwrap()
            .set(serde_json::to_value(&self).unwrap())
            .unwrap()
            .code == StatusCode::OK
    }

    /// Gets the device type for use in google home:
    /// # Return
    /// A str representing the type of device that google home recognizes.
    pub fn get_google_device_type(&self) -> &str {
        match self.kind {
            DeviceType::LIGHT => "action.devices.types.LIGHT",
            DeviceType::SWITCH | DeviceType::SqlSprinklerHost => "action.devices.types.SWITCH",
            DeviceType::GARAGE => "action.devices.types.GARAGE",
            DeviceType::SPRINKLER => "action.devices.types.SPRINKLER",
            DeviceType::ROUTER => "action.devices.types.ROUTER",
            DeviceType::TV => "action.devices.types.TV"
        }
    }

    /// Gets a list of traits for google home that pertains to this device
    /// # Return
    /// A list (vec) of traits that this device has.
    pub fn get_google_device_traits(&self) -> Vec<&str> {
        match self.kind {
            DeviceType::GARAGE => open_close_traits(),
            DeviceType::ROUTER => reboot_traits(),
            DeviceType::TV => tv_traits(),
            _ => on_off_traits()
        }
    }

    /// Gets the hardware type for google home
    /// # Return
    /// The hardware in a nice string format.
    pub fn get_google_device_hardware(&self) -> &str {
        match self.hardware {
            HardwareType::ARDUINO => "Arduino",
            HardwareType::PI => "Raspberry Pi",
            HardwareType::OTHER => "Other",
            HardwareType::LG => "LG",
        }
    }

    /// Gets the name of this device.
    pub fn get_name(&self) -> &String {
        if &self.name == "" {
            return &self.guid;
        }
        return &self.name;
    }

    /// Converts this device into a json object that google smart home can understand.
    /// # Return
    /// A JSON representation of the device in the format that google home uses.
    pub fn to_google_device(&self) -> Value {
        let traits = self.get_google_device_traits();
        let device_type = self.get_google_device_type();
        let hardware_model = self.get_google_device_hardware();
        let attributes = self.get_attributes();
        let json = serde_json::json!({

            "id": self.guid,
            "type": device_type,
            "traits": traits,
            "name": {
                "defaultNames": [
                    self.get_name()
                ],
                "name":self.get_name(),
                "nicknames": self.nicknames
            },
            "attributes": attributes,
            "deviceInfo": {
                "manufacturer": "GTECH",
                "model": hardware_model,
                "hwVersion": "1.0",
                "swVersion": self.sw_version
            },
            "willReportState": true
        });
        json
    }

    /// Checks whether or not this device is online by pinging its IP address.
    /// # Return
    /// True if the ping was successful.
    pub fn is_online(&self) -> bool {
        let mut cmd = Command::new("ping");
        cmd
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .arg(&self.ip)
            .args(["-W", "1", "-c", "1"])
            .status()
            .unwrap()
            .success()
    }
}


/// Gets the device from the database that corresponds to the given UUID.  If the device has the following pattern:
/// xxxxxxxx-yyy-zzzzzzzzzzzz-n then we will get the device status from the SQLSprinkler host.
/// # Params
/// * `guid`  The GUID of the device we want to get.
/// # Return
/// * A device that corresponds to the given uuid, if there is no match, return a default device.
pub fn get_device_from_guid(guid: &String) -> Device {
    if check_if_zone(guid) {
        return get_zone(guid);
    }

    let device_value = get_firebase_devices()
        .at(guid)
        .unwrap()
        .get()
        .unwrap()
        .body;

    let mut dev = match serde_json::from_value(device_value) {
        Ok(d) => d,
        Err(e) => {
            println!("Err: {}", e);
            Device::default()
        }
    };

    match dev.kind {
        DeviceType::SqlSprinklerHost => {
            let ip = &dev.ip;
            dev.last_state = Value::from(get_status_from_sqlsprinkler(ip).unwrap());
            dev.database_update();
        }
        DeviceType::TV => {
            dev = tv::parse_device(dev.clone());
        }
        _ => {}
    }
    dev
}

/// Gets all of the devices that are connected to this user in the database.
/// # Return
/// * A `Vec<Device>` containing all of the device information.
pub fn get_devices_uuid(user_uuid: &String) -> Vec<Device> {
    let firebase_device_list = get_firebase_users()
        .at(&user_uuid)
        .unwrap()
        .at("devices")
        .unwrap()
        .get()
        .unwrap()
        .body;
    let device_list = device_list_from_firebase(firebase_device_list);
    device_list
}

/// Gets all the devices from firebase + any SQLSprinkler devices
fn device_list_from_firebase(body: Value) -> Vec<Device> {
    let device_guid_list: Vec<String> = serde_json::from_value(body).unwrap();
    let mut device_list = vec![];

    // Get all the devices that belong to our user and store them in a list.
    for guid in device_guid_list {
        device_list.push(get_device_from_guid(&guid));
    }

    let mut final_list = vec![];

    for _dev in device_list.clone() {
        let mut dev = _dev;

        match dev.kind {
            DeviceType::TV => {
                dev = tv::parse_device(dev.clone());
                final_list.push(dev);
            }
            DeviceType::SqlSprinklerHost => {
                final_list.push(dev.clone());
                let sprinkler_list = check_if_device_is_sqlsprinkler_host(dev.clone());
                for sprinkler in sprinkler_list {
                    final_list.push(sprinkler);
                }
            }
            _ => {
                final_list.push(dev.clone());
            }
        }
    }
    final_list
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let serialized = serde_json::to_string(&self).unwrap();
        write!(f, "{}", serialized)
    }
}

impl From<Zone> for Device {
    fn from(zone: Zone) -> Device {
        let zone_name = format!("Zone {}", &zone.system_order + 1);
        let pretty_name = format!("{}", &zone.name);
        let nicknames = vec![pretty_name, zone_name];
        Device {
            ip: "".to_string(),
            guid: zone.id.to_string(),
            kind: DeviceType::SPRINKLER,
            hardware: HardwareType::PI,
            last_state: json!({
                "on": zone.state,
                "id": zone.id,
                "index": zone.system_order
            }),
            sw_version: zone.id.to_string(),
            useruuid: "".to_string(),
            name: zone.name,
            nicknames,
        }
    }
}

impl ::std::default::Default for Device {
    fn default() -> Device {
        Device {
            ip: "".to_string(),
            guid: "".to_string(),
            kind: DeviceType::SWITCH,
            hardware: HardwareType::OTHER,
            last_state: Value::from(false),
            sw_version: "0".to_string(),
            useruuid: "".to_string(),
            name: "".to_string(),
            nicknames: vec!["".to_string()],
        }
    }
}

impl Clone for Device {
    fn clone(&self) -> Self {
        Device {
            ip: self.ip.clone(),
            guid: self.guid.clone(),
            kind: self.kind,
            hardware: self.hardware,
            last_state: self.last_state.clone(),
            sw_version: self.sw_version.clone(),
            useruuid: self.useruuid.clone(),
            name: self.name.clone(),
            nicknames: self.nicknames.clone(),
        }
    }
}

impl FromStr for HardwareType {
    type Err = ();
    fn from_str(s: &str) -> Result<HardwareType, ()> {
        match s {
            "ARDUINO" => Ok(HardwareType::ARDUINO),
            "PI" => Ok(HardwareType::PI),
            "OTHER" => Ok(HardwareType::OTHER),
            "LG" => Ok(HardwareType::LG),
            _ => Err(())
        }
    }
}


impl FromStr for DeviceType {
    type Err = ();
    fn from_str(s: &str) -> Result<DeviceType, ()> {
        match s {
            "LIGHT" => Ok(DeviceType::LIGHT),
            "SWITCH" => Ok(DeviceType::SWITCH),
            "GARAGE" => Ok(DeviceType::GARAGE),
            "SPRINKLER" => Ok(DeviceType::SPRINKLER),
            "ROUTER" => Ok(DeviceType::ROUTER),
            "SQLSPRINKLER_HOST" => Ok(DeviceType::SqlSprinklerHost),
            "TV" => Ok(DeviceType::TV),
            _ => Err(())
        }
    }
}