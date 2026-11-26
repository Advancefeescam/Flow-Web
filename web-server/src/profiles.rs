// The first web runtime has a single instance-owner profile. The shared
// settings helpers still use the upstream device/profile classification.
#[path = "../../src-tauri/src/profiles/device_keys.rs"]
pub mod device_keys;

use serde_json::{Value, json};

pub fn dispatch(command: &str) -> Option<Value> {
    match command {
        "profiles_list" => Some(json!({
            "profiles": [{
                "id": "web-default",
                "name": "",
                "colorKey": "",
                "avatar": {"kind": "initial", "emoji": "", "photoVersion": 0},
                "createdAt": 0,
                "splashTone": ""
            }],
            "photos": {},
            "defaultId": "web-default",
            "activeId": "web-default",
            "maxProfiles": 1,
            "askWhosWatching": false
        })),
        "profiles_take_launch_question" | "profiles_hold_link" => Some(json!(false)),
        "profiles_take_held_link" => Some(Value::Null),
        _ => None,
    }
}
