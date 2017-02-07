extern crate serial;

use std::process::*;
use std::os::unix::io::*;
use std::io::Read;
use std::io::Write;
use std::io;
use mio::*;
use mio::unix::EventedFd;
use serial::posix::TTYPort;
use serial::SerialPort;
use std::rc::Rc;
use std::cell::RefCell;
use std::cell::Ref;
use libc;

const SETTINGS: serial::PortSettings = serial::PortSettings {
    baud_rate:    serial::Baud115200,
    char_size:    serial::Bits8,
    parity:       serial::ParityNone,
    stop_bits:    serial::Stop1,
    flow_control: serial::FlowNone
};

#[derive(Clone)]
pub struct ZigbeeSerialPort {
    fd : Rc<RefCell<TTYPort>>,
}
impl ZigbeeSerialPort {
    pub fn new(device: String) -> ZigbeeSerialPort {
        trace!("Opening dev {}", device);
        let mut port = match serial::open(&device) {
            Err(e) => {
                error!("Couldn't open the port!!. Error: {}", e);
                exit(-2);
            },
            Ok(p) => p
        };

        trace!("Device opened in fd {}", port.as_raw_fd());

        match port.configure(&SETTINGS) {
            Err(e) => {
                error!("Couldn't configure the port!!. Error: {}", e);
                exit(-3);
            },
            Ok(_) => {
                ZigbeeSerialPort{
                    fd: Rc::new(RefCell::new(port)),
                }
            }
        }
    }

    pub fn get_fd(&self) -> RawFd{
        (*self.fd.borrow()).as_raw_fd()
    }

}
impl Write for ZigbeeSerialPort {
    fn write(&mut self, buff: &[u8]) -> Result<usize, io::Error>{
        trace!("ZigbeeSerialPort::write() called!");
        self.fd.borrow_mut().write(buff)
    }
    fn flush(&mut self) -> Result<(),io::Error> {
        trace!("ZigbeeSerialPort::flush() called!");
        self.fd.borrow_mut().flush()
    }
}
impl Read for ZigbeeSerialPort {
    fn read(&mut self, buff: &mut [u8]) -> Result<usize, io::Error> {
        trace!("ZigbeeSerialPort::Read() called!");
        self.fd.borrow_mut().read(buff)
    }

}
