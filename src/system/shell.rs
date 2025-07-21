use core::array;
use core::str;

/// Maximum buffer size for input commands
pub const MAX_BUFFER_SIZE: usize = 256;
/// Maximum number of arguments per command
pub const MAX_ARGS: usize = 16;
/// Maximum number of dynamic commands
pub const MAX_DYNAMIC_COMMANDS: usize = 32;

/// ASCII control characters
pub const ASCII_BACKSPACE: u8 = 0x08;
pub const ASCII_LF: u8 = 0x0A;
pub const ASCII_CR: u8 = 0x0D;
pub const ASCII_DEL: u8 = 0x7F;
pub const ASCII_SPACE: u8 = 0x20;

/// Result type for shell operations
#[derive(Debug, PartialEq)]
pub enum ShellResult {
    Ok,
    InvalidParameter,
    OutOfMemory,
    BufferOverflow,
}

/// Function signature for command handlers
pub type CommandFn = fn(argc: usize, argv: &[&str]) -> ShellResult;

/// Function signature for output handlers
pub type OutputFn = fn(&str);

/// Command structure
#[derive(Clone)]
pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
    pub handler: CommandFn,
}

/// Main shell structure
pub struct Shell {
    // Input buffer and parsing
    pub(crate) buffer: [u8; MAX_BUFFER_SIZE],
    pub(crate) buffer_len: usize,

    // Argument parsing
    pub(crate) argc: usize,
    argv_starts: [usize; MAX_ARGS],
    argv_lens: [usize; MAX_ARGS],

    // Commands
    dynamic_commands: [Option<Command>; MAX_DYNAMIC_COMMANDS],
    pub(crate) dynamic_command_count: usize,
    pub(crate) static_commands: Option<&'static [Command]>,

    // Output function
    output_fn: Option<OutputFn>,

    // Configuration
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
    /// Create a new shell instance
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

    /// Set the output function for shell responses
    pub fn set_output_function(&mut self, output_fn: OutputFn) -> ShellResult {
        self.output_fn = Some(output_fn);
        ShellResult::Ok
    }

    /// Enable or disable command echoing
    pub fn set_echo(&mut self, enabled: bool) {
        self.echo_enabled = enabled;
    }

    /// Enable or disable the built-in list command
    pub fn set_list_command(&mut self, enabled: bool) {
        self.list_command_enabled = enabled;
    }

    /// Enable or disable help functionality
    pub fn set_help(&mut self, enabled: bool) {
        self.help_enabled = enabled;
    }

    /// Register a dynamic command
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

    /// Register static commands
    pub fn register_static_commands(&mut self, commands: &'static [Command]) -> ShellResult {
        self.static_commands = Some(commands);
        ShellResult::Ok
    }

    /// Process input data
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

    /// Send output through the configured output function
    pub(crate) fn output(&self, text: &str) {
        if let Some(output_fn) = self.output_fn {
            output_fn(text);
        }
    }

    /// Reset the input buffer
    pub(crate) fn reset_buffer(&mut self) {
        self.buffer.fill(0);
        self.buffer_len = 0;
        self.argc = 0;
        self.argv_starts.fill(0);
        self.argv_lens.fill(0);
    }

    /// Parse the current buffer into arguments
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

            let start = i;

            // Handle quoted arguments
            if self.buffer[i] == b'"' {
                i += 1; // Skip opening quote
                let arg_start = i;

                while i < self.buffer_len {
                    if self.buffer[i] == b'\\' && i + 1 < self.buffer_len {
                        i += 2; // Skip escaped character
                    } else if self.buffer[i] == b'"' {
                        self.argv_starts[self.argc] = arg_start;
                        self.argv_lens[self.argc] = i - arg_start;
                        self.argc += 1;
                        i += 1; // Skip closing quote
                        break;
                    } else {
                        i += 1;
                    }
                }
            } else {
                // Handle unquoted arguments
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

    /// Get an argument as a string slice
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

    /// Process the current command
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

    /// Show help for a specific command
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

    /// List all available commands
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
