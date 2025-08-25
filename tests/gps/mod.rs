//! GPS NMEA parser tests

use libiot::gps::*;

#[test]
fn test_cardinal_direction_parsing() {
    assert_eq!(CardinalDirection::from_char('N'), CardinalDirection::North);
    assert_eq!(CardinalDirection::from_char('E'), CardinalDirection::East);
    assert_eq!(CardinalDirection::from_char('S'), CardinalDirection::South);
    assert_eq!(CardinalDirection::from_char('W'), CardinalDirection::West);
    assert_eq!(
        CardinalDirection::from_char('X'),
        CardinalDirection::Unknown
    );

    assert_eq!(CardinalDirection::North.to_char(), 'N');
    assert_eq!(CardinalDirection::East.to_char(), 'E');
    assert_eq!(CardinalDirection::South.to_char(), 'S');
    assert_eq!(CardinalDirection::West.to_char(), 'W');
    assert_eq!(CardinalDirection::Unknown.to_char(), '\0');
}

#[test]
fn test_position_decimal_conversion() {
    let pos_north = Position::new(48, 7.038, CardinalDirection::North);
    assert!((pos_north.to_decimal_degrees() - 48.11730).abs() < 0.0001);

    let pos_south = Position::new(48, 7.038, CardinalDirection::South);
    assert!((pos_south.to_decimal_degrees() + 48.11730).abs() < 0.0001);

    let pos_east = Position::new(11, 31.000, CardinalDirection::East);
    assert!((pos_east.to_decimal_degrees() - 11.51667).abs() < 0.0001);

    let pos_west = Position::new(11, 31.000, CardinalDirection::West);
    assert!((pos_west.to_decimal_degrees() + 11.51667).abs() < 0.0001);
}

#[test]
fn test_sentence_type_detection() {
    assert_eq!(
        NmeaParser::get_sentence_type("$GPGGA,test"),
        NmeaType::Gpgga
    );
    assert_eq!(
        NmeaParser::get_sentence_type("$GNGGA,test"),
        NmeaType::Gpgga
    );
    assert_eq!(
        NmeaParser::get_sentence_type("$GPRMC,test"),
        NmeaType::Gprmc
    );
    assert_eq!(
        NmeaParser::get_sentence_type("$GNRMC,test"),
        NmeaType::Gprmc
    );
    assert_eq!(
        NmeaParser::get_sentence_type("$GPGLL,test"),
        NmeaType::Gpgll
    );
    assert_eq!(
        NmeaParser::get_sentence_type("$GNGLL,test"),
        NmeaType::Gpgll
    );
    assert_eq!(
        NmeaParser::get_sentence_type("$GPXXX,test"),
        NmeaType::Unknown
    );
    assert_eq!(NmeaParser::get_sentence_type("$GP"), NmeaType::Unknown);
}

#[test]
fn test_checksum_calculation() {
    // Test case from libnmea C library
    let sentence1 = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47";
    assert_eq!(NmeaParser::calculate_checksum(sentence1), 0x47);

    // Test case from NMEA standard
    let sentence2 = "$GPRMC,225446,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E*68";
    assert_eq!(NmeaParser::calculate_checksum(sentence2), 0x68);

    // Test without checksum
    let sentence3 = "$GPGLL,4916.45,N,12311.12,W,225444,A";
    assert_eq!(NmeaParser::calculate_checksum(sentence3), 0x1D);
}

#[test]
fn test_checksum_detection() {
    assert!(NmeaParser::has_checksum("$GPGGA,test*47\r\n"));
    assert!(NmeaParser::has_checksum("$GPRMC,test*68\r\n"));
    assert!(!NmeaParser::has_checksum("$GPGLL,test\r\n"));
    assert!(!NmeaParser::has_checksum("$GP*\r\n")); // Too short
}

#[test]
fn test_sentence_validation() {
    // Valid sentences
    let valid1 = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n";
    assert!(NmeaParser::validate(valid1, true).is_ok());

    let valid2 = "$GPRMC,225446,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E\r\n";
    assert!(NmeaParser::validate(valid2, false).is_ok());

    // Invalid sentences
    assert_eq!(
        NmeaParser::validate("GPGGA,test\r\n", false),
        Err(NmeaError::InvalidStart)
    );
    assert_eq!(
        NmeaParser::validate("$GPGGA,test", false),
        Err(NmeaError::InvalidEnd)
    );
    assert_eq!(
        NmeaParser::validate("$gpgga,test\r\n", false),
        Err(NmeaError::InvalidPrefix)
    );
    assert_eq!(
        NmeaParser::validate("$GPGGA test\r\n", false),
        Err(NmeaError::InvalidPrefix)
    );
    assert_eq!(
        NmeaParser::validate("$GP\r\n", false),
        Err(NmeaError::InvalidLength)
    );

    // Invalid checksum
    let invalid_checksum = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*46\r\n";
    assert_eq!(
        NmeaParser::validate(invalid_checksum, true),
        Err(NmeaError::InvalidChecksum)
    );
}

