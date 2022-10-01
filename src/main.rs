// sudo cat /etc/NetworkManager/system-connections/aaa.nmconnection
// [connection]
// id=aaa
// uuid=<xxx>
// type=wifi
// interface-name=wlo1
// permissions=

// [wifi]
// mac-address-blacklist=
// mode=infrastructure
// ssid=aaa

// [wifi-security]
// auth-alg=open
// key-mgmt=wpa-psk
// psk=testabc

use ini::Ini;
use std::{
    fmt::Formatter,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
struct WifiConfig {
    ssid: String,
    password: String,
    fname: String,
    uuid: String,
}

impl WifiConfig {
    fn init(entry_path: PathBuf) -> Option<Self> {
        let filename = entry_path.file_name()?.to_str()?;
        let nmconnection = Ini::load_from_file(&entry_path).ok()?;
        let mut ssid = nmconnection.section(Some("wifi"))?.get("ssid")?;
        let uuid = nmconnection.section(Some("connection"))?.get("uuid")?;
        let connection_id = nmconnection.section(Some("connection"))?.get("id")?;
        let psk = nmconnection
            .section(Some("wifi-security"))
            .map(|x| x.get("psk").unwrap_or(""))
            .unwrap_or("");
        if ssid.contains(&";") {
            ssid = connection_id;
        }
        Some(WifiConfig {
            ssid: ssid.to_owned(),
            uuid: uuid.to_owned(),
            password: psk.to_owned(),
            fname: filename.to_owned(),
        })
    }
}

fn main() {
    let system_connections_dir = "/etc/NetworkManager/system-connections/";
    println!("[*] searching WIFI info in {} ...", system_connections_dir);
    for entry in fs::read_dir(system_connections_dir).expect("read dir error") {
        let entry_path = entry.unwrap().path();
        if let Some(wifi_config) = WifiConfig::init(entry_path) {
            println!(
                "{}: {}",
                wifi_config.ssid,
                if wifi_config.password.is_empty() {
                    "<EMPTY>"
                } else {
                    &wifi_config.password
                }
            );
        }
    }
    println!("[*] searching over, bye");
}
