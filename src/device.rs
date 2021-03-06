use std::fmt;
use std::process::Command;
use std::str::FromStr;

use aa_consts::*;
use isahc::http::StatusCode;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{battery, tv};
use crate::sqlsprinkler::*;

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

/// Represents hardware types in google home
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone)]
pub enum HardwareType {
    ARDUINO,
    PI,
    OTHER,
    LG,
}

/// Represents all the different types of devices we can have / currently implemented
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone)]
pub enum DeviceType {
    BATTERY,
    LIGHT,
    SWITCH,
    GARAGE,
    SPRINKLER,
    ROUTER,
    SqlSprinklerHost,
    TV,
}

/// Gets attributes for garage doors
/// # Return
/// The attributes needed for garage doors.
fn garage_attribute() -> Value {
    serde_json::json!({
        "discreteOnlyOpenClose": true,
        "openDirection": ["UP","DOWN"]
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

fn battery_attribute() -> Value {
    serde_json::json!({
        "queryOnlyEnergyStorage": true,
        "isRechargeable": true
    })
}

/// Gets all the attributes needed for TV's
/// # Return
/// The attributes needed for TV's
fn tv_attribute() -> Value {
    serde_json::json!({
        "commandOnlyOnOff": false,
        "queryOnlyOnOff": false,
        "volumeMaxLevel": 100,
        "volumeCanMuteAndUnmute": true,
        "levelStepSize": 1,
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

    /// Get the attributes of this device. Please see https://developers.google.com/assistant/smarthome/traits/onoff#device-attributes
    /// for an example of what this is.
    /// # Example
    /// Get the attributes of a light
    /// ```
    /// use aa_models::device;
    /// let device = device::get_device_from_guid(&String::from("test_light"));
    /// println!("{:?}",device.get_attributes());
    /// let expected_attr = serde_json::json!({
    ///         "commandOnlyOnOff": false,
    ///         "queryOnlyOnOff": false
    ///     });
    /// assert_eq!(expected_attr,device.get_attributes());
    /// ```
    /// # Return
    /// The attributes for this device.
    pub fn get_attributes(&self) -> Value {
        match self.kind {
            DeviceType::GARAGE => garage_attribute(),
            DeviceType::LIGHT
            | DeviceType::SWITCH
            | DeviceType::SPRINKLER
            | DeviceType::ROUTER
            | DeviceType::SqlSprinklerHost => on_off_attribute(),
            DeviceType::TV => tv_attribute(),
            DeviceType::BATTERY => battery_attribute(),
        }
    }

    /// Gets a URL to use for turning on/off relays on Arduinos or zones in SQLSprinkler
    /// # Params
    /// * endpoint : The UUID of the device we want to control.
    /// * param :   The state we want to set this device to.
    /// # Example
    /// Get the api url for an arduino
    /// ```
    /// use aa_models::device;
    /// let device = device::get_device_from_guid(&String::from("test_light"));
    /// println!("{}",device.get_api_url_with_param(String::from("on"),String::from("true")));
    /// ```
    /// # Return
    /// A formatted URL we can send a request to.
    pub fn get_api_url_with_param(&self, endpoint: String, param: String) -> String {
        match self.kind {
            DeviceType::SqlSprinklerHost => format!(
                "https://api.peasenet.com/sprinkler/systems/{}/state",
                self.guid
            ),
            _ => format!("{}?param={}", self.get_api_url(endpoint), param),
        }
    }

    /// Updates the device in the backend database
    /// # Example
    /// Set the `test_switch` device state to true, meaning that it has been turned on. The device state is a JSON Value.
    /// ```
    /// use aa_models::device;
    /// use serde_json::Value;
    /// let mut device = device::get_device_from_guid(&String::from("test_switch"));
    ///
    /// // New device state
    /// let device_state = Value::from(true);
    /// device.last_state = device_state;
    /// let result = device.database_update();
    /// println!("Device update success: {}",result);
    /// assert!(result);
    ///
    /// // poll the device again to make sure we have updated in the db.
    /// device = device::get_device_from_guid(&String::from("test_switch"));
    /// assert!(device.last_state.as_bool().unwrap());
    ///
    /// // turn off the device
    /// device.last_state = Value::from(false);
    /// device.database_update();
    ///
    /// // Check the device yet again.
    /// device = device::get_device_from_guid(&String::from("test_switch"));
    /// assert!(!device.last_state.as_bool().unwrap());
    /// ```
    /// Set the `test_light` device state to true, meaning that it has been turned on (brightness trait added for example,
    /// not yet implemented.)
    /// ```
    /// use aa_models::device;
    /// use serde_json::json;
    /// let mut device = device::get_device_from_guid(&String::from("test_light"));
    /// // New device state
    /// device.last_state = json!({
    ///     "on": true,
    ///     "brightness": 23
    /// });
    /// let result = device.database_update();
    /// println!("Device update success: {}",result);
    /// assert!(result);
    /// ```
    /// # Return
    /// A bool representing if the update was successful.
    pub fn database_update(&self) -> bool {
        debug!("Updating device : {}", serde_json::to_value(self).unwrap());
        let success = get_firebase_devices()
            .at(&self.guid)
            .unwrap()
            .set(serde_json::to_value(&self).unwrap())
            .unwrap()
            .code
            == StatusCode::OK;
        debug!("success: {}", success);
        success
    }

    /// Gets the device type for use in google home
    /// # Examples
    /// Gets the type of the device with guid `test_switch`, which should be a SWITCH device type.
    /// ```
    /// use aa_models::device;
    /// let device = device::get_device_from_guid(&String::from("test_switch"));
    /// let device_type = device.get_google_device_type();
    /// println!("{}",device_type);
    /// assert_eq!("action.devices.types.SWITCH",device_type);
    /// ```
    /// Gets the type of the device with guid `test_light` which should be a LIGHT device type.
    /// ```
    /// use aa_models::device;
    /// let device = device::get_device_from_guid(&String::from("test_light"));
    /// let device_type = device.get_google_device_type();
    /// println!("{}",device_type);
    /// assert_eq!("action.devices.types.LIGHT",device_type);
    /// ```
    /// # Return
    /// A str representing the type of device that google home recognizes.
    pub fn get_google_device_type(&self) -> &str {
        match self.kind {
            DeviceType::LIGHT => "action.devices.types.LIGHT",
            DeviceType::SWITCH | DeviceType::SqlSprinklerHost => "action.devices.types.SWITCH",
            DeviceType::GARAGE => "action.devices.types.GARAGE",
            DeviceType::SPRINKLER => "action.devices.types.SPRINKLER",
            DeviceType::ROUTER => "action.devices.types.ROUTER",
            DeviceType::TV => "action.devices.types.TV",
            DeviceType::BATTERY => "action.devices.types.SENSOR",
        }
    }

    /// Gets a list of traits for google home that pertains to this device
    /// Please see https://developers.google.com/assistant/smarthome/traits for a list of traits
    /// Right now, the following device types have the following traits. By default, the trait is OnOff.
    /// * Garage ??? OpenClose
    /// * Router ??? Reboot
    /// * TV ??? OnOff, Volume
    ///
    /// # Examples
    /// ```
    /// use aa_models::device;
    /// let device = device::get_device_from_guid(&String::from("test_switch"));
    /// println!("{:?}",device.get_google_device_traits());
    /// assert_eq!(vec!["action.devices.traits.OnOff"],device.get_google_device_traits());
    /// ```
    /// # Return
    /// A list of traits that this device has.
    pub fn get_google_device_traits(&self) -> Vec<&str> {
        return match self.kind {
            DeviceType::GARAGE => {
                let traits: Vec<&str> = Device::open_close();
                traits
            }
            DeviceType::ROUTER => Device::reboot(),
            DeviceType::TV => {
                let mut traits: Vec<&str> = Device::on_off();
                let mut _new_traits: Vec<&str> = Device::volume();
                traits.append(&mut _new_traits);
                traits
            }
            DeviceType::BATTERY => Device::energy_storage(),
            _ => Device::on_off(),
        };
    }

    /// Gets the hardware type for google home
    ///
    /// # Examples
    /// This shows getting the hardware type for an "Other" device
    /// ```
    /// use aa_models::device;
    /// let device = device::get_device_from_guid(&String::from("test_switch"));
    /// println!("{}",device.get_google_device_hardware());
    /// assert_eq!("Other",device.get_google_device_hardware());
    /// ```
    /// This shows getting the hardware type for an Arduino device
    /// ```
    /// use aa_models::device;
    /// let device = device::get_device_from_guid(&String::from("test_light"));
    /// println!("{}",device.get_google_device_hardware());
    /// assert_eq!("Arduino",device.get_google_device_hardware());
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use aa_models::device;
    ///
    /// let device = device::get_device_from_guid(&String::from("test_switch"));
    /// println!("{}",device.get_name());
    /// assert_eq!("Test Switch",device.get_name());
    /// ```
    pub fn get_name(&self) -> &String {
        if &self.name == "" {
            return &self.guid;
        }
        return &self.name;
    }

    /// Checks whether or not this device is online by pinging its IP address.
    ///
    /// # Examples
    ///
    /// ```
    /// use aa_models::device;
    /// // Test switch as an IP of 127.0.0.1
    /// let device = device::get_device_from_guid(&String::from("test_switch"));
    /// let dev_online = device.is_online();
    /// println!("{}",dev_online);
    /// assert!(dev_online);
    /// ```
    ///
    /// # Return
    /// True if the ping was successful.
    pub fn is_online(&self) -> bool {
        let mut cmd = Command::new("ping");
        cmd.stdout(std::process::Stdio::null())
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
/// # Examples
///
/// This example covers getting a non-SQLSprinkler device
/// ```
/// use aa_models::device;
/// // Get a test device
/// let device = device::get_device_from_guid(&String::from("test_switch"));
/// //...
/// ```
/// # Params
/// * `guid`  The GUID of the device we want to get.
/// # Return
/// * A device that corresponds to the given uuid, if there is no match, return a default device.
pub fn get_device_from_guid(guid: &String) -> Device {
    if check_if_zone(guid) {
        return get_zone(guid);
    }

    let device_value = get_firebase_devices().at(guid).unwrap().get().unwrap().body;

    let mut dev = match serde_json::from_value(device_value) {
        Ok(d) => d,
        Err(e) => {
            debug!("Err: {}", e);
            Device::default()
        }
    };
    if dev == Device::default() {
        debug!("Returning default device");
        return dev;
    }
    match dev.kind {
        DeviceType::SqlSprinklerHost => {
            let ip = &dev.ip;
            if dev.is_online() {
                dev.last_state = Value::from(get_status_from_sqlsprinkler(ip).unwrap());
                dev.database_update();
            }
        }
        DeviceType::TV => {
            dev = tv::parse_device(dev.clone());
        }
        DeviceType::BATTERY => {
            dev = battery::parse_device(dev.clone());
        }
        _ => {}
    }
    dev
}

/// Gets all of the devices that are connected to this user in the database.
///
/// # Example
///```
/// use aa_models::device;
///
/// let device_list = device::get_devices_uuid(&String::from("eoqBGbi9AHUxtOrL3xpJZDQotGP2"));
/// println!("{:?}",device_list);
/// ```
/// # Return
/// * A `Vec<Device>` containing all of the device information.
pub fn get_devices_uuid(user_uuid: &String) -> Vec<Device> {
    let firebase_device_list = get_device_list(user_uuid);
    device_list_from_firebase(serde_json::to_value(firebase_device_list).unwrap())
}

/// Gets all the GUIDs belonging to the given user uuid.
/// # Example
///```
/// use aa_models::device;
///
/// let device_list = device::get_device_list(&String::from("eoqBGbi9AHUxtOrL3xpJZDQotGP2"));
/// println!("{:?}",device_list);
/// ```
/// # Return
/// * A `Vec<Device>` containing all of the device information.
pub fn get_device_list(user_uuid: &String) -> Vec<String> {
    let firebase_device_list = get_firebase_users()
        .at(&user_uuid)
        .unwrap()
        .at("devices")
        .unwrap()
        .get()
        .unwrap()
        .body;
    let list: Vec<String> = match serde_json::from_value(firebase_device_list) {
        Ok(r) => r,
        Err(..) => vec![],
    };
    list
}

/// Sets the list of devices for the user
/// # Params
/// user_uuid: The user uuid we want to update
/// device_list: The list of device GUIDs.
/// # Example
///```
/// use aa_models::device;
/// use aa_models::device::{get_device_list,set_device_list};
/// // Create a copy of the current device list
/// let backup_list = get_device_list(&String::from("eoqBGbi9AHUxtOrL3xpJZDQotGP2"));
/// let mut list = get_device_list(&String::from("eoqBGbi9AHUxtOrL3xpJZDQotGP2"));
/// list.push(String::from("8675309"));
/// set_device_list(&String::from("eoqBGbi9AHUxtOrL3xpJZDQotGP2"),list.clone());
/// assert_ne!(&backup_list, &list);
/// set_device_list(&String::from("eoqBGbi9AHUxtOrL3xpJZDQotGP2"),backup_list.clone());
/// list = get_device_list(&String::from("eoqBGbi9AHUxtOrL3xpJZDQotGP2"));
/// assert_eq!(&backup_list,&list);
/// ```
pub fn set_device_list(user_uuid: &String, device_list: Vec<String>) -> bool {
    get_firebase_users()
        .at(&user_uuid)
        .unwrap()
        .at("devices")
        .unwrap()
        .set(serde_json::to_value(device_list).unwrap())
        .is_ok()
}

/// Adds a device to the given users account.
/// # Params
/// user_uuid: The user to add the device to.
/// device: The device we want to add.
/// # Example
///```
/// use aa_models::device;
/// use aa_models::device::{get_device_list, set_device_list, DeviceType, add_device};
///
/// let uuid = String::from("eoqBGbi9AHUxtOrL3xpJZDQotGP2");
/// let mut device = device::Device::default();
/// device.guid = String::from("test-in-testing");
/// device.kind = DeviceType::LIGHT;
/// device.useruuid = uuid.clone();
/// add_device(&uuid,device);
/// ```
pub fn add_device(user_uuid: &String, mut device: Device) {
    device.useruuid = user_uuid.parse().unwrap();
    let mut list = get_device_list(user_uuid);
    list.push(device.guid.clone());
    set_device_list(user_uuid, list);
    get_firebase_devices()
        .at(&device.guid)
        .unwrap()
        .set(serde_json::to_value(&device).unwrap())
        .unwrap();
}

/// Removes a device from the database. The user id of the given device guid must match.
/// If there is not a match, no action will be taken.
/// # Params
/// user_uuid: The user to remove the device of.
/// device_guid: The device we want to remove.
/// # Example
/// ```
/// use aa_models::device;
/// use aa_models::device::{remove_device};
///
/// let uuid = String::from("eoqBGbi9AHUxtOrL3xpJZDQotGP2");
/// let device_guid = String::from("test-in-testing");
/// remove_device(&uuid,&device_guid);
/// ```
pub fn remove_device(user_uuid: &String, device_guid: &String) -> bool {
    let device_from_guid = get_device_from_guid(device_guid);

    if device_from_guid.guid != device_guid.clone() || device_from_guid == Device::default() {
        return false;
    }

    let mut list = get_device_list(user_uuid);
    let index = list
        .iter()
        .position(|x| *x == device_from_guid.guid)
        .unwrap();
    debug!("[delete] Device index is {}", index);
    list.remove(index);
    set_device_list(user_uuid, list);
    get_firebase_devices()
        .at(device_guid)
        .unwrap()
        .remove()
        .unwrap()
        .is_success()
}

/// Gets all the devices from firebase + any SQLSprinkler devices
fn device_list_from_firebase(body: Value) -> Vec<Device> {
    let device_guid_list: Vec<String> = match serde_json::from_value(body) {
        Ok(res) => res,
        Err(..) => vec![String::from("")],
    };
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
                // Only get the sprinkler system list if the device is online.
                if dev.is_online() {
                    let sprinkler_list = check_if_device_is_sqlsprinkler_host(dev.clone());
                    for sprinkler in sprinkler_list {
                        final_list.push(sprinkler);
                    }
                    final_list.push(dev.clone());
                } else {
                    // Force device to show as being turned off.
                    dev.last_state = serde_json::Value::Bool(false);
                }
            }
            // Push everything else.
            _ => {
                final_list.push(dev.clone());
            }
        }
    }
    final_list
}

impl fmt::Display for Device {
    /// Pretty-prints this Device as a JSON object.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let serialized = serde_json::to_string(&self).unwrap();
        write!(f, "{}", serialized)
    }
}

impl From<Zone> for Device {
    /// Converts a SQLSprinkler zone to a Device.
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
    /// Creates a clone of this device.
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

impl GoogleDevice for Device {
    /// Gets this device as a JSON value that can be used as an OnSync request for
    /// Google Home. Please see:
    /// https://developers.google.com/assistant/smarthome/reference/intent/sync#examples_1
    /// for more information on how this JSON looks like.
    ///
    /// # Example
    ///
    ///```
    /// use aa_models::device;
    /// use aa_models::device::GoogleDevice;
    /// let device = device::Device::default();
    /// println!("{}",device.google_smarthome_json());
    ///```
    fn google_smarthome_json(&self) -> Value {
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
}

impl DeviceTrait for Device {}

impl FromStr for HardwareType {
    type Err = ();
    fn from_str(s: &str) -> Result<HardwareType, ()> {
        match s {
            "ARDUINO" => Ok(HardwareType::ARDUINO),
            "PI" => Ok(HardwareType::PI),
            "OTHER" => Ok(HardwareType::OTHER),
            "LG" => Ok(HardwareType::LG),
            _ => Err(()),
        }
    }
}

impl FromStr for DeviceType {
    type Err = ();
    fn from_str(s: &str) -> Result<DeviceType, ()> {
        match s {
            "BATTERY" => Ok(DeviceType::BATTERY),
            "LIGHT" => Ok(DeviceType::LIGHT),
            "SWITCH" => Ok(DeviceType::SWITCH),
            "GARAGE" => Ok(DeviceType::GARAGE),
            "SPRINKLER" => Ok(DeviceType::SPRINKLER),
            "ROUTER" => Ok(DeviceType::ROUTER),
            "SQLSPRINKLER_HOST" => Ok(DeviceType::SqlSprinklerHost),
            "TV" => Ok(DeviceType::TV),
            _ => Err(()),
        }
    }
}

pub trait GoogleDevice {
    fn google_smarthome_json(&self) -> Value;
}

/// A group of traits that relate to individual devices. Each method here returns a vec to be compliance with how
/// Google Home handles the traits.
pub trait DeviceTrait {
    /// Gets all the traits that belong to a TV.
    fn volume() -> Vec<&'static str> {
        let mut arr = vec![];
        arr.push("action.devices.traits.Volume");
        arr
    }

    /// Gets all the traits that belong to opening/closing doors
    fn open_close() -> Vec<&'static str> {
        vec!["action.devices.traits.OpenClose"]
    }

    /// Gets all traits that belong to turning things on/off
    fn on_off() -> Vec<&'static str> {
        vec!["action.devices.traits.OnOff"]
    }

    /// Gets all traits that belong to things that can be rebooted
    fn reboot() -> Vec<&'static str> {
        vec!["action.devices.traits.Reboot"]
    }

    /// Gets all the traits that belong to things with energy storage
    fn energy_storage() -> Vec<&'static str> {
        vec!["action.devices.traits.EnergyStorage"]
    }
}
