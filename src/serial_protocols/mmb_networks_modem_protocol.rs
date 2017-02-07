extern crate byteorder;
extern crate serial;

use zigbee_modem::ZigbeeModem;
use serial_protocols::serial_port_parser::SerialPortParser;
use std::fmt;
use std::mem;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;
use std::ops;
use std::collections::HashMap;
use serial::SerialPort;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::rc::Weak;
use std::rc::Rc;
use std::cell::RefCell;
use zigbee_serial_port::ZigbeeSerialPort;

macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

// https://mmbnetworks.atlassian.net/wiki/display/SPRHA17/Protocol+Architecture

struct MmbZigbeeModemError{
    error: &'static str
}
impl MmbZigbeeModemError{
	fn new(message: &'static str) -> MmbZigbeeModemError {
		trace!("{}", message);
		MmbZigbeeModemError{
			error: message
		}
	}
}
impl fmt::Debug for MmbZigbeeModemError{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "MmbModem: Error!: {}", self.error)
	}
}

const START_OF_FRAME: u8 = 0xF1;
const HEADER_SIZE: usize = 5;
const CHECKSUM_SIZE: usize = 2;

#[derive(Debug, PartialEq, Eq, Hash)]
enum PrimaryHeader {
    UTILITY_HEADER = 0x55,
    NETWORK_COMMISSIONING_HEADER = 0x01,
    SECURITY_CONFIG_HEADER  = 0x02,
    ZIGBEE_SUPPORT_CONFIG_HEADER = 0x03,
    ZDO_MESSAGES_HEADER = 0x04,
    ZCL_MESSAGES_HEADER = 0x05,
    GENERAL_CLUSTERS_HEADER = 0x11,
    HA_CLUSTERS_HEADER = 0x12,
    BOOTLOAD_HEADER = 0x0B,
    OTA_BOOTLOAD_HEADER = 0xB0,
    DIAGNOSTICS_HEADER = 0xD1,
}
impl From<u8> for PrimaryHeader {
	fn from(num: u8) -> PrimaryHeader {
		unsafe{ mem::transmute(num) }
	}
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum HeaderUtilities {
    RESET = 0x00,
    MODULE_INFO_REQUEST = 0x02,
    MODULE_INFO_RESPONSE = 0x03,
    BOOTLOADER_VERSION_REQUEST = 0x04,
    BOOTLOADER_VERSION_RESPONSE = 0x05,
    APPLICATION_VERSION_COUNT_REQUEST = 0x06,
    APPLICATION_VERSION_COUNT_RESPONSE = 0x07,
    APPLICATION_VERSION_REQUEST = 0x08,
    APPLICATION_VERSION_RESPONSE = 0x09,
    RESTORE_DEFAULTS = 0x10,
    HOST_STARTUP_READY = 0x20,
    STARTUP_SYNC_REQUEST = 0x21,
    STARTUP_SYNC_COMPLETE = 0x22,
    ANTENNA_CONFIGURATION_REQUEST = 0x23,
    ANTENNA_CONFIGURATION_RESPONSE = 0x24,
    ANTENNA_CONFIGURATION_WRITE = 0x25,
    LED_CONFIGURATION_REQUEST = 0x26,
    LED_CONFIGURATION_RESPONSE = 0x27,
    LED_CONFIGURATION_WRITE = 0x28,
    SERIAL_ACK_CONFIG_WRITE = 0x30,
    SERIAL_ACK_CONFIG_REQUEST = 0x31,
    SERIAL_ACK_CONFIG_RESPONSE = 0x32,
    MANUFACTURER_ID_REQUEST = 0x40,
    MANUFACTURER_ID_RESPONSE = 0x41,
    MANUFACTURER_ID_WRITE = 0x42,
    SLEEPY_PARAMETERS_REQUEST_CMD = 0x50,
    SLEEPY_PARAMETERS_RESPONSE_CMD = 0x51,
    SLEEPY_PARAMETERS_WRITE_CMD = 0x52,
    SLEEPY_HIBERNATE_DURATION_REQUEST_CMD = 0x53,
    SLEEPY_HIBERNATE_DURATION_RESPONSE_CMD = 0x54,
    SLEEPY_HIBERNATE_DURATION_WRITE_CMD = 0x55,
    STATUS_RESPONSE = 0x80,
    ERROR = 0xE0,
}
impl From<u8> for HeaderUtilities {
	fn from(num: u8) -> HeaderUtilities {
		unsafe{ mem::transmute(num) }
	}
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum HeaderNetworkCommissioning {
    JOIN_NETWORK = 0x00,
    FORM_NETWORK = 0x01,
    PERMIT_JOIN = 0x03,
    LEAVE_NETWORK = 0x04,
    REJOIN_NETWORK = 0x05,
    NETWORK_STATUS_REQUEST = 0x08,
    NETWORK_STATUS_RESPONSE = 0x09,
    TRUST_CENTER_DEVICE_UPDATE = 0x10,
    NETWORK_AUTO_JOIN = 0x11,
    NETWORK_RESET_AUTO_JOIN = 0x12,
}
impl From<u8> for HeaderNetworkCommissioning {
	fn from(num: u8) -> HeaderNetworkCommissioning {
		unsafe{ mem::transmute(num) }
	}
}


// TODO Type system is killing me... :(
#[derive(Debug, PartialEq, Eq, Hash)]
enum HeaderNothing {
    UNKNOWN = 0xFF
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum SecondaryHeader{
    HeaderUtilities(HeaderUtilities),
    HeaderNetworkCommissioning(HeaderNetworkCommissioning),
    HeaderNothing(HeaderNothing)
}

impl SecondaryHeader {
	fn from(primary_header: &PrimaryHeader, num: u8) -> SecondaryHeader {
        match primary_header {
            &PrimaryHeader::NETWORK_COMMISSIONING_HEADER => SecondaryHeader::HeaderNetworkCommissioning(HeaderNetworkCommissioning::from(num)),
            &PrimaryHeader::UTILITY_HEADER => SecondaryHeader::HeaderUtilities(HeaderUtilities::from(num)),
            _ => SecondaryHeader::HeaderNothing(HeaderNothing::UNKNOWN)
        }
	}
}



// TODO: So when we process the message, we would like to emit Zigbee events that will be
// related with the nature of the message.
// Before rolling my own event-drvien implementation, I need to research and find something
// that I can reuse.
struct Header {
	start_of_frame: u8,
	primary_header: PrimaryHeader,
	secondary_header: SecondaryHeader,
	frame_seq_number: u8,
	payload_length: i32
}

impl Header {
	fn new(buff: &[u8]) -> Result<Header, MmbZigbeeModemError> {
		if buff[0] != START_OF_FRAME {
			Err(MmbZigbeeModemError::new("Message Format error: Can't find the START_OF_FRAME byte"))
		}else{

			Ok(Header{
				start_of_frame: START_OF_FRAME,
				primary_header: PrimaryHeader::from(buff[1]),
				secondary_header: SecondaryHeader::from(&PrimaryHeader::from(buff[1]), buff[2]),
				frame_seq_number: buff[3],
				payload_length: buff[4] as i32
			})
		}
	}
}

struct MmbZigbeeModemMessage{
	header: Header,
	payload: Vec<u8>,
	checksum: [u8;2]
}
impl MmbZigbeeModemMessage {
	fn new(buff: &[u8]) -> Result<MmbZigbeeModemMessage, MmbZigbeeModemError>{
		let mut offset = HEADER_SIZE;
		let res = Header::new(&buff[0..offset]);
		if let Err(error) = res {
			return Err(error);
		}
		let header = res.unwrap();
		if header.payload_length as usize != buff.len() - (HEADER_SIZE + CHECKSUM_SIZE) {
			return Err(MmbZigbeeModemError::new("Message format error: The size of the message is different than expected"));
		}
		let payload = &buff[offset..offset + (header.payload_length - 1) as usize];
		offset += (header.payload_length - 1) as usize;
        // TODO: Check the checksum
		let checksum = [buff[offset], buff[offset + 2]];

		Ok(MmbZigbeeModemMessage {
			header: header,
			payload: payload.to_vec(),
			checksum: checksum
		})
	}
}
impl fmt::Debug for MmbZigbeeModemMessage {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "MmbModem: [Header][Payload][CRC]" )
	}
}

struct MessageHandler;
impl MessageHandler{
    fn startup(mut serial_port: Rc<RefCell<ZigbeeSerialPort>>, msg: &MmbZigbeeModemMessage) -> Result<(), String> {
        let mut _serial_port = serial_port.borrow_mut();
        match _serial_port.write("12345".as_bytes()){
            Ok(bytes_written) => {
                trace!("{} bytes writen!!", bytes_written);
                Ok(())
            },
            Err(e) => {
                trace!("Error!!: e = {:?}", e);
                Err(e.to_string())
            }

        }
    }

