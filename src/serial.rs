use x86::io::{outb, inb};
use spin::Mutex;
use lazy_static::lazy_static;

const COM1: u16 = 0x3F8; // First serial port

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(COM1) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

pub struct SerialPort {
    base: u16,
}

impl SerialPort {
    pub unsafe fn new(base: u16) -> SerialPort {
        SerialPort { base }
    }

    pub fn init(&mut self) {
        unsafe {
            // Disable interrupts
            outb(self.base + 1, 0x00);
            // Enable DLAB (set baud rate divisor)
            outb(self.base + 3, 0x80);
            // Set divisor to 3 (lo byte) 38400 baud
            outb(self.base + 0, 0x03);
            // (hi byte)
            outb(self.base + 1, 0x00);
            // 8 bits, no parity, one stop bit
            outb(self.base + 3, 0x03);
            // Enable FIFO, clear them, with 14-byte threshold
            outb(self.base + 2, 0xC7);
            // IRQs enabled, RTS/DSR set
            outb(self.base + 4, 0x0B);
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        unsafe {
            // Wait for transmit buffer to be empty
            while (inb(self.base + 5) & 0x20) == 0 {}
            outb(self.base, byte);
        }
    }

    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }
}

impl core::fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).expect("Printing to serial failed");
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}   