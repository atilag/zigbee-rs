extern crate byteorder;

use serial_port_parser::SerialPortParser;
use std::fmt;
use std::mem;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;
use std::ops;


enum HeaderFields {
	// Bit 8 (base 1)
	IsResponseOrCommand =	0b10000000,
	// Bit 7 (base 1)
	FromModemOrHost =		0b01000000,
	// Bit 6 (base 1)
	IsNormalOrBypass =		0b00100000,
}

enum HeaderMessageTypes {
	GenericDataInOut =		0b00000,
	ZdoZdp = 				0b00001,
	TrustCenterAuthDevice = 0b00010,
	TrustCenterGetEntry =	0b00011,
	RegisterEndPoint =		0b00100,
	InterPan =				0b00101,
	EspBackend = 			0b10000,
	UartTunnel =			0b11100,
	DevUtilsLite =			0b11101,
	DeviceConfig =			0b11110,
	ProtocolVersion =		0b11111
}

/*
enum ModemMessages {
	GenericDataIn =				0b00000,
	ZdoZdp = 					0b00001,
	TrustCenterUpdateDevice =	0b00010,
	InterPan =					0b00101,
	EspBackendLogData = 		0b10000,
	UartTunnel =				0b11100
}*/

enum MessageTypes {
	GenericDataOutMsg =			0x00,
	GenericDataOutConfirm =		0x80,
	ZdoZdpReq =					0x01,
	ZdoZdpRes =					0x81,
	TrustCenterAuthDeviceReq =	0x02,
	TrustCenterAuthDeviceRes =	0x82,
	TrustCenterGetEntryReq = 	0x03,
	TrustCenterGetEntryRes =	0x83,
	RegisterEndPointReq =		0x04,
	DeregisterEndPointReq =		0x88,
	InterPanMsg =				0x05,
	InterPanConfirm =			0x85,
	BackupRestorePanReq =		0x0a,
	BackupRestorePanRes =		0x8a,
	BackupEntryReq =			0x0b,
	BackupEntryRes =			0x8b,
	// ...
	GenericDataInMsg =			0x40
}

impl From<u8> for MessageTypes {
	fn from(num: u8) -> MessageTypes {
		unsafe{ mem::transmute(num) }
	}
}

enum AddressMode {
	Indirect =	0x00,
	Group =		0x01,
	Network =	0x02,
	Eui =		0x03
}

impl From<u8> for AddressMode {
	fn from(num: u8) -> AddressMode {
		unsafe{ mem::transmute(num) }
	}
}

enum AddressLength {
	Zero,
	TwoBytes{ address: u16 },
	FourBytes{ address: u64 }
}

impl AddressMode{
	fn address_length_type( mode: AddressMode ) -> AddressLength {
		match mode {
			AddressMode::Indirect => AddressLength::Zero,
			AddressMode::Group | AddressMode::Network => AddressLength::TwoBytes{address: 0},
			AddressMode::Eui => AddressLength::FourBytes{address: 0}
		}
	}

	fn address_length( mode: AddressMode ) -> usize {
		match mode {
			AddressMode::Indirect => 0,
			AddressMode::Group | AddressMode::Network => 2,
			AddressMode::Eui => 8
		}
	}

}


// TODO Lo mismo no son tan comunes.... por lo visto cambiar el orden de los campos en funcion
// del tipo de mensaje
struct CommonMsgFields<T,S>{
	msg_type: u8,
	destination_address_mode: u8,
	destination_address: T,
	destination_endpoint: u8,
	source_address_mode: u8,
	source_address: S,
	source_endpoint: u8,
	profile_id: u16, // little endian
	cluster_id: u16, // little endian
	link_quality: u8,
	was_broadcast: u8,
	security_status: u8
}

struct CommonMsgFields2<T>{
	msg_type: u8,
	destination_address_mode: u8,
	destination_address: T,
	profile_id: u16, // little endian
	destination_endpoint: u8,
	cluster_id: u16, // little endian
	source_endpoint: u8,
	tx_options: u8
}

enum MessageBody {
	Nothing,
	GenericDataInMsg {
		common_fields: CommonMsgFields<AddressLength,AddressLength>,
		asdu_length: u8, /* Payload length */
		asdu: Vec<u8> /* Payload */
	},
	GenericDataOutMsg {
		common_fields: CommonMsgFields2<AddressLength>,
		asdu_length: u8,
		asdu: Vec<u8>
	},
}

pub struct DevelcoZigbeeModemMessage {
	header: Vec<u8>,
	body: MessageBody
}

