use core::fmt;
use volatile::Volatile;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(
        Writer::new(
            ColorCode::new(
                Color::Cyan,
                Color::Black
            )
        )
    );
}

const BUFFER_WIDTH: usize = 80;
const BUFFER_HEIGHT: usize = 25;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// A screen character in the VGA text buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

/// A combination of a foreground and a background color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode( 
            (background as u8) << 4 | (foreground as u8) 
        )
    }
}

/// Represents the standard color palette in VGA text mode
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// A writer type that allows writing ASCII bytes and strings to an underlying `Buffer`.
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    /// Creates a new Writer which writes to the VGA text buffer
    fn new(color_code: ColorCode) -> Self {
        Writer {
            color_code,
            column_position: 0,
            buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        }
    }

    /// Writes a string to the VGA text buffer
    /// 
    /// Simply writes each byte of the given string,
    /// using the write_byte method
    pub fn write_string(&mut self, s: &str) {
        for b in s.bytes() {
            self.write_byte(b)
        }
    }

    /// Writes the given byte to the VGA text buffer
    /// 
    /// If the byte is not printable (not in the range 0x20 to 0x7e), 
    /// the character code 0xfe is written.
    /// The newline character inserts a new line.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n'       => self.new_line(),
            0x20..=0x7e => self.write_regular_byte(byte),
            _           => self.write_regular_byte(0xfe),
        }
    }

    fn write_regular_byte(&mut self, byte: u8) {
        if self.column_position >= BUFFER_WIDTH {
            self.new_line();
        }

        let row = BUFFER_HEIGHT - 1;
        let col = self.column_position;

        let character = ScreenChar {
            ascii_character: byte,
            color_code: self.color_code,
        };

        self.buffer.chars[row][col].write(character);

        self.column_position += 1;
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let c = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(c);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };

        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

/// Prints to the VGA text buffer
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

/// Prints to the VGA text buffer, appending a newline
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn println_can_print_a_single_line() {
        println!("output");
    }

    #[test_case]
    fn println_can_print_many_lines() {
        for i in 0..200 {
            println!("output line {}", i);
        }
    }

    #[test_case]
    fn writing_lines_has_correct_output() {
        use core::fmt::Write;
        use x86_64::instructions::interrupts;

        let line = "A string that can fit in a single line";

        interrupts::without_interrupts(|| {
            let mut writer = WRITER.lock();

            writeln!(writer, "\n{}", line).expect("writeln failed");

            let screen_text = &writer.buffer.chars;

            assert!(
                line.chars()
                .enumerate()
                .all(|(i, c)| {
                    let screen_char = screen_text[BUFFER_HEIGHT - 2][i].read();
                    let screen_char = char::from(screen_char.ascii_character);
                    c == screen_char
                })
            );
            assert!(are_all_blanks(&screen_text[BUFFER_HEIGHT - 2][line.len()..]));
            assert!(are_all_blanks(&screen_text[BUFFER_HEIGHT - 1]));
        });
    }

    fn are_all_blanks(screen_chars: &[Volatile<ScreenChar>]) -> bool {
        screen_chars
        .iter()
        .map(|sc| char::from(sc.read().ascii_character))
        .all(|c| c == ' ')
    }
}