#[test]
fn test_position_parsing() {
    // Test valid positions
    let (degrees, minutes) = NmeaParser::parse_position("4807.038").unwrap();
    assert_eq!(degrees, 48);
    assert!((minutes - 7.038).abs() < 0.001);

    let (degrees, minutes) = NmeaParser::parse_position("12311.12").unwrap();
    assert_eq!(degrees, 123);
    assert!((minutes - 11.12).abs() < 0.001);

    let (degrees, minutes) = NmeaParser::parse_position("0000.000").unwrap();
    assert_eq!(degrees, 0);
    assert!((minutes - 0.0).abs() < 0.001);

    // Test invalid positions
    assert!(NmeaParser::parse_position("").is_err());
    assert!(NmeaParser::parse_position("48").is_err());
    assert!(NmeaParser::parse_position("4807").is_err());
    assert!(NmeaParser::parse_position("abc.def").is_err());
}

#[test]
fn test_time_parsing() {
    // Test valid times
    let time = NmeaParser::parse_time("123519").unwrap();
    assert_eq!(time.hour, 12);
    assert_eq!(time.minute, 35);
    assert_eq!(time.second, 19);

    let time = NmeaParser::parse_time("000000").unwrap();
    assert_eq!(time.hour, 0);
    assert_eq!(time.minute, 0);
    assert_eq!(time.second, 0);

    let time = NmeaParser::parse_time("235959").unwrap();
    assert_eq!(time.hour, 23);
    assert_eq!(time.minute, 59);
    assert_eq!(time.second, 59);

    // Test time with fractional seconds (should be ignored)
    let time = NmeaParser::parse_time("123519.123").unwrap();
    assert_eq!(time.hour, 12);
    assert_eq!(time.minute, 35);
    assert_eq!(time.second, 19);

    // Test invalid times
    assert!(NmeaParser::parse_time("").is_err());
    assert!(NmeaParser::parse_time("12345").is_err());
    assert!(NmeaParser::parse_time("246000").is_err()); // Invalid hour
    assert!(NmeaParser::parse_time("126000").is_err()); // Invalid minute
    assert!(NmeaParser::parse_time("123560").is_err()); // Invalid second
    assert!(NmeaParser::parse_time("abcdef").is_err());
}

#[test]
fn test_date_parsing() {
    // Test valid dates
    let date = NmeaParser::parse_date("230394").unwrap();
    assert_eq!(date.day, 23);
    assert_eq!(date.month, 3);
    assert_eq!(date.year, 1994);

    let date = NmeaParser::parse_date("010100").unwrap();
    assert_eq!(date.day, 1);
    assert_eq!(date.month, 1);
    assert_eq!(date.year, 2000);

    let date = NmeaParser::parse_date("311299").unwrap();
    assert_eq!(date.day, 31);
    assert_eq!(date.month, 12);
    assert_eq!(date.year, 1999);

    // Test invalid dates
    assert!(NmeaParser::parse_date("").is_err());
    assert!(NmeaParser::parse_date("23039").is_err()); // Too short
    assert!(NmeaParser::parse_date("2303944").is_err()); // Too long
    assert!(NmeaParser::parse_date("000394").is_err()); // Invalid day
    assert!(NmeaParser::parse_date("230094").is_err()); // Invalid month
    assert!(NmeaParser::parse_date("320394").is_err()); // Invalid day
    assert!(NmeaParser::parse_date("231394").is_err()); // Invalid month
    assert!(NmeaParser::parse_date("abcdef").is_err());
}

#[test]
fn test_field_splitting() {
    let sentence = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n";
    let fields = NmeaParser::split_fields(sentence).unwrap();

    assert_eq!(fields.len(), 14);
    assert_eq!(fields[0], "123519");
    assert_eq!(fields[1], "4807.038");
    assert_eq!(fields[2], "N");
    assert_eq!(fields[3], "01131.000");
    assert_eq!(fields[4], "E");
    assert_eq!(fields[5], "1");
    assert_eq!(fields[6], "08");
    assert_eq!(fields[7], "0.9");
    assert_eq!(fields[8], "545.4");
    assert_eq!(fields[9], "M");
    assert_eq!(fields[10], "46.9");
    assert_eq!(fields[11], "M");
    assert_eq!(fields[12], "");
    assert_eq!(fields[13], "");
}

