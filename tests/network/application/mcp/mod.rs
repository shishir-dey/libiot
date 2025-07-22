#[cfg(test)]
mod tests {
    use libiot::network::application::mcp::handlers::*;
    use libiot::network::application::mcp::*;
    use libiot::register_mcp_functions;

    #[test]
    fn test_mcp_message_parsing() {
        let json = r#"{"function": "gpio", "arguments": "{\"pin\": 13, \"state\": true}"}"#;
        let result: Result<(McpMessage, _), _> = serde_json_core::from_str(json);

        assert!(result.is_ok());
        let (message, _) = result.unwrap();
        assert_eq!(message.function, "gpio");
        // The JSON parser preserves the escaped quotes in the string
        assert_eq!(message.arguments, r#"{\"pin\": 13, \"state\": true}"#);
    }

    #[test]
    fn test_gpio_handler() {
        let mut handler = GpioHandler::new();

        // Test setting GPIO pin
        let result = handler.call(r#"{"pin": 13, "state": true}"#);
        assert!(result.is_ok());

        // Test reading GPIO pin
        let result = handler.call(r#"{"pin": 13}"#);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_some());
    }

    #[test]
    fn test_temperature_handler() {
        let mut handler = TemperatureSensorHandler::new();

        // Test celsius reading with empty args
        let result = handler.call("");
        assert!(result.is_ok());

        // Test celsius reading with empty JSON
        let result = handler.call("{}");
        assert!(result.is_ok());

        // Test fahrenheit reading
        let result = handler.call(r#"{"unit": "fahrenheit"}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ping_handler() {
        let mut handler = PingHandler;
        let result = handler.call("");
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.is_some());
        assert!(response.unwrap().contains("pong"));
    }

    #[test]
    fn test_function_registry() {
        let mut registry = FunctionRegistry::new();

        // Register handlers of the same type
        registry.register("ping", PingHandler).unwrap();

        // Test valid function call
        let response = registry.execute("ping", "");
        assert_eq!(response.status, ResponseStatus::Ok);
        assert!(response.result.is_some());

        // Test invalid function call
        let response = registry.execute("invalid_function", "");
        assert_eq!(response.status, ResponseStatus::NotFound);
        assert!(response.error.is_some());
    }

    #[test]
    fn test_response_serialization() {
        let response = McpResponse {
            status: ResponseStatus::Ok,
            error: None,
            result: Some(heapless::String::try_from(r#"{"message":"test"}"#).unwrap()),
        };

        let mut buf = [0u8; 256];
        let result = serde_json_core::to_slice(&response, &mut buf);
        assert!(result.is_ok());

        let serialized_len = result.unwrap();
        let json_str = core::str::from_utf8(&buf[..serialized_len]).unwrap();
        assert!(json_str.contains("\"status\":\"ok\""));
        assert!(json_str.contains("\"result\""));
    }

    #[test]
    fn test_macro_registration() {
        let mut gpio_registry = FunctionRegistry::new();

        register_mcp_functions!(
            gpio_registry,
            ("gpio1", GpioHandler::new()),
            ("gpio2", GpioHandler::new()),
        );

        // Verify both functions are registered and working
        let response1 = gpio_registry.execute("gpio1", r#"{"pin": 1}"#);
        assert_eq!(response1.status, ResponseStatus::Ok);

        let response2 = gpio_registry.execute("gpio2", r#"{"pin": 2}"#);
        assert_eq!(response2.status, ResponseStatus::Ok);
    }
}
