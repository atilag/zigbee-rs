use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;
use zigbee_serial_port::ZigbeeSerialPort;

pub trait SerialPortParser{
	// type Message: fmt::Debug;
	// type Error: fmt::Debug;
	//fn parse(&self, buff : &[u8]) -> Result<Self::Message, Self::Error>;
	fn parse(&self, buff : &[u8]) -> Result<(),()>;
	fn set_serial_port(&mut self, serial_port: Rc<RefCell<ZigbeeSerialPort>>);

}
