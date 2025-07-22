//! Command shell interface for embedded systems.
//!
//! This module provides a complete command-line interface implementation designed
//! for embedded systems and `no_std` environments. It supports command registration,
//! argument parsing, help system, and extensible command handling.
//!
//! # Features
//!
//! - **Zero-allocation**: Uses fixed-size buffers for predictable memory usage
//! - **Command Registration**: Support for both static and dynamic command registration
//! - **Argument Parsing**: Handles quoted arguments and escape sequences
//! - **Help System**: Built-in help for individual commands and command listing
//! - **Input Processing**: Character-by-character input processing with echo support
//! - **Extensible**: Easy to add custom commands and modify behavior
//!
//! # Architecture
//!
//! The shell consists of several key components:
//!
//! ```text
//! ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
//! │   Input Layer   │───▶│  Argument       │───▶│   Command       │
//! │   (Character    │    │  Parser         │    │   Registry      │
//! │   Processing)   │    │                 │    │                 │
//! └─────────────────┘    └─────────────────┘    └─────────────────┘
//!          │                       │                       │
//!          ▼                       ▼                       ▼
//! ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
//! │   Line Buffer   │    │   Argument      │    │   Command       │
//! │   Management    │    │   Storage       │    │   Execution     │
//! └─────────────────┘    └─────────────────┘    └─────────────────┘
//! ```
//!
//! # Usage Examples
//!
//! ## Basic Shell Setup
//!
//! ```rust
//! use libiot::system::shell::{Shell, Command, ShellResult};
//!
//! fn hello_command(argc: usize, argv: &[&str]) -> ShellResult {
//!     if argc > 1 {
//!         println!("Hello, {}!", argv[1]);
//!     } else {
//!         println!("Hello, World!");
//!     }
//!     ShellResult::Ok
//! }
//!
//! fn output_handler(text: &str) {
//!     print!("{}", text);
//! }
//!
//! let mut shell = Shell::new();
//! shell.set_output_function(output_handler);
//! shell.register_command("hello", "Say hello", hello_command).unwrap();
//!
//! // Process input character by character
//! let input = b"hello world\r";
//! shell.input(input).unwrap();
//! ```
//!
//! ## Static Command Registration
//!
//! ```rust
//! use libiot::system::shell::{Shell, Command, ShellResult};
//!
//! fn status_cmd(argc: usize, argv: &[&str]) -> ShellResult {
//!     println!("System status: OK");
//!     ShellResult::Ok
//! }
//!
//! fn reset_cmd(argc: usize, argv: &[&str]) -> ShellResult {
//!     println!("System reset requested");
//!     ShellResult::Ok
//! }
//!
//! const STATIC_COMMANDS: &[Command] = &[
//!     Command { name: "status", description: "Show system status", handler: status_cmd },
//!     Command { name: "reset", description: "Reset the system", handler: reset_cmd },
//! ];
//!
//! let mut shell = Shell::new();
//! shell.register_static_commands(STATIC_COMMANDS).unwrap();
//! ```
//!
//! ## Advanced Argument Parsing
//!
//! The shell supports quoted arguments and escape sequences:
//!
//! ```text
//! > echo "Hello World"           # Quoted argument with spaces
//! > config set "device name" "My Device"  # Multiple quoted arguments
//! > echo "Line 1\nLine 2"        # Escape sequences within quotes
//! > path "C:\\Program Files"     # Escaped backslashes
//! ```

use core::array;
use core::str;

/// Maximum buffer size for input command lines.
///
/// This defines the maximum length of a command line that can be processed
/// by the shell. Commands longer than this will result in a buffer overflow error.
pub const MAX_BUFFER_SIZE: usize = 256;

/// Maximum number of arguments per command.
///
/// This limits how many separate arguments (including the command name itself)
/// can be parsed from a single command line. The first argument is always
/// the command name.
pub const MAX_ARGS: usize = 16;

