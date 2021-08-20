use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::models::device::Device;

/// A struct representing the command output for getting the tv volume
#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct VolState {
    pub muted: bool,
    pub returnValue: bool,
    pub scenario: String,
    pub volume: u8,
    pub volumeMax: u8,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct TvState {
    pub on: bool,
    pub muted: bool,
    pub volume: u8,
    pub volumeMax: u8,
}

impl ::std::default::Default for TvState {
    fn default() -> Self {
        TvState {
            on: false,
            muted: false,
            volume: 0,
            volumeMax: 0,
        }
    }
}


/// Checks to see if the given device is a TV, if so, add the fields required for TV.
/// # Param
/// * dev : The Device we want to check to see if it is a TV.
/// # Return
/// True if the device is a TV, false otherwise.
pub fn parse_device(mut dev: Device) -> Device {
    if dev.kind == crate::models::device::DeviceType::TV {
        let is_online = dev.is_online();
        // !!! ONLY QUERY TV WHEN IT IS ON !!!
        if is_online {
            dev.last_state = serde_json::json!(get_tv_state());
        } else {
            dev.last_state = serde_json::json!(TvState::default())
        }
        dev.database_update();
        return dev.clone();
    }
    dev.clone()
}

/// Allows setting TV volume to value
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct SetVolState(u8);

/// Allows toggling mute of TV
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct SetMuteState(bool);

/// Allows turning on/off TV.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct SetPowerState(bool);

/// The output of the requests to the tv.
#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct ReturnVal {
    returnValue: bool,
}

/// Sets the volume state of the TV to the given VolState
/// # Param
/// `state` A SetVolState containing the integer value of the volume we want to set the tv to.
/// # Return
/// The success of the command
pub fn set_volume_state(state: SetVolState) -> bool {
    let mut set_vol_command = Command::new("upstairs-tv");
    let vol_output = set_vol_command.arg("set")
        .arg("vol")
        .arg(state.0.to_string())
        .output().unwrap()
        .stdout;
    let vol_return_str = String::from_utf8(vol_output).unwrap();
    let vol_return: ReturnVal = serde_json::from_str(vol_return_str.as_str()).unwrap();
    vol_return.returnValue
}

/// Sets the power of the TV to the requested value (true/on - false/off)
pub fn set_power_state(state: bool) -> bool {
    Command::new("upstairs-tv")
        .arg("set")
        .arg("power")
        .arg(state.to_string())
        .status()
        .unwrap()
        .success()
}

/// Sets the volume state of the TV to the given VolState
/// # Param
/// `state` A SetMuteState containing the integer value of the volume we want to set the tv to.
/// # Return
/// The success of the command
pub fn set_mute_state(state: SetMuteState) -> bool {
    Command::new("upstairs-tv")
        .arg("set")
        .arg("mute")
        .arg(state.0.to_string())
        .status()
        .unwrap()
        .success()
}

/// Gets the volume states from the TV.
/// # Return
/// A VolState struct containing all of the information for the volume of the TV.
pub fn get_tv_state() -> TvState {
    let mut output = Command::new("upstairs-tv");
    output.arg("get")
        .arg("vol");

    let is_success = output.status().unwrap().success();
    if is_success {
        let data = String::from_utf8(output.output().unwrap().stdout).unwrap();
        let vol_state: VolState = serde_json::from_str(data.as_str()).unwrap();
        return TvState {
            on: true,
            muted: vol_state.muted,
            volume: vol_state.volume,
            volumeMax: vol_state.volumeMax,
        };
    }
    TvState::default()
}