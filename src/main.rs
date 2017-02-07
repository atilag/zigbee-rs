extern crate mio;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate serial;
extern crate byteorder;
#[macro_use] extern crate lazy_static;
extern crate libc;

mod zigbee_modem;
mod serial_protocols; //.serial_port_parser;
mod zigbee_serial_port;


use std::env;
//use develco_zigbee_modem_protocol::*;
use zigbee_modem::ZigbeeModem;
use serial_protocols::mmb_networks_modem_protocol::MmbZigbeeModemProtocol;

fn usage(program_name : String) -> String{
    println!("Usage:");
    println!("{} <zigbee_device> ", program_name);
    println!("e.g: {} /dev/ttyUSB0", program_name);
    std::process::exit(-1);
}

fn main() {
    let zigbee_device_name = match env::args().nth(1) {
        None         => usage(env::args().nth(0).unwrap()),
        Some(device) => device
    };
    env_logger::init().ok().expect("Error initializing loggger");
    let mut zigbee_device = zigbee_modem::ZigbeeModem::<MmbZigbeeModemProtocol>::new(zigbee_device_name, MmbZigbeeModemProtocol::new());
    zigbee_device.run();
}