#[test]
fn test_gpgga_parsing() {
    let sentence = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n";
    let parsed = NmeaParser::parse(sentence, true).unwrap();

    if let NmeaSentence::Gpgga(gpgga) = parsed {
        assert_eq!(gpgga.base.sentence_type, NmeaType::Gpgga);
        assert_eq!(gpgga.base.errors, 0);

        assert_eq!(gpgga.time.hour, 12);
        assert_eq!(gpgga.time.minute, 35);
        assert_eq!(gpgga.time.second, 19);

        assert_eq!(gpgga.latitude.degrees, 48);
        assert!((gpgga.latitude.minutes - 7.038).abs() < 0.001);
        assert_eq!(gpgga.latitude.cardinal, CardinalDirection::North);

        assert_eq!(gpgga.longitude.degrees, 11);
        assert!((gpgga.longitude.minutes - 31.000).abs() < 0.001);
        assert_eq!(gpgga.longitude.cardinal, CardinalDirection::East);

        assert_eq!(gpgga.position_fix, 1);
        assert_eq!(gpgga.satellites_used, 8);
        assert!((gpgga.hdop - 0.9).abs() < 0.001);
        assert!((gpgga.altitude - 545.4).abs() < 0.001);
        assert_eq!(gpgga.altitude_unit, 'M');
        assert!((gpgga.undulation - 46.9).abs() < 0.001);
        assert_eq!(gpgga.undulation_unit, 'M');
    } else {
        panic!("Expected GPGGA sentence");
    }
}

#[test]
fn test_gprmc_parsing() {
    let sentence = "$GPRMC,225446,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E*68\r\n";
    let parsed = NmeaParser::parse(sentence, true).unwrap();

    if let NmeaSentence::Gprmc(gprmc) = parsed {
        assert_eq!(gprmc.base.sentence_type, NmeaType::Gprmc);
        assert_eq!(gprmc.base.errors, 0);

        assert_eq!(gprmc.time.hour, 22);
        assert_eq!(gprmc.time.minute, 54);
        assert_eq!(gprmc.time.second, 46);

        assert_eq!(gprmc.status, true);

        assert_eq!(gprmc.latitude.degrees, 49);
        assert!((gprmc.latitude.minutes - 16.45).abs() < 0.001);
        assert_eq!(gprmc.latitude.cardinal, CardinalDirection::North);

        assert_eq!(gprmc.longitude.degrees, 123);
        assert!((gprmc.longitude.minutes - 11.12).abs() < 0.001);
        assert_eq!(gprmc.longitude.cardinal, CardinalDirection::West);

        assert!((gprmc.speed_knots - 0.5).abs() < 0.001);
        assert!((gprmc.track_degrees - 54.7).abs() < 0.001);

        assert_eq!(gprmc.date.day, 19);
        assert_eq!(gprmc.date.month, 11);
        assert_eq!(gprmc.date.year, 1994);

        assert!((gprmc.magnetic_variation - 20.3).abs() < 0.001);
        assert_eq!(gprmc.magnetic_variation_direction, CardinalDirection::East);
    } else {
        panic!("Expected GPRMC sentence");
    }
}

#[test]
fn test_gpgll_parsing() {
    let sentence = "$GPGLL,4916.45,N,12311.12,W,225444,A*1D\r\n";
    let parsed = NmeaParser::parse(sentence, true).unwrap();

    if let NmeaSentence::Gpgll(gpgll) = parsed {
        assert_eq!(gpgll.base.sentence_type, NmeaType::Gpgll);
        assert_eq!(gpgll.base.errors, 0);

        assert_eq!(gpgll.latitude.degrees, 49);
        assert!((gpgll.latitude.minutes - 16.45).abs() < 0.001);
        assert_eq!(gpgll.latitude.cardinal, CardinalDirection::North);

        assert_eq!(gpgll.longitude.degrees, 123);
        assert!((gpgll.longitude.minutes - 11.12).abs() < 0.001);
        assert_eq!(gpgll.longitude.cardinal, CardinalDirection::West);

        assert_eq!(gpgll.time.hour, 22);
        assert_eq!(gpgll.time.minute, 54);
        assert_eq!(gpgll.time.second, 44);

        assert_eq!(gpgll.status, true);
    } else {
        panic!("Expected GPGLL sentence");
    }
}

