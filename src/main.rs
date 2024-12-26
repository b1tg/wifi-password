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
    fn init_from_netplan(entry_path: PathBuf) -> Option<Self> {
        let filename = entry_path.file_name()?.to_str()?;
        let content = fs::read_to_string(&entry_path).expect("try sudo");
        let mut ssid = "".to_string();
        let mut psk = "".to_string();
        for line in content.lines() {
            if line.starts_with("            name") {
                ssid = line[19..line.len() - 1].to_string();
            }
            if line.starts_with("            password: ") {
                psk = line[23..line.len() - 1].to_string();
            }
        }
        if ssid.is_empty() {
            return None;
        }

        Some(WifiConfig {
            ssid: ssid.to_owned(),
            uuid: "".to_owned(),
            password: psk.to_owned(),
            fname: filename.to_owned(),
        })
    }
    fn init_from_nm(entry_path: PathBuf) -> Option<Self> {
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
    fn init_from_netsh() -> Option<Vec<WifiConfig>> {
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
    WifiConfig::init_from_netsh();
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
    if let Ok(o) = fs::exists(system_connections_dir) {
        if o {
            println!("[*] searching WIFI info in {} ...", system_connections_dir);
            for entry in fs::read_dir(system_connections_dir).expect("read dir error") {
                let entry_path = entry.unwrap().path();
                if let Some(wifi_config) = WifiConfig::init_from_nm(entry_path) {
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
        }
    }
    let netplan_dir = "/etc/netplan/";
    if let Ok(o) = fs::exists(system_connections_dir) {
        if o {
            println!("[*] searching WIFI info in {} ...", netplan_dir);
            for entry in fs::read_dir(netplan_dir).expect("read dir error") {
                let entry_path = entry.unwrap().path();
                if let Some(wifi_config) = WifiConfig::init_from_netplan(entry_path) {
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
        }
    }
    println!("[*] searching over, bye");
}
// /etc/netplan/90-NM-56ffe0d9-aaf8-48ce-8c05-f108689d03ba.yaml
#[test]
fn test_netplan() {
    let netplan_dir = "/etc/netplan/";
    println!("[*] searching WIFI info in {} ...", netplan_dir);
    for entry in fs::read_dir(netplan_dir).expect("read dir error") {
        let entry_path = entry.unwrap().path();
        if let Some(wifi_config) = WifiConfig::init_from_netplan(entry_path) {
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
