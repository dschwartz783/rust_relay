use std::{fs::{File, exists}, io::{Read, Result}, net::Ipv4Addr, path::Path, process::exit, thread::sleep, time::Duration, u8};

use gpiocdev::{line::Value};
use log::{error, info};
use surge_ping::ping;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    const LINE: u32 = 26;
    const CHIP: &str = "/dev/gpiochip4";
    const IPHONE_IP: [u8; 4] = [192, 168, 125, 97];

    info!("RELAY STARTUP. MONITORING IP: {IPHONE_IP:?}");
    info!("OUTPUT ON LINE {LINE} ON CHIP {CHIP}");

    let override_path = Path::new("/relay_override");
    let mut ping_fail_count = 0;

    // Code below will change this value as needed, and the line before the last will change the output to this value on line LINE
    let mut value = Value::Active;

    let req_result = gpiocdev::Request::builder()
        .on_chip(CHIP)
        .with_line(LINE)
        .as_output(value)
        .request();

    match req_result {
        Ok(_) => {
            info!("REQUEST CREATED");
        }

        Err(_) => {
            error!("FAILED TO CREATE REQUEST");
            exit(1)
        }
    }

    loop {
        if exists(override_path)? {
            info!("OVERRIDE FILE FOUND");
            let override_file = &mut File::open(override_path)?;
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
            match ping(std::net::IpAddr::V4(Ipv4Addr::from_octets(IPHONE_IP)), &[0]).await {
                Ok(ping_result) => {
                    info!("Device found!!! {ping_result:?}");
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

        let _ = req_result.as_ref().unwrap().set_value(LINE, value);

        sleep(Duration::from_secs(10));
    }
}