/// Maximum number of dynamic commands that can be registered.
///
/// This defines how many commands can be registered at runtime using
/// [`register_command`](Shell::register_command). Static commands registered
/// with [`register_static_commands`](Shell::register_static_commands) don't count against this limit.
pub const MAX_DYNAMIC_COMMANDS: usize = 32;

// ASCII control character constants for input processing
/// ASCII backspace character (0x08).
pub const ASCII_BACKSPACE: u8 = 0x08;
/// ASCII line feed character (0x0A).
pub const ASCII_LF: u8 = 0x0A;
/// ASCII carriage return character (0x0D).
pub const ASCII_CR: u8 = 0x0D;
/// ASCII delete character (0x7F).
pub const ASCII_DEL: u8 = 0x7F;
/// ASCII space character (0x20).
pub const ASCII_SPACE: u8 = 0x20;

/// Result type for shell operations.
///
/// This enum represents the possible outcomes of shell operations and
/// provides specific error types for different failure modes.
///
/// # Examples
///
/// ```rust
/// use libiot::system::shell::ShellResult;
///
/// fn example_command(argc: usize, argv: &[&str]) -> ShellResult {
///     if argc < 2 {
///         return ShellResult::InvalidParameter;
///     }
///     
///     if argv[1] == "error" {
///         return ShellResult::OutOfMemory;
///     }
///     
///     ShellResult::Ok
/// }
/// ```
#[derive(Debug, PartialEq)]
pub enum ShellResult {
    /// Operation completed successfully.
    Ok,
    /// Invalid parameter was provided to a command or shell operation.
    InvalidParameter,
    /// Insufficient memory to complete the operation.
    OutOfMemory,
    /// Input buffer overflow occurred.
    BufferOverflow,
}

/// Function signature for command handlers.
///
/// Command handlers receive the argument count and a slice of argument strings.
/// The first argument (argv[0]) is always the command name itself.
///
/// # Arguments
///
/// * `argc` - Number of arguments (including command name)
/// * `argv` - Array of argument strings
///
/// # Returns
///
/// A [`ShellResult`] indicating success or the type of error that occurred.
///
/// # Examples
///
/// ```rust
/// use libiot::system::shell::{ShellResult, CommandFn};
///
/// let echo_command: CommandFn = |argc, argv| {
///     for i in 1..argc {
///         print!("{} ", argv[i]);
///     }
///     println!();
///     ShellResult::Ok
/// };
/// ```
pub type CommandFn = fn(argc: usize, argv: &[&str]) -> ShellResult;

/// Function signature for output handlers.
///
/// Output handlers receive text from the shell and are responsible for
/// displaying it to the user through the appropriate output mechanism
/// (UART, LCD, etc.).
///
/// # Arguments
///
/// * `text` - Text to output to the user
///
/// # Examples
///
/// ```rust
/// use libiot::system::shell::OutputFn;
///
/// let uart_output: OutputFn = |text| {
///     // Send text to UART
///     print!("{}", text);
/// };
/// ```
pub type OutputFn = fn(&str);

/// Command structure containing metadata and handler function.
///
/// Each command consists of a name, description, and handler function.
/// Commands can be registered statically (at compile time) or dynamically
/// (at runtime).
///
/// # Examples
///
/// ```rust
/// use libiot::system::shell::{Command, ShellResult};
///
/// let help_command = Command {
///     name: "help",
///     description: "Show help information",
///     handler: |argc, argv| {
///         println!("Help system not implemented");
///         ShellResult::Ok
///     },
/// };
/// ```
#[derive(Clone)]
pub struct Command {
    /// The command name as typed by the user.
    ///
    /// Command names are case-sensitive and should be unique within
    /// the shell instance. They typically use lowercase letters and
    /// may include underscores or hyphens.
    pub name: &'static str,

    /// A brief description of what the command does.
    ///
    /// This description is displayed in help output and should be
    /// concise but informative. It's shown when the user requests
    /// help for the specific command or lists all commands.
    pub description: &'static str,

