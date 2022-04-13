#![no_main]
#![no_std]
#![warn(clippy::pedantic)]

use core::{fmt, panic::PanicInfo};

use rp_pico::hal::{
    uart::{common_configs::_9600_8_N_1, UartConfig},
    usb::UsbBus,
};
use usbd_serial::SerialPort;

// polling interval in microseconds
pub const SCAN_TIME: u32 = 1000;

// multiple of SCAN_TIME that a key has to be held to consider it pressed
pub const DEBOUNCE_TIME: u16 = 5;

pub const UART_CONFIG: UartConfig = _9600_8_N_1;

#[doc(hidden)]
pub struct FmtSerial<'a>(pub &'a mut SerialPort<'static, UsbBus>);

impl<'a> fmt::Write for FmtSerial<'a> {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        let mut write_ptr = s.as_bytes();
        while !write_ptr.is_empty() {
            match self.0.write(write_ptr) {
                Ok(len) => write_ptr = &write_ptr[len..],
                Err(_) => break,
            }
        }

        Ok(())
    }
}

#[macro_export]
macro_rules! println {
    ($serial:expr, $($tt:tt)*) => {match $crate::FmtSerial($serial) {
        mut serial => {
            use core::fmt::Write;
            let _ = core::write!(serial, $($tt)*);
            let _ = core::write!(serial, "\r\n");
        }
    }};
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    cortex_m::asm::udf();
}
