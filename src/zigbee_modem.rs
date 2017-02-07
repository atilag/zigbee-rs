extern crate serial;

use std::process::*;
use std::*;
use std::os::unix::io::*;
use mio::*;
use mio::unix::EventedFd;
use serial::SerialPort;
use std::io::Read;
use std::io::Write;
use serial_protocols::serial_port_parser::SerialPortParser;
use std::rc::Weak;
use std::rc::Rc;
use std::cell::RefCell;
use zigbee_serial_port::ZigbeeSerialPort;

pub struct ZigbeeModem<T: SerialPortParser>{
    serial_port: Rc<RefCell<ZigbeeSerialPort>>,
	token: Token,
    parser: T,
    poll: Poll,
}

impl <T: SerialPortParser> ZigbeeModem<T> {
	pub fn new(device: String, mut parser: T) -> ZigbeeModem<T> {
        let serial_port = ZigbeeSerialPort::new(device);
        ZigbeeModem{
            serial_port: Rc::new(RefCell::new(serial_port)),
			token: Token(1),
            parser: parser,
            poll: Poll::new().ok().expect("Error creating the event loop!!")
		}

	}

	pub fn run(&self){
		trace!("Starting...");
        let __refcell_serial = self.serial_port.borrow();
        let fd = __refcell_serial.get_fd();
		let evented_fd = EventedFd(&fd.as_raw_fd());
        trace!("{:?}", evented_fd);
		self.poll.register(&evented_fd, self.token, Ready::readable(), PollOpt::edge())
            .unwrap_or_else(|err|{
                error!("Error registering modem device!. Error {}", err);
		});

		let mut events = Events::with_capacity(1024);

    	loop{
        	self.poll.poll(&mut events, None).unwrap();
        	for event in events.iter() {
        		match event.token() {
        			Token(1) => {
        				match self.on_incoming_data() {
                            Err(e) => println!("Warning: Error parsing message: {:?}. Keep going...", e),
                            _ => {}
                        }
        			}
        			_ => {
        				self.on_error();
        				return;
        			}
        		}
        	}
    	}
    }

    fn parse(&self, buff: &[u8]) -> Result<(),()>{
        let msg = self.parser.parse(buff);
        trace!("Msg received: {:?}", msg.unwrap());
        Ok(())
    }

	fn on_incoming_data(&self) -> Result<(),()> {
		trace!("Got data from the modem");
	    let mut buff: Vec<u8> = vec![0;256];
        let mut _serial_port = self.serial_port.borrow_mut();
		match _serial_port.read(&mut buff[..]){
            Err(ref e) => {
                error!("Couldn't read from the serial port!!. Error = {}", e);
                Err(())
            },
            Ok(size) => {
                self.parse(&buff[0..size])
            }
        }
	}

	fn on_error(&self) {
		error!("Got an error!!");
	}
}