    /// The function that implements the command logic.
    ///
    /// This function is called when the user invokes the command.
    /// It receives the parsed arguments and should return a result
    /// indicating success or failure.
    pub handler: CommandFn,
}

/// Main shell structure managing input processing and command execution.
///
/// The shell handles character-by-character input processing, argument parsing,
/// command lookup, and execution. It maintains both static and dynamic command
/// registries and provides built-in help functionality.
///
/// # Examples
///
/// ```rust
/// use libiot::system::shell::Shell;
///
/// let mut shell = Shell::new();
/// shell.set_echo(true);
/// shell.set_help(true);
///
/// // Configure output handler
/// shell.set_output_function(|text| print!("{}", text));
/// ```
pub struct Shell {
    // Input buffer and parsing state
    pub(crate) buffer: [u8; MAX_BUFFER_SIZE],
    pub(crate) buffer_len: usize,

    // Argument parsing results
    pub(crate) argc: usize,
    argv_starts: [usize; MAX_ARGS],
    argv_lens: [usize; MAX_ARGS],

    // Command storage
    dynamic_commands: [Option<Command>; MAX_DYNAMIC_COMMANDS],
    pub(crate) dynamic_command_count: usize,
    pub(crate) static_commands: Option<&'static [Command]>,

    // Output function
    output_fn: Option<OutputFn>,

