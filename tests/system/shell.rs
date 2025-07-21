use libiot::system::shell::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};

/// Thread-safe test output capture
static TEST_OUTPUT: OnceLock<Arc<Mutex<VecDeque<String>>>> = OnceLock::new();

fn get_test_output_buffer() -> &'static Arc<Mutex<VecDeque<String>>> {
    TEST_OUTPUT.get_or_init(|| Arc::new(Mutex::new(VecDeque::new())))
}

fn test_output_fn(text: &str) {
    let buffer = get_test_output_buffer();
    buffer.lock().unwrap().push_back(text.to_string());
}

fn get_test_output() -> String {
    let buffer = get_test_output_buffer();
    let mut buf = buffer.lock().unwrap();
    buf.drain(..).collect::<Vec<_>>().join("")
}

fn clear_test_output() {
    let buffer = get_test_output_buffer();
    buffer.lock().unwrap().clear();
}

/// Test command handlers
fn test_command_handler(_argc: usize, _argv: &[&str]) -> ShellResult {
    ShellResult::Ok
}

fn echo_command_handler(argc: usize, _argv: &[&str]) -> ShellResult {
    // This would typically output the arguments, but we can't access output here
    if argc > 1 {
        ShellResult::Ok
    } else {
        ShellResult::InvalidParameter
    }
}

fn fail_command_handler(_argc: usize, _argv: &[&str]) -> ShellResult {
    ShellResult::InvalidParameter
}

/// Test command handler that captures arguments for verification
static CAPTURED_ARGS: OnceLock<Arc<Mutex<Option<Vec<String>>>>> = OnceLock::new();

fn get_captured_args_buffer() -> &'static Arc<Mutex<Option<Vec<String>>>> {
    CAPTURED_ARGS.get_or_init(|| Arc::new(Mutex::new(None)))
}

fn capture_args_handler(argc: usize, argv: &[&str]) -> ShellResult {
    let buffer = get_captured_args_buffer();
    *buffer.lock().unwrap() = Some(argv[1..argc].iter().map(|s| s.to_string()).collect());
    ShellResult::Ok
}

fn get_captured_args() -> Vec<String> {
    let buffer = get_captured_args_buffer();
    buffer.lock().unwrap().take().unwrap_or_default()
}