#[test]
fn test_invalid_sentence_parsing() {
    // Test unsupported sentence type
    let sentence = "$GPXXX,test,data\r\n";
    assert_eq!(
        NmeaParser::parse(sentence, false),
        Err(NmeaError::UnsupportedSentence)
    );

    // Test invalid sentence format
    let sentence = "GPGGA,test,data\r\n";
    assert_eq!(
        NmeaParser::parse(sentence, false),
        Err(NmeaError::InvalidStart)
    );
}

#[test]
fn test_sentence_with_empty_fields() {
    // GPGGA with some empty fields
    let sentence = "$GPGGA,123519,,N,,E,1,08,0.9,545.4,M,46.9,M,,*2C\r\n";
    let parsed = NmeaParser::parse(sentence, true).unwrap();

    if let NmeaSentence::Gpgga(gpgga) = parsed {
        assert_eq!(gpgga.base.sentence_type, NmeaType::Gpgga);
        // Should have some errors due to empty position fields
        assert!(gpgga.base.errors > 0);

        assert_eq!(gpgga.time.hour, 12);
        assert_eq!(gpgga.time.minute, 35);
        assert_eq!(gpgga.time.second, 19);

        // Position fields should be default due to parsing errors
        assert_eq!(gpgga.latitude.degrees, 0);
        assert_eq!(gpgga.longitude.degrees, 0);

        assert_eq!(gpgga.position_fix, 1);
        assert_eq!(gpgga.satellites_used, 8);
    } else {
        panic!("Expected GPGGA sentence");
    }
}

#[test]
fn test_sentence_methods() {
    let sentence = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n";
    let parsed = NmeaParser::parse(sentence, true).unwrap();

    assert_eq!(parsed.sentence_type(), NmeaType::Gpgga);
    assert_eq!(parsed.errors(), 0);

    let unknown = NmeaSentence::Unknown;
    assert_eq!(unknown.sentence_type(), NmeaType::Unknown);
    assert_eq!(unknown.errors(), 0);
}

#[test]
fn test_real_nmea_sentences() {
    // Real NMEA sentences from GPS devices
    let sentences = [
        "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76\r\n",
        "$GPRMC,092750.000,A,5321.6802,N,00630.3372,W,0.02,31.66,280511,,,A*43\r\n",
        "$GPGLL,5321.6802,N,00630.3372,W,092750.000,A,A*7C\r\n",
        "$GNGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*5E\r\n",
        "$GNRMC,225446,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E*50\r\n",
    ];

    for sentence in &sentences {
        let result = NmeaParser::parse(sentence, true);
        assert!(result.is_ok(), "Failed to parse: {}", sentence);

        let parsed = result.unwrap();
        assert_eq!(parsed.errors(), 0, "Parse errors in: {}", sentence);
    }
}

#[test]
fn test_gnss_sentences() {
    // Test GNSS (multi-constellation) sentences
    let gngga = "$GNGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*5E\r\n";
    let parsed = NmeaParser::parse(gngga, true).unwrap();
    assert_eq!(parsed.sentence_type(), NmeaType::Gpgga);

    let gnrmc = "$GNRMC,225446,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E*50\r\n";
    let parsed = NmeaParser::parse(gnrmc, true).unwrap();
    assert_eq!(parsed.sentence_type(), NmeaType::Gprmc);

    let gngll = "$GNGLL,4916.45,N,12311.12,W,225444,A*05\r\n";
    let parsed = NmeaParser::parse(gngll, true).unwrap();
    assert_eq!(parsed.sentence_type(), NmeaType::Gpgll);
}

#[test]
fn test_edge_cases() {
    // Test minimum valid sentence
    let min_sentence = "$GPGGA,,,,,,,,,,,,,*56\r\n";
    let result = NmeaParser::parse(min_sentence, true);
    assert!(result.is_ok());

    // Test sentence at maximum length
    let long_sentence = format!("$GPGGA,{},*00\r\n", "1,".repeat(35));
    assert!(long_sentence.len() <= NMEA_MAX_LENGTH);

    // Test position at equator and prime meridian
    let equator_sentence = "$GPGGA,123519,0000.000,N,00000.000,E,1,08,0.9,0.0,M,0.0,M,,*6B\r\n";
    let parsed = NmeaParser::parse(equator_sentence, true).unwrap();
    if let NmeaSentence::Gpgga(gpgga) = parsed {
        assert_eq!(gpgga.latitude.degrees, 0);
        assert_eq!(gpgga.longitude.degrees, 0);
        assert!((gpgga.latitude.minutes - 0.0).abs() < 0.001);
        assert!((gpgga.longitude.minutes - 0.0).abs() < 0.001);
    }
}