    // Configuration options
    pub(crate) echo_enabled: bool,
    pub(crate) list_command_enabled: bool,
    pub(crate) help_enabled: bool,
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell {
    /// Create a new shell instance with default settings.
    ///
    /// The shell is created with:
    /// - Echo enabled
    /// - Help system enabled  
    /// - List command enabled
    /// - No output function (must be set before use)
    /// - No registered commands
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libiot::system::shell::Shell;
    ///
    /// let shell = Shell::new();
    /// // Configure the shell before use
    /// ```
    pub fn new() -> Self {
        Self {
            buffer: [0; MAX_BUFFER_SIZE],
            buffer_len: 0,
            argc: 0,
            argv_starts: [0; MAX_ARGS],
            argv_lens: [0; MAX_ARGS],
            dynamic_commands: core::array::from_fn(|_| None),
            dynamic_command_count: 0,
            static_commands: None,
            output_fn: None,
            echo_enabled: true,
            list_command_enabled: true,
            help_enabled: true,
        }
    }

    /// Set the output function for shell responses.
    ///
    /// The output function is called whenever the shell needs to send
    /// text to the user, including command echoes, help text, and
    /// error messages.
    ///
    /// # Arguments
    ///
    /// * `output_fn` - Function to handle shell output
    ///
    /// # Returns
    ///
    /// * [`ShellResult::Ok`] - Output function set successfully
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libiot::system::shell::Shell;
    ///
    /// let mut shell = Shell::new();
    ///
    /// // Set up UART output
    /// shell.set_output_function(|text| {
    ///     // Send to UART hardware
    ///     print!("{}", text);
    /// });
    /// ```
    pub fn set_output_function(&mut self, output_fn: OutputFn) -> ShellResult {
        self.output_fn = Some(output_fn);
        ShellResult::Ok
    }

    /// Enable or disable command echoing.
    ///
    /// When echo is enabled, the shell displays characters as they are
    /// typed and provides visual feedback for backspace operations.
    /// This is typically enabled for interactive use but may be disabled
    /// for automated input processing.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable command echoing
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libiot::system::shell::Shell;
    ///
    /// let mut shell = Shell::new();
    ///
    /// // Disable echo for automated testing
    /// shell.set_echo(false);
    ///
    /// // Enable echo for interactive use
    /// shell.set_echo(true);
    /// ```
    pub fn set_echo(&mut self, enabled: bool) {
        self.echo_enabled = enabled;
    }

    /// Enable or disable the built-in list command.
    ///
    /// When enabled, the shell provides a built-in "list" command that
    /// displays all available commands. This is useful for command
    /// discovery but can be disabled if the functionality is not needed
    /// or conflicts with a custom command.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable the list command
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libiot::system::shell::Shell;
    ///
    /// let mut shell = Shell::new();
    ///
    /// // Disable built-in list command to implement custom version
    /// shell.set_list_command(false);
    /// ```
    pub fn set_list_command(&mut self, enabled: bool) {
        self.list_command_enabled = enabled;
    }

    /// Enable or disable help functionality.
    ///
    /// When enabled, commands can be invoked with `-h` or `--help` flags
    /// to display their description. This adds automatic help support
    /// to all registered commands.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable help functionality
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libiot::system::shell::Shell;
    ///
    /// let mut shell = Shell::new();
    ///
    /// // Disable help for minimal shell implementation
    /// shell.set_help(false);
    ///
    /// // Enable help (default)
    /// shell.set_help(true);
    /// ```
    pub fn set_help(&mut self, enabled: bool) {
        self.help_enabled = enabled;
    }

    /// Register a dynamic command at runtime.
    ///
    /// Dynamic commands are stored in the shell's internal memory and
    /// can be registered after the shell is created. The number of
    /// dynamic commands is limited by [`MAX_DYNAMIC_COMMANDS`].
    ///
    /// # Arguments
    ///
    /// * `name` - Command name (must not be empty)
    /// * `description` - Command description for help text
    /// * `handler` - Function to handle command execution
    ///
    /// # Returns
    ///
    /// * [`ShellResult::Ok`] - Command registered successfully
    /// * [`ShellResult::InvalidParameter`] - Empty command name provided
    /// * [`ShellResult::OutOfMemory`] - Maximum dynamic commands exceeded
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libiot::system::shell::{Shell, ShellResult};
    ///
    /// let mut shell = Shell::new();
    ///
    /// shell.register_command("uptime", "Show system uptime", |argc, argv| {
    ///     println!("System uptime: {} seconds", 12345);
    ///     ShellResult::Ok
    /// }).unwrap();
    /// ```
    pub fn register_command(
        &mut self,
        name: &'static str,
        description: &'static str,
        handler: CommandFn,
    ) -> ShellResult {
        if name.is_empty() {
            return ShellResult::InvalidParameter;
        }

        if self.dynamic_command_count >= MAX_DYNAMIC_COMMANDS {
            return ShellResult::OutOfMemory;
        }

        let command = Command {
            name,
            description,
            handler,
        };

        self.dynamic_commands[self.dynamic_command_count] = Some(command);
        self.dynamic_command_count += 1;

        ShellResult::Ok
    }

    /// Register static commands defined at compile time.
    ///
    /// Static commands are stored as a reference to an external array
    /// and don't consume shell memory. This is the preferred method
    /// for registering large numbers of commands or commands that
    /// are known at compile time.
    ///
    /// # Arguments
    ///
    /// * `commands` - Array of static commands to register
    ///
    /// # Returns
    ///
    /// * [`ShellResult::Ok`] - Commands registered successfully
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libiot::system::shell::{Shell, Command, ShellResult};
    ///
    /// const COMMANDS: &[Command] = &[
    ///     Command {
    ///         name: "version",
    ///         description: "Show firmware version",
    ///         handler: |_, _| {
    ///             println!("Firmware v1.0.0");
    ///             ShellResult::Ok
    ///         },
    ///     },
    ///     Command {
    ///         name: "info",
    ///         description: "Show device information",
    ///         handler: |_, _| {
    ///             println!("Device: IoT Controller");
    ///             ShellResult::Ok
    ///         },
    ///     },
    /// ];
    ///
    /// let mut shell = Shell::new();
    /// shell.register_static_commands(COMMANDS).unwrap();
    /// ```
    pub fn register_static_commands(&mut self, commands: &'static [Command]) -> ShellResult {
        self.static_commands = Some(commands);
        ShellResult::Ok
    }

    /// Process input data character by character.
    ///
    /// This is the main input processing function that handles character
    /// echoing, line editing (backspace), and command execution when
    /// a complete line is received.
    ///
    /// # Arguments
    ///
    /// * `data` - Input data to process (typically from UART or keyboard)
    ///
    /// # Returns
    ///
    /// * [`ShellResult::Ok`] - Input processed successfully
    /// * [`ShellResult::BufferOverflow`] - Input line too long
    ///
    /// # Character Handling
    ///
    /// - **CR/LF**: Triggers command parsing and execution
    /// - **Backspace/Delete**: Removes last character with visual feedback
    /// - **Printable ASCII**: Added to input buffer with optional echo
    /// - **Control characters**: Ignored (except CR, LF, backspace, delete)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libiot::system::shell::Shell;
    ///
    /// let mut shell = Shell::new();
    /// shell.set_output_function(|text| print!("{}", text));
    ///
    /// // Process individual characters
    /// shell.input(b"h").unwrap();
    /// shell.input(b"e").unwrap();
    /// shell.input(b"l").unwrap();
    /// shell.input(b"p").unwrap();
    /// shell.input(b"\r").unwrap();  // Execute command
    ///
    /// // Or process entire strings
    /// shell.input(b"list\n").unwrap();
    /// ```
    pub fn input(&mut self, data: &[u8]) -> ShellResult {
        for &byte in data {
            match byte {
                ASCII_CR | ASCII_LF => {
                    if self.echo_enabled {
                        self.output(if byte == ASCII_CR { "\r" } else { "\n" });
                    }
                    self.process_command();
                    self.reset_buffer();
                }
                ASCII_BACKSPACE | ASCII_DEL => {
                    if self.buffer_len > 0 {
                        self.buffer_len -= 1;
                        self.buffer[self.buffer_len] = 0;
                        if self.echo_enabled {
                            self.output("\x08 \x08"); // Backspace, space, backspace
                        }
                    }
                }
                _ => {
                    if byte >= 0x20 && byte < 0x7F {
                        // Printable ASCII
                        if self.buffer_len < MAX_BUFFER_SIZE - 1 {
                            self.buffer[self.buffer_len] = byte;
                            self.buffer_len += 1;

                            if self.echo_enabled {
                                let ch = [byte];
                                if let Ok(s) = str::from_utf8(&ch) {
                                    self.output(s);
                                }
                            }
                        } else {
                            return ShellResult::BufferOverflow;
                        }
                    }
                }
            }
        }

        ShellResult::Ok
    }

    /// Send output through the configured output function.
    ///
    /// This is an internal function used by the shell to send text to
    /// the user. It calls the output function set by [`set_output_function`](Self::set_output_function).
    ///
    /// # Arguments
    ///
    /// * `text` - Text to send to the output function
    pub(crate) fn output(&self, text: &str) {
        if let Some(output_fn) = self.output_fn {
            output_fn(text);
        }
    }

    /// Reset the input buffer and parsing state.
    ///
    /// This internal function clears the input buffer and resets all
    /// parsing state, preparing for the next command line.
    pub(crate) fn reset_buffer(&mut self) {
        self.buffer.fill(0);
        self.buffer_len = 0;
        self.argc = 0;
        self.argv_starts.fill(0);
        self.argv_lens.fill(0);
    }

    /// Parse the current buffer into arguments.
    ///
    /// This internal function implements the argument parsing logic,
    /// including support for quoted arguments and escape sequences.
    /// The parsing handles:
    ///
    /// - Space-separated arguments
    /// - Quoted arguments with spaces: `"hello world"`
    /// - Escape sequences: `\"`, `\\`, `\n`, `\t`, `\r`
    /// - Mixed quoted and unquoted arguments
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Parsing completed successfully
    /// * `Err(ShellResult)` - Parsing failed
    ///
    /// # Examples
    ///
    /// ```text
    /// echo hello world                    # 3 args: ["echo", "hello", "world"]
    /// config "device name" value          # 3 args: ["config", "device name", "value"]
    /// echo "Line 1\nLine 2"              # 2 args: ["echo", "Line 1\nLine 2"]
    /// path "C:\\Program Files\\App"       # 2 args: ["path", "C:\Program Files\App"]
    /// ```
    fn parse_arguments(&mut self) -> Result<(), ShellResult> {
        if self.buffer_len == 0 {
            return Ok(());
        }

        self.argc = 0;
        let mut i = 0;

        while i < self.buffer_len && self.argc < MAX_ARGS {
            // Skip leading spaces
            while i < self.buffer_len && self.buffer[i] == ASCII_SPACE {
                i += 1;
            }

            if i >= self.buffer_len {
                break;
            }

            // Handle quoted arguments
            if self.buffer[i] == b'"' {
                i += 1; // Skip opening quote
                let arg_start = self.argc; // Store the argument index for this quoted string
                let mut write_pos = i; // Position where we write processed characters
                let read_start = i; // Remember where this argument content starts

                while i < self.buffer_len {
                    if self.buffer[i] == b'\\' && i + 1 < self.buffer_len {
                        // Handle escape sequences
                        i += 1; // Skip the backslash
                        let escaped_char = self.buffer[i];
                        match escaped_char {
                            b'"' => self.buffer[write_pos] = b'"', // Escaped quote becomes literal quote
                            b'\\' => self.buffer[write_pos] = b'\\', // Escaped backslash becomes literal backslash
                            b'n' => self.buffer[write_pos] = b'\n',  // Escaped n becomes newline
                            b't' => self.buffer[write_pos] = b'\t',  // Escaped t becomes tab
                            b'r' => self.buffer[write_pos] = b'\r', // Escaped r becomes carriage return
                            _ => {
                                // For unrecognized escape sequences, keep the escaped character as-is
                                self.buffer[write_pos] = escaped_char;
                            }
                        }
                        write_pos += 1;
                        i += 1;
                    } else if self.buffer[i] == b'"' {
                        // Found closing quote
                        self.argv_starts[self.argc] = read_start;
                        self.argv_lens[self.argc] = write_pos - read_start;
                        self.argc += 1;
                        i += 1; // Skip closing quote
                        break;
                    } else {
                        // Regular character - copy it if we're compacting due to escape sequences
                        if write_pos != i {
                            self.buffer[write_pos] = self.buffer[i];
                        }
                        write_pos += 1;
                        i += 1;
                    }
                }

                // Handle unclosed quoted strings - still add the argument
                if i >= self.buffer_len && self.argc == arg_start {
                    // We reached end of buffer without finding closing quote
                    self.argv_starts[self.argc] = read_start;
                    self.argv_lens[self.argc] = write_pos - read_start;
                    self.argc += 1;
                }
            } else {
                // Handle unquoted arguments
                let start = i;
                while i < self.buffer_len && self.buffer[i] != ASCII_SPACE {
                    if self.buffer[i] == b'"' {
                        // Quote in the middle - treat as end of argument
                        break;
                    }
                    i += 1;
                }

                self.argv_starts[self.argc] = start;
                self.argv_lens[self.argc] = i - start;
                self.argc += 1;
            }
        }

        Ok(())
    }

    /// Get an argument as a string slice.
    ///
    /// This internal function retrieves a parsed argument by index.
    /// Arguments are zero-indexed, with argv[0] being the command name.
    ///
    /// # Arguments
    ///
    /// * `index` - Argument index (0 = command name, 1+ = arguments)
    ///
    /// # Returns
    ///
    /// * `Some(arg)` - Argument string at the specified index
    /// * `None` - Index out of bounds
    fn get_arg(&self, index: usize) -> Option<&str> {
        if index >= self.argc {
            return None;
        }

        let start = self.argv_starts[index];
        let len = self.argv_lens[index];

        if start + len <= self.buffer_len {
            str::from_utf8(&self.buffer[start..start + len]).ok()
        } else {
            None
        }
    }

    /// Process the current command after parsing.
    ///
    /// This internal function handles the complete command processing pipeline:
    /// 1. Parse arguments from the input buffer
    /// 2. Check for help flags (`-h`, `--help`)
    /// 3. Look up the command in dynamic and static registries
    /// 4. Execute the command handler
    /// 5. Handle built-in commands (like `list`)
    /// 6. Display error messages for unknown commands
    fn process_command(&mut self) {
        if let Err(_) = self.parse_arguments() {
            self.output("Error parsing command\r\n");
            return;
        }

        if self.argc == 0 {
            return;
        }

        let command_name = match self.get_arg(0) {
            Some(name) => name,
            None => return,
        };

        // Check for help flag
        if self.help_enabled && self.argc == 2 {
            if let Some(arg) = self.get_arg(1) {
                if arg == "-h" || arg == "--help" {
                    self.show_command_help(command_name);
                    return;
                }
            }
        }

        // Look for command in dynamic commands
        let mut found = false;
        for i in 0..self.dynamic_command_count {
            if let Some(ref cmd) = self.dynamic_commands[i] {
                if cmd.name == command_name {
                    let mut argv = [""; MAX_ARGS];
                    for j in 0..self.argc {
                        argv[j] = self.get_arg(j).unwrap_or("");
                    }
                    (cmd.handler)(self.argc, &argv[..self.argc]);
                    found = true;
                    break;
                }
            }
        }

        // Look for command in static commands
        if !found {
            if let Some(static_commands) = self.static_commands {
                for cmd in static_commands {
                    if cmd.name == command_name {
                        let mut argv = [""; MAX_ARGS];
                        for j in 0..self.argc {
                            argv[j] = self.get_arg(j).unwrap_or("");
                        }
                        (cmd.handler)(self.argc, &argv[..self.argc]);
                        found = true;
                        break;
                    }
                }
            }
        }

        // Handle built-in commands
        if !found {
            if self.list_command_enabled && command_name == "list" {
                self.list_commands();
                found = true;
            }
        }

        if !found {
            if self.list_command_enabled {
                self.output("Unknown command. Type 'list' to see available commands.\r\n");
            } else {
                self.output("Unknown command.\r\n");
            }
        }
    }

    /// Show help for a specific command.
    ///
    /// This internal function displays the description of a specific command
    /// when the user requests help with `-h` or `--help` flags.
    ///
    /// # Arguments
    ///
    /// * `command_name` - Name of the command to show help for
    fn show_command_help(&self, command_name: &str) {
        let mut found = false;

        // Check dynamic commands
        for i in 0..self.dynamic_command_count {
            if let Some(ref cmd) = self.dynamic_commands[i] {
                if cmd.name == command_name {
                    self.output(cmd.description);
                    self.output("\r\n");
                    found = true;
                    break;
                }
            }
        }

        // Check static commands
        if !found {
            if let Some(static_commands) = self.static_commands {
                for cmd in static_commands {
                    if cmd.name == command_name {
                        self.output(cmd.description);
                        self.output("\r\n");
                        found = true;
                        break;
                    }
                }
            }
        }

        if !found {
            self.output("Command not found.\r\n");
        }
    }

    /// List all available commands with descriptions.
    ///
    /// This internal function implements the built-in `list` command that
    /// displays all registered commands along with their descriptions.
    /// Commands are displayed in the order they were registered.
    fn list_commands(&self) {
        self.output("Available commands:\r\n");

        // List dynamic commands
        for i in 0..self.dynamic_command_count {
            if let Some(ref cmd) = self.dynamic_commands[i] {
                self.output(cmd.name);
                self.output("\t\t");
                self.output(cmd.description);
                self.output("\r\n");
            }
        }

        // List static commands
        if let Some(static_commands) = self.static_commands {
            for cmd in static_commands {
                self.output(cmd.name);
                self.output("\t\t");
                self.output(cmd.description);
                self.output("\r\n");
            }
        }
    }
}