impl <'a> DevelcoZigbeeModemMessage {
	/*pub fn new(buff: Vec<u8>) -> Result<DevelcoZigbeeModemMessage, DevelcoZigbeeModemError<'a>> {
		match MessageTypes::from(buff[3]) {
			MessageTypes::GenericDataInMsg => {
				let destination_address_length = AddressMode::address_length(AddressMode::from(buff[5]));
				let source_address_length = AddressMode::address_length(AddressMode::from(buff[8+destination_address_length]));
				let common_fields = CommonMsgFields {
					msg_type: buff[3],
					destination_address_mode: buff[4],
					destination_address: match destination_address_length {
						8 => Cursor::new(&buff[5..5+destination_address_length]).read_u64::<LittleEndian>().unwrap(),
						2 | _ => Cursor::new(&buff[5..5+destination_address_length]).read_u16::<LittleEndian>().unwrap() as u64
					},
					destination_endpoint: buff[6 + dErr(DevelcoZigbeeModemError{
					error: "Unknown message type!"
				})estination_address_length],
					source_address_mode: buff[7 + destination_address_length],
					source_address: match source_address_length {
						8 =>  Cursor::new(&buff[9+destination_address_length..9+destination_address_length+source_address_length])
								.read_u64::<LittleEndian>().unwrap(),
						2 | _ =>  Cursor::new(&buff[9+destination_address_length..9+destination_address_length+source_address_length])
									.read_u16::<LittleEndian>().unwrap() as u64
					},
					source_endpoint: buff[9 + destination_address_length+source_address_length],
					profile_id: Cursor::new(&buff[10+destination_address_length+source_address_length..12+destination_address_length+source_address_length])
								  .read_u16::<LittleEndian>().unwrap(),
					cluster_id: Cursor::new(&buff[10+destination_address_length+source_address_length..12+destination_address_length+source_address_length])
								  .read_u16::<LittleEndian>().unwrap(),
					link_quality: buff[14 + destination_address_length+source_address_length],
					was_broadcast: buff[15 + destination_address_length+source_address_length],
					security_status: buff[16 + destination_address_length+source_address_length]
				};

				let body = MessageBody::GenericDataInMsg{
					common_fields: common_fields,
					asdu_length: buff[17 + destination_address_length+source_address_length],
					asdu: buff[18+destination_address_length+source_address_length..].to_vec()
				};

				Ok(DevelcoZigbeeModemMessage {
					header: buff[0..4].to_vec(),
					body: body
				})
			},

			MessageTypes::GenericDataOutMsg => {
				let body = MessageBody::Nothing{};
				Ok(DevelcoZigbeeModemMessage {
					header: buff[0..4].to_vec(),
					body: body
				})
			},
			_ => {
				error!("Unknown message type!");
				Err(DevelcoZigbeeModemError{
					error: "Unknown message type!"
				})
			}
		};

		Err(DevelcoZigbeeModemError{
			error: "Unknown message type!"
		})
	}*/


	pub fn new(buff: Vec<u8>) -> Result<DevelcoZigbeeModemMessage, DevelcoZigbeeModemError<'a>> {
		match MessageTypes::from(buff[3]) {
			MessageTypes::GenericDataInMsg => {
				let destination_address_length = AddressMode::address_length_type(AddressMode::from(buff[5]));
				let source_address_length = AddressMode::address_length_type(AddressMode::from(buff[8+mem::size_of::<destination_address>()]));


				let common_fields : CommonMsgFields<source_address_length, destination_address_length>;
				let asdu_length = buff[mem::size_of::<common_fields>() + 1];
				let asdu = vec![asdu_length;0];
				let msg = MessageBody::GenericDataInMsg{
					common_fields: common_fields,
					asdu_length: asdu_length,
					asdu: asdu
				};

				unsafe{
					msg.common_fields = ptr::read(buff.as_ptr() as *const MessageBody::GenericDataInMsg);
				}

			},
			MessageTypes::GenericDataOutMsg => {
				let body = MessageBody::Nothing{};

				Ok()
			},
			_ => {
				error!("Unknown message type!");
				Err(DevelcoZigbeeModemError{
					error: "Unknown message type!"
				})
			}
		};
		Err(DevelcoZigbeeModemError{
			error: "Unknown message type!"
		})
	}
}


pub struct DevelcoZigbeeModemError<'a>{
	error: &'a str
}

pub struct DevelcoZigbeeModemProtocol<'a> {
	pub foo: &'a str
}

impl <'a> SerialPortParser for DevelcoZigbeeModemProtocol<'a>{
	type Message = DevelcoZigbeeModemMessage;
	type Error = DevelcoZigbeeModemError<'a>;
	fn parse(&self, buff : Vec<u8>)	-> Result<DevelcoZigbeeModemMessage, DevelcoZigbeeModemError<'a>> {

		/********************/
		for byte in &buff {
			print!("0x{:X} " , byte);
		}
		println!("END");
		/*******************/

		/*let mut msg = DevelcoZigbeeModemMessage{
			header: vec![0;3]
		};
		msg.header.clone_from_slice(&buff[..3]);*/

		let msg = DevelcoZigbeeModemMessage::new(buff);
		msg
	}
}

impl fmt::Debug for DevelcoZigbeeModemMessage {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "DevelcoModem: [Field][Filed2][AnotherField][][][..]" )
	}
}

impl <'a> fmt::Debug for DevelcoZigbeeModemError<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "DevelcoModem: Error!: {}", self.error)
	}
}