fn clear_captured_args() {
    let buffer = get_captured_args_buffer();
    *buffer.lock().unwrap() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_creation() {
        let _shell = Shell::new();
        // Can only test that creation succeeds since fields are private
    }

    #[test]
    fn test_shell_default() {
        let _shell = Shell::default();
        // Can only test that creation succeeds since fields are private
    }

    #[test]
    fn test_set_output_function() {
        let mut shell = Shell::new();

        let result = shell.set_output_function(test_output_fn);
        assert_eq!(result, ShellResult::Ok);
    }

    #[test]
    fn test_set_echo() {
        let mut shell = Shell::new();

        // Test that these methods can be called - behavior tested in input tests
        shell.set_echo(false);
        shell.set_echo(true);
    }

    #[test]
    fn test_set_list_command() {
        let mut shell = Shell::new();

        // Test that these methods can be called - behavior tested in command tests
        shell.set_list_command(false);
        shell.set_list_command(true);
    }

    #[test]
    fn test_set_help() {
        let mut shell = Shell::new();

        // Test that these methods can be called - behavior tested in help tests
        shell.set_help(false);
        shell.set_help(true);
    }

    #[test]
    fn test_register_command() {
        let mut shell = Shell::new();

        let result = shell.register_command("test", "Test command", test_command_handler);
        assert_eq!(result, ShellResult::Ok);
    }

    #[test]
    fn test_register_command_empty_name() {
        let mut shell = Shell::new();

        let result = shell.register_command("", "Empty name command", test_command_handler);
        assert_eq!(result, ShellResult::InvalidParameter);
    }

    #[test]
    fn test_register_command_overflow() {
        let mut shell = Shell::new();

        // Fill up all command slots
        for i in 0..MAX_DYNAMIC_COMMANDS {
            let name = format!("cmd{}", i).leak(); // Leak the string to get 'static
            let result = shell.register_command(name, "Test command", test_command_handler);
            assert_eq!(result, ShellResult::Ok);
        }

        // Try to register one more
        let result = shell.register_command("overflow", "Overflow command", test_command_handler);
        assert_eq!(result, ShellResult::OutOfMemory);
    }

    #[test]
    fn test_register_static_commands() {
        let mut shell = Shell::new();

        static COMMANDS: [Command; 2] = [
            Command {
                name: "static1",
                description: "Static command 1",
                handler: test_command_handler,
            },
            Command {
                name: "static2",
                description: "Static command 2",
                handler: test_command_handler,
            },
        ];

        let result = shell.register_static_commands(&COMMANDS);
        assert_eq!(result, ShellResult::Ok);
    }

    #[test]
    fn test_input_printable_characters() {
        let mut shell = Shell::new();
        shell.set_output_function(test_output_fn);
        clear_test_output();

        let input = b"hello";
        let result = shell.input(input);
        assert_eq!(result, ShellResult::Ok);

        // Should echo the characters
        let echoed = get_test_output();
        assert_eq!(echoed, "hello");
    }

    #[test]
    fn test_input_no_echo() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);
        shell.set_echo(false);

        let input = b"hello";
        let result = shell.input(input);
        assert_eq!(result, ShellResult::Ok);

        // Should not echo
        let echoed = get_test_output();
        assert_eq!(echoed, "");
    }

    #[test]
    fn test_input_carriage_return() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        let input = b"test\r";
        let result = shell.input(input);
        assert_eq!(result, ShellResult::Ok);

        let echoed = get_test_output();
        assert!(echoed.contains("test"));
        assert!(echoed.contains("\r"));
    }

    #[test]
    fn test_input_line_feed() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        let input = b"test\n";
        let result = shell.input(input);
        assert_eq!(result, ShellResult::Ok);

        let echoed = get_test_output();
        assert!(echoed.contains("test"));
        assert!(echoed.contains("\n"));
    }

    #[test]
    fn test_input_backspace() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        // Type "hello" then backspace twice
        let result1 = shell.input(b"hello");
        assert_eq!(result1, ShellResult::Ok);

        let result2 = shell.input(&[ASCII_BACKSPACE, ASCII_BACKSPACE]);
        assert_eq!(result2, ShellResult::Ok);

        let echoed = get_test_output();
        assert!(echoed.contains("hello"));
        assert!(echoed.contains("\x08 \x08")); // Backspace sequence
    }

    #[test]
    fn test_input_delete() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        let result1 = shell.input(b"test");
        assert_eq!(result1, ShellResult::Ok);

        let result2 = shell.input(&[ASCII_DEL]);
        assert_eq!(result2, ShellResult::Ok);

        // Can verify through behavior - DEL should work similar to backspace
        let echoed = get_test_output();
        assert!(echoed.contains("test"));
    }

    #[test]
    fn test_input_backspace_empty_buffer() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        // Backspace on empty buffer should not cause issues
        let result = shell.input(&[ASCII_BACKSPACE]);
        assert_eq!(result, ShellResult::Ok);
    }

    #[test]
    fn test_input_buffer_overflow() {
        let mut shell = Shell::new();

        // Fill buffer to capacity
        let large_input = vec![b'a'; MAX_BUFFER_SIZE - 1];
        let result1 = shell.input(&large_input);
        assert_eq!(result1, ShellResult::Ok);

        // Try to add one more character
        let result2 = shell.input(b"x");
        assert_eq!(result2, ShellResult::BufferOverflow);
    }

    #[test]
    fn test_input_non_printable_characters() {
        let mut shell = Shell::new();

        // Control characters (except CR, LF, BS, DEL) should be ignored
        let input = &[0x01, 0x02, 0x1F]; // Non-printable ASCII
        let result = shell.input(input);
        assert_eq!(result, ShellResult::Ok);
        // Can't verify buffer length, but function should succeed
    }

    #[test]
    fn test_command_execution_dynamic() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        shell.register_command("test", "Test command", test_command_handler);

        let result = shell.input(b"test\r");
        assert_eq!(result, ShellResult::Ok);

        // Command should execute without error
        let out = get_test_output();
        assert!(!out.contains("Unknown command"));
    }

    #[test]
    fn test_command_execution_static() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        static COMMANDS: [Command; 1] = [Command {
            name: "static_test",
            description: "Static test command",
            handler: test_command_handler,
        }];

        shell.register_static_commands(&COMMANDS);

        let result = shell.input(b"static_test\r");
        assert_eq!(result, ShellResult::Ok);

        let out = get_test_output();
        assert!(!out.contains("Unknown command"));
    }

    #[test]
    fn test_unknown_command() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        let result = shell.input(b"unknown_command\r");
        assert_eq!(result, ShellResult::Ok);

        let out = get_test_output();
        assert!(out.contains("Unknown command"));
        assert!(out.contains("Type 'list'"));
    }

    #[test]
    fn test_unknown_command_no_list() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);
        shell.set_list_command(false);

        let result = shell.input(b"unknown_command\r");
        assert_eq!(result, ShellResult::Ok);

        let out = get_test_output();
        assert!(out.contains("Unknown command"));
        assert!(!out.contains("Type 'list'"));
    }

    #[test]
    fn test_list_command() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        shell.register_command("test1", "Test command 1", test_command_handler);
        shell.register_command("test2", "Test command 2", test_command_handler);

        let result = shell.input(b"list\r");
        assert_eq!(result, ShellResult::Ok);

        let out = get_test_output();
        assert!(out.contains("Available commands"));
        assert!(out.contains("test1"));
        assert!(out.contains("test2"));
        assert!(out.contains("Test command 1"));
        assert!(out.contains("Test command 2"));
    }

    #[test]
    fn test_list_command_disabled() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);
        shell.set_list_command(false);

        let result = shell.input(b"list\r");
        assert_eq!(result, ShellResult::Ok);

        let out = get_test_output();
        assert!(out.contains("Unknown command"));
    }

    #[test]
    fn test_help_command() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        shell.register_command("test", "Test command description", test_command_handler);

        let result = shell.input(b"test --help\r");
        assert_eq!(result, ShellResult::Ok);

        let out = get_test_output();
        assert!(out.contains("Test command description"));
    }

    #[test]
    fn test_help_command_short_flag() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        shell.register_command("test", "Test command description", test_command_handler);

        let result = shell.input(b"test -h\r");
        assert_eq!(result, ShellResult::Ok);

        let out = get_test_output();
        assert!(out.contains("Test command description"));
    }

    #[test]
    fn test_help_disabled() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);
        shell.set_help(false);

        shell.register_command("test", "Test command description", test_command_handler);

        let result = shell.input(b"test --help\r");
        assert_eq!(result, ShellResult::Ok);

        // Should execute command normally, not show help
        let out = get_test_output();
        assert!(!out.contains("Test command description"));
    }

    #[test]
    fn test_argument_parsing_simple() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        shell.register_command("echo", "Echo command", echo_command_handler);

        let result = shell.input(b"echo hello world\r");
        assert_eq!(result, ShellResult::Ok);
    }

    #[test]
    fn test_argument_parsing_quoted() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        shell.register_command("echo", "Echo command", echo_command_handler);

        let result = shell.input(b"echo \"hello world\" test\r");
        assert_eq!(result, ShellResult::Ok);
    }

    #[test]
    fn test_argument_parsing_escaped_quotes() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        shell.register_command("echo", "Echo command", echo_command_handler);

        let result = shell.input(b"echo \"hello \\\"world\\\" test\"\r");
        assert_eq!(result, ShellResult::Ok);
    }

    #[test]
    fn test_argument_parsing_multiple_spaces() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        shell.register_command("echo", "Echo command", echo_command_handler);

        let result = shell.input(b"echo    hello     world    \r");
        assert_eq!(result, ShellResult::Ok);
    }

    #[test]
    fn test_empty_command() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        let result = shell.input(b"\r");
        assert_eq!(result, ShellResult::Ok);

        // Empty command should not produce any error output
        let out = get_test_output();
        assert!(!out.contains("Unknown command"));
    }

    #[test]
    fn test_whitespace_only_command() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        let result = shell.input(b"   \r");
        assert_eq!(result, ShellResult::Ok);

        let out = get_test_output();
        assert!(!out.contains("Unknown command"));
    }

    #[test]
    fn test_command_with_failing_handler() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        shell.register_command("fail", "Failing command", fail_command_handler);

        let result = shell.input(b"fail\r");
        assert_eq!(result, ShellResult::Ok);

        // Command should execute and return error, but shell input should still succeed
    }

    #[test]
    fn test_mixed_input_processing() {
        let mut shell = Shell::new();
        clear_test_output();
        shell.set_output_function(test_output_fn);

        // Simulate typing "hello", backspace, "p", enter
        let result1 = shell.input(b"hello");
        assert_eq!(result1, ShellResult::Ok);

        let result2 = shell.input(&[ASCII_BACKSPACE]);
        assert_eq!(result2, ShellResult::Ok);

        let result3 = shell.input(b"p\r");
        assert_eq!(result3, ShellResult::Ok);

        let out = get_test_output();
        assert!(out.contains("hello"));
        assert!(out.contains("\x08 \x08"));
        assert!(out.contains("p"));
    }

    #[test]
    fn test_constants() {
        assert_eq!(MAX_BUFFER_SIZE, 256);
        assert_eq!(MAX_ARGS, 16);
        assert_eq!(MAX_DYNAMIC_COMMANDS, 32);
        assert_eq!(ASCII_BACKSPACE, 0x08);
        assert_eq!(ASCII_LF, 0x0A);
        assert_eq!(ASCII_CR, 0x0D);
        assert_eq!(ASCII_DEL, 0x7F);
        assert_eq!(ASCII_SPACE, 0x20);
    }

    #[test]
    fn test_shell_result_debug() {
        // Test that ShellResult implements Debug
        let result = ShellResult::Ok;
        assert_eq!(format!("{:?}", result), "Ok");

        let result = ShellResult::InvalidParameter;
        assert_eq!(format!("{:?}", result), "InvalidParameter");

        let result = ShellResult::OutOfMemory;
        assert_eq!(format!("{:?}", result), "OutOfMemory");

        let result = ShellResult::BufferOverflow;
        assert_eq!(format!("{:?}", result), "BufferOverflow");
    }

    #[test]
    fn test_shell_result_partial_eq() {
        // Test that ShellResult implements PartialEq
        assert_eq!(ShellResult::Ok, ShellResult::Ok);
        assert_ne!(ShellResult::Ok, ShellResult::InvalidParameter);
    }

    #[test]
    fn test_command_clone() {
        // Test that Command implements Clone
        let cmd = Command {
            name: "test",
            description: "Test command",
            handler: test_command_handler,
        };

        let cloned = cmd.clone();
        assert_eq!(cloned.name, "test");
        assert_eq!(cloned.description, "Test command");
    }

    #[test]
    fn test_quoted_string_missing_closing_quote_bug() {
        clear_captured_args();
        let mut shell = Shell::new();
        shell.register_command("capture", "Capture args", capture_args_handler);

        // This should handle the unclosed quote gracefully, not drop the argument
        let result = shell.input(b"capture \"hello world\r");
        assert_eq!(result, ShellResult::Ok);

        let args = get_captured_args();

        assert_eq!(
            args.len(),
            1,
            "Unclosed quoted argument should still be captured"
        );
        assert_eq!(
            args[0], "hello world",
            "Unclosed quoted argument should contain expected content"
        );
    }

    #[test]
    fn test_escaped_quotes_in_quoted_strings_bug() {
        clear_captured_args();
        let mut shell = Shell::new();
        shell.register_command("capture", "Capture args", capture_args_handler);

        // Test escaped quotes within quoted strings
        let result = shell.input(b"capture \"hello \\\"world\\\" test\"\r");
        assert_eq!(result, ShellResult::Ok);

        let args = get_captured_args();
        assert_eq!(args.len(), 1);

        assert_eq!(
            args[0], "hello \"world\" test",
            "Escaped quotes should be preserved as literal quotes"
        );
    }

    #[test]
    fn test_escaped_backslash_in_quoted_strings_bug() {
        clear_captured_args();
        let mut shell = Shell::new();
        shell.register_command("capture", "Capture args", capture_args_handler);

        // Test escaped backslashes
        let result = shell.input(b"capture \"path\\\\to\\\\file\"\r");
        assert_eq!(result, ShellResult::Ok);

        let args = get_captured_args();
        assert_eq!(args.len(), 1);

        assert_eq!(
            args[0], "path\\to\\file",
            "Escaped backslashes should be preserved as literal backslashes"
        );
    }

    #[test]
    fn test_mixed_escaped_characters_bug() {
        clear_captured_args();
        let mut shell = Shell::new();
        shell.register_command("capture", "Capture args", capture_args_handler);

        // Test various escape sequences
        let result = shell.input(b"capture \"say \\\"hello\\\" and \\\\goodbye\\\\\"\r");
        assert_eq!(result, ShellResult::Ok);

        let args = get_captured_args();
        assert_eq!(args.len(), 1);

        assert_eq!(
            args[0], "say \"hello\" and \\goodbye\\",
            "Mixed escape sequences should be handled correctly"
        );
    }
}
