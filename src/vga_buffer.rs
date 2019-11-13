use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

/// The 16 color standard palette in VGA text mode.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0x0,
    Blue = 0x1,
    Green = 0x2,
    Cyan = 0x3,
    Red = 0x4,
    Magenta = 0x5,
    Brown = 0x6,
    LightGray = 0x7,
    DarkGray = 0x8,
    LightBlue = 0x9,
    LightGreen = 0xA,
    LightCyan = 0xB,
    LightRed = 0xC,
    Pink = 0xD,
    Yellow = 0xE,
    White = 0xF,
}

/// A combination of a foreground and background color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    /// Create a `ColorCode` with the given foreground and background colors.
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// A VGA text buffer character, consisting of an ASCII code point and a `ColorCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

/// The height of the VGA text buffer (commonly 25 lines).
const BUFFER_HEIGHT: usize = 25;
/// The width of the VGA text buffer (commonly 80 columns).
const BUFFER_WIDTH: usize = 80;

/// A structure representing the VGA text mode buffer.
#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// A writer type that allows writing ASCII bytes and strings to an underlying `Buffer`.
///
/// Wraps lines at `BUFFER_WIDTH`. Supports newline characters and implements `core::fmt::Write`.
pub struct Writer {
    row_position: usize,
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    /// Writes an ASCII byte to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = self.row_position;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    /// Writes an ASCII string to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character.  Replaces non-ASCII
    /// characters with pink hearts <3.
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range -> print a pink ♥
                // assumes the original IBM PC character set (code page 437)
                _ => {
                    let old_color_code = self.color_code;
                    self.color_code = ColorCode::new(Color::Pink, Color::DarkGray);
                    self.write_byte(0x03);
                    self.color_code = old_color_code;
                }
            }
        }
    }

    /// Advances one line (row) and returns to the first column *unless* already on the last row in
    /// which case shifts all lines up by one (discarding the contents of the first row) and clears
    /// the last row.
    fn new_line(&mut self) {
        if self.row_position >= BUFFER_HEIGHT - 1 {
            for row in 1..BUFFER_HEIGHT {
                for col in 0..BUFFER_WIDTH {
                    let character = self.buffer.chars[row][col].read();
                    self.buffer.chars[row - 1][col].write(character);
                }
            }
            self.clear_row(BUFFER_HEIGHT - 1);
        } else {
            self.row_position += 1;
        }
        self.column_position = 0;
    }

    /// Clears a row by overwriting it with space characters.
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

lazy_static! {
    /// A global `Writer` instance for printing to the VGA text mode buffer.
    ///
    /// Used by the `print!` and `println!` macros.
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        row_position: 0,
        column_position: 0,
        color_code: ColorCode::new(Color::LightGray, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

/// Like the `print!` macro in `std`, but prints to the VGA text buffer.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

/// Like the `println!` macro in `std`, but prints to the VGA text buffer.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Prints the given formatted string to the VGA text buffer through the global `WRITER` instance.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
