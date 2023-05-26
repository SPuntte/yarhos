use core::ops::{Deref, DerefMut};

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
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }

    fn set_foreground(&mut self, color: Color) {
        self.0 &= !0x0F;
        self.0 |= color as u8;
    }

    fn set_background(&mut self, color: Color) {
        self.0 &= !0xF0;
        self.0 |= (color as u8) << 4;
    }
}

/// A VGA text buffer character, consisting of an IBM PC code point and a `ColorCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    code_point: u8,
    color_code: ColorCode,
}

impl DerefMut for ScreenChar {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}

impl Deref for ScreenChar {
    type Target = ScreenChar;

    fn deref(&self) -> &Self::Target {
        self
    }
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

/// Controls whether `Writer` interprets control character bytes as glyphs.
#[derive(Debug, Clone, Copy)]
pub enum ControlCharMode {
    Control,
    Glyph,
}

/// A writer type that allows writing code page 437[^cp437] bytes and strings to an underlying buffer.
///
/// Wraps lines at `BUFFER_WIDTH`. Control character handling is controlled via `control_char_mode`.
/// Implements `core::fmt::Write`.
///
/// [^cp437]: https://en.wikipedia.org/wiki/Code_page_437
pub struct Writer {
    row_position: usize,
    column_position: usize,
    color_code: ColorCode,
    control_char_mode: ControlCharMode,
    buffer: &'static mut Buffer,
}

impl Writer {
    /// Sets whether control character bytes are interpreted as glyphs.
    pub fn set_control_mode(&mut self, control_char_mode: ControlCharMode) {
        self.control_char_mode = control_char_mode;
    }

    /// Sets the active `Color`s.
    pub fn set_color(&mut self, foreground: Color, background: Color) {
        self.color_code = ColorCode::new(foreground, background);
    }

    /// Sets the active foreground `Color`.
    pub fn set_fg_color(&mut self, color: Color) {
        self.color_code.set_foreground(color);
    }

    /// Sets the active background `Color`.
    pub fn set_bg_color(&mut self, color: Color) {
        self.color_code.set_background(color);
    }

    /// Writes a byte to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. The behavior regarding `\t`, `\n`, and `\r` is controlled
    /// through `set_control_mode()`.
    pub fn write_byte(&mut self, byte: u8) {
        if matches!(self.control_char_mode, ControlCharMode::Control)
            && self.handle_control_char(byte)
        {
            return;
        }

        if self.column_position >= BUFFER_WIDTH {
            self.new_line();
        }

        let row = self.row_position;
        let col = self.column_position;

        let color_code = self.color_code;
        self.buffer.chars[row][col].write(ScreenChar {
            code_point: byte,
            color_code,
        });
        self.column_position += 1;
    }

    /// Writes a string to the buffer.
    ///
    /// Interprets `s` as bytes and assumes the IBM PC character set (code page 437).
    ///
    /// Wraps lines at `BUFFER_WIDTH`. The behavior regarding `\t`, `\n`, and `\r` is controlled
    /// through `set_control_mode()`.
    pub fn write_string(&mut self, s: &str) {
        // TODO: Support conversion from Unicode via the codepage_437 crate?
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    /// Outputs the full code page 437 character set as a 16 by 16 block.
    ///
    /// Disregards but preserves current `ControlCharMode`.
    pub fn print_character_set(&mut self) {
        let ccmode_save = self.control_char_mode;
        self.control_char_mode = ControlCharMode::Glyph;
        if self.column_position > 0 {
            self.new_line();
        }
        for row in 0..0x10u8 {
            for col in 0..0x10u8 {
                self.write_byte((row << 4) | col);
            }
            self.new_line();
        }
        self.control_char_mode = ccmode_save;
    }

    /// Clears the underlying buffer with current backroung color and places the cursor at the
    /// upper left corner.
    pub fn clear(&mut self) {
        let blank = ScreenChar {
            code_point: b' ',
            color_code: self.color_code,
        };
        self.fill(blank);
        self.row_position = 0;
        self.column_position = 0;
    }

    fn fill(&mut self, character: ScreenChar) {
        for row in 0..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                self.buffer.chars[row][col].write(character);
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
            let blank = ScreenChar {
                code_point: b' ',
                color_code: self.color_code,
            };
            self.clear_row(BUFFER_HEIGHT - 1, blank);
        } else {
            self.row_position += 1;
        }
        self.column_position = 0;
    }

    /// Clears a row by overwriting it.
    fn clear_row(&mut self, row: usize, blank: ScreenChar) {
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    fn handle_control_char(&mut self, byte: u8) -> bool {
        assert!(
            matches!(self.control_char_mode, ControlCharMode::Control),
            "Writer::handle_control_char() called in glyph mode."
        );
        match byte {
            b'\t' => {
                // TODO: variable tab width
                self.write_byte(b' ');
                true
            }
            b'\n' => {
                self.new_line();
                true
            }
            b'\r' => {
                self.column_position = 0;
                true
            }
            _ => false,
        }
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
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
        control_char_mode: ControlCharMode::Control,
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
    ($($arg:tt)*) => ($crate::vga_buffer::_println(format_args!($($arg)*)));
}

/// Prints the given formatted string to the VGA text buffer through the global `WRITER` instance.
#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

#[doc(hidden)]
pub fn _println(args: core::fmt::Arguments) {
    use core::fmt::Write;

    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut w = WRITER.lock();
        w.write_fmt(args).unwrap();
        w.new_line();
    });
}

/// Sets whether `WRITER` interprets control character bytes as glyphs.
pub fn set_control_mode(control_char_mode: ControlCharMode) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().set_control_mode(control_char_mode);
    });
}

/// Sets the active `Color`s (foreground and background) for `WRITER`.
pub fn set_color(foreground: Color, background: Color) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().set_color(foreground, background);
    });
}

/// Sets the active foreground `Color` for `WRITER`.
pub fn set_fg_color(color: Color) {
    x86_64::instructions::interrupts::without_interrupts(|| WRITER.lock().set_fg_color(color));
}

/// Sets the active background `Color` for `WRITER`.
pub fn set_bg_color(color: Color) {
    x86_64::instructions::interrupts::without_interrupts(|| WRITER.lock().set_bg_color(color));
}

/// Using `WRITER`, outputs the full code page 437 character set as a 16 by 16
/// block.
pub fn print_character_set() {
    x86_64::instructions::interrupts::without_interrupts(|| WRITER.lock().print_character_set());
}

/// Clears the `WRITER` buffer with currently active background color and resets the cursor.
pub fn clear() {
    x86_64::instructions::interrupts::without_interrupts(|| WRITER.lock().clear());
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

#[test_case]
fn test_println_output() {
    use core::fmt::Write;

    let s = "The quick brown foz jumps over the lazy dog.";
    assert!(s.len() <= BUFFER_WIDTH);

    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut w = WRITER.lock();
        w.clear();
        writeln!(w, "{}", s).expect("writeln!() failed");
        for (i, c) in s.chars().enumerate() {
            let screen_char = w.buffer.chars[0][i].read();
            assert_eq!(char::from(screen_char.code_point), c);
        }
    });
}
