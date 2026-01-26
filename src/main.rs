use std::{fs::{DirBuilder, File, exists}, io::{Read, Result}, net::Ipv4Addr, path::Path, process::exit, thread::sleep, time::Duration, u8};
use config::{Config};
use gpiocdev::{line::Value};
use log::{error, info};
use surge_ping::ping;

// mod config;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    const CONFIG_DIR: &str = "/home/david/.config/relay/";
    let config_file: String = format!("{}config.yaml", CONFIG_DIR);

    match DirBuilder::new().create("/home/david/.config/relay/") {
        Ok(_) => {info!("INITIALIZED CONFIG DIR")}
        Err(_) => {}
    };

    let relay_config: Config;

    if !exists(&config_file).unwrap() {
        let _ = File::create_new(&config_file).unwrap();
    }

    match Config::builder()
        .add_source(config::File::new(config_file.as_str(), config::FileFormat::Yaml))
        .add_source(config::Environment::with_prefix("RELAY"))
        .build() {
            Ok(_config) => {
                println!("{:?}", _config);

                relay_config = _config
            }
            Err(e) => {
                println!("{}", e);
                exit(1);
            }
        }

    let line: u32 = relay_config.get_int("LINE").unwrap().try_into().unwrap();
    let chip: &str = &relay_config.get_string("CHIP").unwrap();
    let iphone_ip_array: Vec<u8> = relay_config.get_array("IPHONE_IP").unwrap().iter().map(|v| v.clone().into_int().unwrap() as u8).collect();
    let iphone_ip: [u8; 4] = iphone_ip_array.try_into().unwrap_or_else(|_| panic!("COULD NOT READ IPHONE_IP"));

    info!("RELAY STARTUP. MONITORING IP: {iphone_ip:?}");
    info!("OUTPUT ON LINE {line} ON CHIP {chip}");

    let override_path = Path::new("/relay_override");
    let mut ping_fail_count = 0;

    // Code below will change this value as needed, and the line before the last will change the output to this value on line LINE
    let mut value = Value::Active;

    let req_result = gpiocdev::Request::builder()
        .on_chip(chip)
        .with_line(line)
        .as_output(value)
        .request();

    match req_result {
        Ok(_) => {
            info!("REQUEST CREATED");
        }

        Err(_) => {
            error!("FAILED TO CREATE REQUEST");
            exit(1);
        }
    }

    loop {
        if exists(override_path)? {
            info!("OVERRIDE FILE FOUND");
            let override_file = &mut std::fs::File::open(override_path)?;
            let readbuf: &mut String = &mut String::new();

            override_file.read_to_string(readbuf)?;
            
            if readbuf.len() >= 1 {
                match readbuf[..1].parse::<u8>() {
                    Ok(read_int) => {
                        value = if read_int == 1 { Value::Active } else { Value::Inactive };
                        info!("STATE OVERRIDDEN. NEW STATE: {read_int}");
                    }
                    Err(e) => {
                        error!("COULD NOT PARSE OVERRIDE FILE: {e}");
                    }
                };
            }
        } else {
            match ping(std::net::IpAddr::V4(Ipv4Addr::from_octets(iphone_ip)), &[0]).await {
                Ok(ping_result) => {
                    info!("DEVICE FOUND: {ping_result:?}");
                    ping_fail_count = 0;
                    value = Value::Active;
                }
                Err(e) => {
                    error!("PING FAIL: {}", e);
                    ping_fail_count += 1;
                }
            }

            if ping_fail_count >= 120 {
                value = Value::Inactive;
            }
        }

        let _ = req_result.as_ref().unwrap().set_value(line, value);

        sleep(Duration::from_secs(10));
    }
}
