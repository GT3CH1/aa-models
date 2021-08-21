# Arduino Automation Models

This project contains all the models & data structures that the main project,
[Arduino Automation](https://github.com/GT3CH1/arduino-automoation), which also uses the
[Arduino Automation Daemon](https://github.com/GT3CH1/aa-daemon) project to serve HTTP requests.

## What in the world is this for?

Well, it's primarily used for integration into Google Home. I have been working on a solution that allows me to easily
create and manage my IOT devices, which I have also hand-created. This thought was originally completely housed in
Google's Firebase
(which this project CAN and is using as a backend, it originally used MySQL). Not that firebase is bad or anything, but
deploying code to Firebase proved to be too much of a hassle over time.

## What Google Home Traits/Device Types are currently supported?

As of now, the following devices and their traits are supported

<table style="text-align: left">
    <tr>
        <th>Device Type</th>
        <th>Traits</th>
    </tr>
    <tr>
        <td>Switches</td><td>OnOff</td>
    </tr>
    <tr>
        <td>Lights</td><td>OnOff</td>
    </tr>
    <tr>
        <td>Garage Doors</td><td>OpenClose</td>
    </tr>
    <tr>
        <td>Routers</td><td>Reboot</td>
    </tr>
    <tr>
        <td>LG TV's</td><td>OnOff, Volume</td>
    </tr>
</table>

For more information on what these mean, please see the
[Google Smart Home Guide](https://developers.google.com/assistant/smarthome/guides)

## TODO's

* [ ] Better device attribute handling (somehow make attribute values on the fly?)
* [ ] A way to add attributes together (ie, with TV, add two vecs together)  (probably `vec.append`)
* [ ] Better error handling

## Notice

You will notice methods in this project that make calls to
firebase, [using my fork of rust-firebase](https://github.com/GT3CH1/rust-firebase). I will __NOT__ be supplying this
code as-is as it contains my Firebase secrets. However, this is _format_ I've used.

```rust
use firebase::Firebase;

pub fn get_firebase_users() -> Firebase {
    Firebase::authed("https://<rtdb-url>.firebaseio.com", "<key>").unwrap()
}

pub fn get_firebase_devices() -> Firebase {
    Firebase::authed("https://<rtdb-url-devices>.firebaseio.com/", "<key>").unwrap()
}
```