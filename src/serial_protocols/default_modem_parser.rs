pub struct DefaultModemParser {
}

impl DefaultModemParser {
}

impl SerialPortParser for DefaultModemParser {
    pub fn parse<T>(&self, buff : Vec<u8>) -> Result<T> {
		Ok(())
	}
}
