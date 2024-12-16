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

use encoding::all::GB18030;
use encoding::{DecoderTrap, Encoding};
use ini::Ini;
use regex::Regex;
use std::io::Write;
use std::process::Command;
use std::{fs, path::PathBuf};

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
    // netsh wlan show profiles
    // netsh wlan show profiles name=wifi1 key=clear
    fn fetch_win() -> Option<Vec<WifiConfig>> {
        let mut results = vec![];
        let output = run_command("netsh wlan show profiles")?;
        let re = Regex::new(r".*: (.+)$").unwrap();
        let names: Vec<&str> = output
            .split("\r\n")
            .filter_map(|x| {
                let captures = re.captures(x)?;
                if let Some(name) = captures.get(1) {
                    return Some(name.as_str());
                }
                None
            })
            .collect();
        for name in names {
            let output = run_command(&format!("netsh wlan show profiles name={} key=clear", name))?;
            let re_en = Regex::new(r".*Key Content.*: (.+)$").unwrap();
            let re_zh = Regex::new(r".*关键内容.*: (.+)$").unwrap();
            let passwords: Vec<&str> = output
                .split("\r\n")
                .filter_map(|x| {
                    let captures = re_en.captures(x)?;
                    if let Some(name) = captures.get(1) {
                        return Some(name.as_str());
                    }
                    let captures = re_zh.captures(x)?;
                    if let Some(name) = captures.get(1) {
                        return Some(name.as_str());
                    }
                    None
                })
                .collect();
            results.push(WifiConfig {
                ssid: name.to_string(),
                password: passwords.get(0).or(Some(&"<EMPTY>"))?.to_string(),
                fname: "".to_string(),
                uuid: "".to_string(),
            })
        }
        Some(results)
    }
}

fn run_command(cmd_str: &str) -> Option<String> {
    let cmd_str: Vec<&str> = cmd_str.trim().split_ascii_whitespace().collect();
    let mut command = Command::new(cmd_str.get(0)?);
    let command = command.args(cmd_str.get(1..)?);
    let stdout = command.output().ok()?.stdout;
    if let Ok(output) = String::from_utf8(stdout.clone()) {
        return Some(output);
    }
    let output = GB18030.decode(&stdout, DecoderTrap::Strict).ok()?;
    Some(output)
}
#[test]
fn test_run_command() {
    dbg!(run_command("netsh wlan show profiles"));
    dbg!(run_command("whoami"));
    assert_eq!(run_command("badbad"), None);
}
#[test]
fn test_win() {
    WifiConfig::fetch_win();
}
#[cfg(target_os = "windows")]
fn main() {
    use std::io::Read;

    println!("[*] searching WIFI info");
    if let Some(entries) = WifiConfig::fetch_win() {
        for entry in entries {
            println!("{}: {}", entry.ssid, entry.password);
        }
    }
    print!("Press Enter to continue...");
    std::io::stdout().flush().unwrap();
    std::io::stdin().read(&mut [0u8]).unwrap();
    println!("[*] searching over, bye");
}
#[cfg(target_os = "linux")]
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