    fn form_network(msg: &MmbZigbeeModemMessage) -> Result<(),String>{
        Ok(())
    }

    fn join_network(msg: &MmbZigbeeModemMessage) -> Result<(),String>{
        Ok(())
    }

    fn not_implemented() -> Result<(),String>{
        Err("Message handler not implemented!".to_string())
    }
}


pub enum MmbZigbeeModemState {
    UNINITIALIZED,
    INITIALIZING,
    INITIALIZED
}



/******************/


pub struct MmbZigbeeModemProtocol {
    state: MmbZigbeeModemState,
    serial_port: Option<Rc<RefCell<ZigbeeSerialPort>>>
}
impl MmbZigbeeModemProtocol {
    pub fn new()-> MmbZigbeeModemProtocol {
        MmbZigbeeModemProtocol {
            state: MmbZigbeeModemState::UNINITIALIZED,
            serial_port: None
        }
    }

    fn process(&self, msg: &MmbZigbeeModemMessage) -> Result<(), String> {
        match (&msg.header.primary_header, &msg.header.secondary_header) {
            (&PrimaryHeader::UTILITY_HEADER, &SecondaryHeader::HeaderUtilities(HeaderUtilities::STARTUP_SYNC_REQUEST)) => {
                /*let mut _serial_port = match self.serial_port {
                    Some(ref port) => port.borrow_mut(),
                    None => return Err("No serial port!!".to_string())
                };
                MessageHandler::startup(&_serial_port, &msg)*/
                let mut _serial_port = self.serial_port.clone().unwrap();
                MessageHandler::startup(_serial_port, &msg)
            },
            (&PrimaryHeader::UTILITY_HEADER, &SecondaryHeader::HeaderUtilities(HeaderUtilities::STARTUP_SYNC_COMPLETE)) => {
                Ok(())
            },
            (&PrimaryHeader::NETWORK_COMMISSIONING_HEADER, &SecondaryHeader::HeaderNetworkCommissioning(HeaderNetworkCommissioning::FORM_NETWORK))  => {
                MessageHandler::form_network(&msg)
            },
            (&PrimaryHeader::NETWORK_COMMISSIONING_HEADER, &SecondaryHeader::HeaderNetworkCommissioning(HeaderNetworkCommissioning::JOIN_NETWORK))  => {
                MessageHandler::join_network(&msg)
            },
            (&PrimaryHeader::NETWORK_COMMISSIONING_HEADER, &SecondaryHeader::HeaderNetworkCommissioning(HeaderNetworkCommissioning::LEAVE_NETWORK))  => {
                MessageHandler::not_implemented()
            },
            (&PrimaryHeader::NETWORK_COMMISSIONING_HEADER, &SecondaryHeader::HeaderNetworkCommissioning(HeaderNetworkCommissioning::NETWORK_AUTO_JOIN))  => {
                MessageHandler::not_implemented()
            },
            _ => Err("Unknown header!!".to_string())
        }
    }

    fn write(&mut self, buff: &[u8]) -> Result<usize, Error> {
        trace!("Sending: {:?} to modem", buff);
        match self.serial_port {
            Some(ref fd) => {
                fd.borrow_mut().write(buff)
            }
            None => Err(Error::new(ErrorKind::Other, "Serial port to write not found!"))
        }
    }

	fn print(buff: &[u8]){
		for byte in buff {
			trace!("0x{:X} " , byte);
		}
	}
}
impl SerialPortParser for MmbZigbeeModemProtocol {
    fn parse(&self, buff : &[u8]) -> Result<(),()> {
		Self::print(buff);
		let mmb_msg = MmbZigbeeModemMessage::new(buff).unwrap();
        //self.process(&mmb_msg);
        match self.process(&mmb_msg) {
            Ok(_) => Ok(()),
            Err(msg) => {
                error!("Error parsing message from the UART: {}", msg);
                Err(())
            }
        }
    }

    fn set_serial_port(&mut self,  serial_port: Rc<RefCell<ZigbeeSerialPort>>) {
        self.serial_port = Some(serial_port);
    }

}
