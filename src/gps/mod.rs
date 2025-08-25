//! GPS NMEA 0183 sentence parser
//!
//! This module provides a lightweight NMEA parser for embedded systems,
//! supporting common GPS sentence types like GPGGA, GPRMC, and GPGLL.

/// Maximum length of an NMEA sentence including \r\n
pub const NMEA_MAX_LENGTH: usize = 82;

/// NMEA sentence prefix length (e.g., "GPGGA")
pub const NMEA_PREFIX_LENGTH: usize = 5;

/// NMEA sentence ending characters
pub const NMEA_END_CHAR_1: u8 = b'\r';
/// NMEA sentence ending characters
pub const NMEA_END_CHAR_2: u8 = b'\n';

/// NMEA sentence types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NmeaType {
    /// Unknown sentence type
    Unknown,
    /// GPGGA - Global Positioning System Fix Data
    Gpgga,
    /// GPGLL - Geographic Position - Latitude/Longitude
    Gpgll,
    /// GPGSA - GPS DOP and active satellites
    Gpgsa,
    /// GPGSV - GPS Satellites in view
    Gpgsv,
    /// GPRMC - Recommended Minimum Course
    Gprmc,
    /// GPTXT - Text Transmission
    Gptxt,
    /// GPVTG - Track made good and Ground speed
    Gpvtg,
}

/// Cardinal direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardinalDirection {
    /// North
    North,
    /// East
    East,
    /// South
    South,
    /// West
    West,
    /// Unknown direction
    Unknown,
}

impl CardinalDirection {
    /// Parse cardinal direction from a character
    pub fn from_char(c: char) -> Self {
        match c {
            'N' => CardinalDirection::North,
            'E' => CardinalDirection::East,
            'S' => CardinalDirection::South,
            'W' => CardinalDirection::West,
            _ => CardinalDirection::Unknown,
        }
    }

    /// Convert to character representation
    pub fn to_char(self) -> char {
        match self {
            CardinalDirection::North => 'N',
            CardinalDirection::East => 'E',
            CardinalDirection::South => 'S',
            CardinalDirection::West => 'W',
            CardinalDirection::Unknown => '\0',
        }
    }
}

/// GPS position (latitude or longitude)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    /// Degrees component of the position
    pub degrees: i32,
    /// Minutes component of the position (decimal)
    pub minutes: f64,
    /// Cardinal direction (N/S for latitude, E/W for longitude)
    pub cardinal: CardinalDirection,
}

impl Position {
    /// Create a new position
    pub fn new(degrees: i32, minutes: f64, cardinal: CardinalDirection) -> Self {
        Self {
            degrees,
            minutes,
            cardinal,
        }
    }

    /// Convert to decimal degrees
    pub fn to_decimal_degrees(&self) -> f64 {
        let decimal = self.degrees as f64 + self.minutes / 60.0;
        match self.cardinal {
            CardinalDirection::South | CardinalDirection::West => -decimal,
            _ => decimal,
        }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self {
            degrees: 0,
            minutes: 0.0,
            cardinal: CardinalDirection::Unknown,
        }
    }
}

/// Time structure for NMEA sentences
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NmeaTime {
    /// Hour (0-23)
    pub hour: u8,
    /// Minute (0-59)
    pub minute: u8,
    /// Second (0-59)
    pub second: u8,
}

impl Default for NmeaTime {
    fn default() -> Self {
        Self {
            hour: 0,
            minute: 0,
            second: 0,
        }
    }
}

/// Date structure for NMEA sentences
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NmeaDate {
    /// Day of month (1-31)
    pub day: u8,
    /// Month (1-12)
    pub month: u8,
    /// Year (4-digit)
    pub year: u16,
}

impl Default for NmeaDate {
    fn default() -> Self {
        Self {
            day: 1,
            month: 1,
            year: 2000,
        }
    }
}

/// Base NMEA sentence structure
#[derive(Debug, Clone, PartialEq)]
pub struct NmeaBase {
    /// Type of NMEA sentence
    pub sentence_type: NmeaType,
    /// Number of parsing errors encountered
    pub errors: u32,
}

/// GPGGA sentence - Global Positioning System Fix Data
#[derive(Debug, Clone, PartialEq)]
pub struct Gpgga {
    /// Base sentence information
    pub base: NmeaBase,
    /// UTC time of position fix
    pub time: NmeaTime,
    /// Latitude position
    pub latitude: Position,
    /// Longitude position
    pub longitude: Position,
    /// Position fix indicator (0=invalid, 1=GPS fix, 2=DGPS fix)
    pub position_fix: u8,
    /// Number of satellites being tracked
    pub satellites_used: u8,
    /// Horizontal dilution of precision
    pub hdop: f32,
    /// Antenna altitude above/below mean-sea-level (geoid)
    pub altitude: f32,
    /// Units of antenna altitude (usually 'M' for meters)
    pub altitude_unit: char,
    /// Geoidal separation (difference between WGS-84 earth ellipsoid and mean-sea-level)
    pub undulation: f32,
    /// Units of geoidal separation (usually 'M' for meters)
    pub undulation_unit: char,
    /// Time in seconds since last DGPS update
    pub dgps_age: Option<f32>,
    /// DGPS station ID number
    pub dgps_station_id: Option<u16>,
}

impl Default for Gpgga {
    fn default() -> Self {
        Self {
            base: NmeaBase {
                sentence_type: NmeaType::Gpgga,
                errors: 0,
            },
            time: NmeaTime::default(),
            latitude: Position::default(),
            longitude: Position::default(),
            position_fix: 0,
            satellites_used: 0,
            hdop: 0.0,
            altitude: 0.0,
            altitude_unit: 'M',
            undulation: -9999.999,
            undulation_unit: 'M',
            dgps_age: None,
            dgps_station_id: None,
        }
    }
}

/// GPRMC sentence - Recommended Minimum Course
#[derive(Debug, Clone, PartialEq)]
pub struct Gprmc {
    /// Base sentence information
    pub base: NmeaBase,
    /// UTC time of position fix
    pub time: NmeaTime,
    /// Date of position fix
    pub date: NmeaDate,
    /// Status (true = valid, false = invalid)
    pub status: bool,
    /// Latitude position
    pub latitude: Position,
    /// Longitude position
    pub longitude: Position,
    /// Speed over ground in knots
    pub speed_knots: f32,
    /// Track angle in degrees (true north)
    pub track_degrees: f32,
    /// Magnetic variation in degrees
    pub magnetic_variation: f32,
    /// Direction of magnetic variation (E/W)
    pub magnetic_variation_direction: CardinalDirection,
}

impl Default for Gprmc {
    fn default() -> Self {
        Self {
            base: NmeaBase {
                sentence_type: NmeaType::Gprmc,
                errors: 0,
            },
            time: NmeaTime::default(),
            date: NmeaDate::default(),
            status: false,
            latitude: Position::default(),
            longitude: Position::default(),
            speed_knots: 0.0,
            track_degrees: 0.0,
            magnetic_variation: 0.0,
            magnetic_variation_direction: CardinalDirection::Unknown,
        }
    }
}

/// GPGLL sentence - Geographic Position - Latitude/Longitude
#[derive(Debug, Clone, PartialEq)]
pub struct Gpgll {
    /// Base sentence information
    pub base: NmeaBase,
    /// Latitude position
    pub latitude: Position,
    /// Longitude position
    pub longitude: Position,
    /// UTC time of position fix
    pub time: NmeaTime,
    /// Status (true = valid, false = invalid)
    pub status: bool,
}

impl Default for Gpgll {
    fn default() -> Self {
        Self {
            base: NmeaBase {
                sentence_type: NmeaType::Gpgll,
                errors: 0,
            },
            latitude: Position::default(),
            longitude: Position::default(),
            time: NmeaTime::default(),
            status: false,
        }
    }
}

/// Parsed NMEA sentence
#[derive(Debug, Clone, PartialEq)]
pub enum NmeaSentence {
    /// GPGGA sentence
    Gpgga(Gpgga),
    /// GPRMC sentence
    Gprmc(Gprmc),
    /// GPGLL sentence
    Gpgll(Gpgll),
    /// Unknown or unsupported sentence
    Unknown,
}

impl NmeaSentence {
    /// Get the sentence type
    pub fn sentence_type(&self) -> NmeaType {
        match self {
            NmeaSentence::Gpgga(_) => NmeaType::Gpgga,
            NmeaSentence::Gprmc(_) => NmeaType::Gprmc,
            NmeaSentence::Gpgll(_) => NmeaType::Gpgll,
            NmeaSentence::Unknown => NmeaType::Unknown,
        }
    }

    /// Get the number of parsing errors
    pub fn errors(&self) -> u32 {
        match self {
            NmeaSentence::Gpgga(s) => s.base.errors,
            NmeaSentence::Gprmc(s) => s.base.errors,
            NmeaSentence::Gpgll(s) => s.base.errors,
            NmeaSentence::Unknown => 0,
        }
    }
}

/// NMEA parsing errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NmeaError {
    /// Sentence length is invalid (too short or too long)
    InvalidLength,
    /// Sentence doesn't start with '$'
    InvalidStart,
    /// Sentence doesn't end with '\r\n'
    InvalidEnd,
    /// Invalid sentence prefix (not 5 uppercase letters followed by comma)
    InvalidPrefix,
    /// Checksum validation failed
    InvalidChecksum,
    /// Error parsing field data
    ParseError,
    /// Sentence type is not supported
    UnsupportedSentence,
}

/// NMEA parser utilities
#[derive(Debug)]
pub struct NmeaParser;

impl NmeaParser {
    /// Get sentence type from NMEA string
    pub fn get_sentence_type(sentence: &str) -> NmeaType {
        if sentence.len() < 6 {
            return NmeaType::Unknown;
        }

        let prefix = &sentence[1..6];
        match prefix {
            "GPGGA" | "GNGGA" => NmeaType::Gpgga,
            "GPRMC" | "GNRMC" => NmeaType::Gprmc,
            "GPGLL" | "GNGLL" => NmeaType::Gpgll,
            "GPGSA" | "GNGSA" => NmeaType::Gpgsa,
            "GPGSV" | "GNGSV" => NmeaType::Gpgsv,
            "GPTXT" | "GNTXT" => NmeaType::Gptxt,
            "GPVTG" | "GNVTG" => NmeaType::Gpvtg,
            _ => NmeaType::Unknown,
        }
    }

    /// Calculate NMEA checksum
    pub fn calculate_checksum(sentence: &str) -> u8 {
        let bytes = sentence.as_bytes();
        let mut checksum = 0u8;

        // Start after '$' and stop at '*' or end
        let start = if bytes[0] == b'$' { 1 } else { 0 };

        for &byte in &bytes[start..] {
            if byte == b'*' || byte == NMEA_END_CHAR_1 {
                break;
            }
            checksum ^= byte;
        }

        checksum
    }

    /// Check if sentence has a checksum
    pub fn has_checksum(sentence: &str) -> bool {
        sentence.len() >= 5 && sentence.chars().nth(sentence.len() - 5) == Some('*')
    }

    /// Validate NMEA sentence
    pub fn validate(sentence: &str, check_checksum: bool) -> Result<(), NmeaError> {
        let len = sentence.len();

        // Check length
        if len < 9 {
            return Err(NmeaError::InvalidLength);
        }
        if len > NMEA_MAX_LENGTH {
            return Err(NmeaError::InvalidLength);
        }

        let bytes = sentence.as_bytes();

        // Check start character
        if bytes[0] != b'$' {
            return Err(NmeaError::InvalidStart);
        }

        // Check end characters
        if len >= 2 && (bytes[len - 2] != NMEA_END_CHAR_1 || bytes[len - 1] != NMEA_END_CHAR_2) {
            return Err(NmeaError::InvalidEnd);
        }

        // Check prefix (5 uppercase letters)
        for i in 1..6 {
            if i >= len {
                return Err(NmeaError::InvalidPrefix);
            }
            let c = bytes[i];
            if !(c >= b'A' && c <= b'Z') {
                return Err(NmeaError::InvalidPrefix);
            }
        }

        // Check comma after prefix
        if len <= 6 || bytes[6] != b',' {
            return Err(NmeaError::InvalidPrefix);
        }

        // Check checksum if requested and present
        if check_checksum && Self::has_checksum(sentence) {
            let expected_checksum = Self::calculate_checksum(sentence);
            let checksum_str = &sentence[len - 4..len - 2];
            if let Ok(actual_checksum) = u8::from_str_radix(checksum_str, 16) {
                if expected_checksum != actual_checksum {
                    return Err(NmeaError::InvalidChecksum);
                }
            } else {
                return Err(NmeaError::InvalidChecksum);
            }
        }

        Ok(())
    }

    /// Parse position from NMEA format (e.g., "4916.45")
    pub fn parse_position(value: &str) -> Result<(i32, f64), NmeaError> {
        if value.is_empty() {
            return Err(NmeaError::ParseError);
        }

        // Find decimal point
        if let Some(dot_pos) = value.find('.') {
            if dot_pos < 2 {
                return Err(NmeaError::ParseError);
            }

            // Minutes start 2 digits before the decimal point
            let minutes_start = dot_pos - 2;
            let degrees_str = &value[..minutes_start];
            let minutes_str = &value[minutes_start..];

            let degrees = degrees_str
                .parse::<i32>()
                .map_err(|_| NmeaError::ParseError)?;
            let minutes = minutes_str
                .parse::<f64>()
                .map_err(|_| NmeaError::ParseError)?;

            Ok((degrees, minutes))
        } else {
            Err(NmeaError::ParseError)
        }
    }

    /// Parse time from NMEA format (e.g., "225444" or "225444.123")
    pub fn parse_time(value: &str) -> Result<NmeaTime, NmeaError> {
        if value.is_empty() {
            return Err(NmeaError::ParseError);
        }

        // Handle fractional seconds by ignoring them
        let time_str = if let Some(dot_pos) = value.find('.') {
            &value[..dot_pos]
        } else {
            value
        };

        if time_str.len() < 6 {
            return Err(NmeaError::ParseError);
        }

        let time_num = time_str.parse::<u32>().map_err(|_| NmeaError::ParseError)?;

        let hour = (time_num / 10000) as u8;
        let minute = ((time_num % 10000) / 100) as u8;
        let second = (time_num % 100) as u8;

        if hour > 23 || minute > 59 || second > 59 {
            return Err(NmeaError::ParseError);
        }

        Ok(NmeaTime {
            hour,
            minute,
            second,
        })
    }

    /// Parse date from NMEA format (e.g., "230394" for 23/03/1994)
    pub fn parse_date(value: &str) -> Result<NmeaDate, NmeaError> {
        if value.is_empty() || value.len() != 6 {
            return Err(NmeaError::ParseError);
        }

        let date_num = value.parse::<u32>().map_err(|_| NmeaError::ParseError)?;

        let day = (date_num / 10000) as u8;
        let month = ((date_num % 10000) / 100) as u8;
        let year_short = (date_num % 100) as u16;

        // Convert 2-digit year to 4-digit year (assuming 2000-2099)
        let year = if year_short >= 80 {
            1900 + year_short
        } else {
            2000 + year_short
        };

        if day == 0 || day > 31 || month == 0 || month > 12 {
            return Err(NmeaError::ParseError);
        }

        Ok(NmeaDate { day, month, year })
    }

    /// Split sentence into fields by comma
    pub fn split_fields(sentence: &str) -> Result<heapless::Vec<&str, 32>, NmeaError> {
        // Remove sentence type and checksum
        let start = sentence.find(',').ok_or(NmeaError::ParseError)? + 1;
        let end = if Self::has_checksum(sentence) {
            sentence.len() - 5 // Remove "*XX\r\n"
        } else {
            sentence.len() - 2 // Remove "\r\n"
        };

        if start >= end {
            return Ok(heapless::Vec::new());
        }

        let data_part = &sentence[start..end];
        let mut fields = heapless::Vec::new();

        for field in data_part.split(',') {
            fields.push(field).map_err(|_| NmeaError::ParseError)?;
        }

        Ok(fields)
    }

    /// Parse NMEA sentence
    pub fn parse(sentence: &str, check_checksum: bool) -> Result<NmeaSentence, NmeaError> {
        // Validate sentence
        Self::validate(sentence, check_checksum)?;

        // Get sentence type
        let sentence_type = Self::get_sentence_type(sentence);

        // Split into fields
        let fields = Self::split_fields(sentence)?;

        // Parse based on type
        match sentence_type {
            NmeaType::Gpgga => Ok(NmeaSentence::Gpgga(Self::parse_gpgga(&fields)?)),
            NmeaType::Gprmc => Ok(NmeaSentence::Gprmc(Self::parse_gprmc(&fields)?)),
            NmeaType::Gpgll => Ok(NmeaSentence::Gpgll(Self::parse_gpgll(&fields)?)),
            _ => Err(NmeaError::UnsupportedSentence),
        }
    }

    /// Parse GPGGA sentence
    fn parse_gpgga(fields: &[&str]) -> Result<Gpgga, NmeaError> {
        let mut gpgga = Gpgga::default();
        let mut errors = 0u32;

        // Parse each field
        for (i, &field) in fields.iter().enumerate() {
            if field.is_empty() {
                continue;
            }

            match i {
                0 => {
                    // Time
                    match Self::parse_time(field) {
                        Ok(time) => gpgga.time = time,
                        Err(_) => errors += 1,
                    }
                }
                1 => {
                    // Latitude
                    match Self::parse_position(field) {
                        Ok((degrees, minutes)) => {
                            gpgga.latitude.degrees = degrees;
                            gpgga.latitude.minutes = minutes;
                        }
                        Err(_) => errors += 1,
                    }
                }
                2 => {
                    // Latitude direction
                    gpgga.latitude.cardinal =
                        CardinalDirection::from_char(field.chars().next().unwrap_or('\0'));
                }
                3 => {
                    // Longitude
                    match Self::parse_position(field) {
                        Ok((degrees, minutes)) => {
                            gpgga.longitude.degrees = degrees;
                            gpgga.longitude.minutes = minutes;
                        }
                        Err(_) => errors += 1,
                    }
                }
                4 => {
                    // Longitude direction
                    gpgga.longitude.cardinal =
                        CardinalDirection::from_char(field.chars().next().unwrap_or('\0'));
                }
                5 => {
                    // Position fix indicator
                    gpgga.position_fix = field.parse().unwrap_or(0);
                }
                6 => {
                    // Number of satellites
                    gpgga.satellites_used = field.parse().unwrap_or(0);
                }
                7 => {
                    // HDOP
                    gpgga.hdop = field.parse().unwrap_or(0.0);
                }
                8 => {
                    // Altitude
                    gpgga.altitude = field.parse().unwrap_or(0.0);
                }
                9 => {
                    // Altitude unit
                    gpgga.altitude_unit = field.chars().next().unwrap_or('M');
                }
                10 => {
                    // Undulation
                    gpgga.undulation = field.parse().unwrap_or(-9999.999);
                }
                11 => {
                    // Undulation unit
                    gpgga.undulation_unit = field.chars().next().unwrap_or('M');
                }
                12 => {
                    // DGPS age
                    if let Ok(age) = field.parse::<f32>() {
                        gpgga.dgps_age = Some(age);
                    }
                }
                13 => {
                    // DGPS station ID
                    if let Ok(id) = field.parse::<u16>() {
                        gpgga.dgps_station_id = Some(id);
                    }
                }
                _ => {} // Ignore extra fields
            }
        }

        gpgga.base.errors = errors;
        Ok(gpgga)
    }

    /// Parse GPRMC sentence
    fn parse_gprmc(fields: &[&str]) -> Result<Gprmc, NmeaError> {
        let mut gprmc = Gprmc::default();
        let mut errors = 0u32;

        for (i, &field) in fields.iter().enumerate() {
            if field.is_empty() {
                continue;
            }

            match i {
                0 => {
                    // Time
                    match Self::parse_time(field) {
                        Ok(time) => gprmc.time = time,
                        Err(_) => errors += 1,
                    }
                }
                1 => {
                    // Status
                    gprmc.status = field == "A";
                }
                2 => {
                    // Latitude
                    match Self::parse_position(field) {
                        Ok((degrees, minutes)) => {
                            gprmc.latitude.degrees = degrees;
                            gprmc.latitude.minutes = minutes;
                        }
                        Err(_) => errors += 1,
                    }
                }
                3 => {
                    // Latitude direction
                    gprmc.latitude.cardinal =
                        CardinalDirection::from_char(field.chars().next().unwrap_or('\0'));
                }
                4 => {
                    // Longitude
                    match Self::parse_position(field) {
                        Ok((degrees, minutes)) => {
                            gprmc.longitude.degrees = degrees;
                            gprmc.longitude.minutes = minutes;
                        }
                        Err(_) => errors += 1,
                    }
                }
                5 => {
                    // Longitude direction
                    gprmc.longitude.cardinal =
                        CardinalDirection::from_char(field.chars().next().unwrap_or('\0'));
                }
                6 => {
                    // Speed in knots
                    gprmc.speed_knots = field.parse().unwrap_or(0.0);
                }
                7 => {
                    // Track in degrees
                    gprmc.track_degrees = field.parse().unwrap_or(0.0);
                }
                8 => {
                    // Date
                    match Self::parse_date(field) {
                        Ok(date) => gprmc.date = date,
                        Err(_) => errors += 1,
                    }
                }
                9 => {
                    // Magnetic variation
                    gprmc.magnetic_variation = field.parse().unwrap_or(0.0);
                }
                10 => {
                    // Magnetic variation direction
                    gprmc.magnetic_variation_direction =
                        CardinalDirection::from_char(field.chars().next().unwrap_or('\0'));
                }
                _ => {} // Ignore extra fields
            }
        }

        gprmc.base.errors = errors;
        Ok(gprmc)
    }

    /// Parse GPGLL sentence
    fn parse_gpgll(fields: &[&str]) -> Result<Gpgll, NmeaError> {
        let mut gpgll = Gpgll::default();
        let mut errors = 0u32;

        for (i, &field) in fields.iter().enumerate() {
            if field.is_empty() {
                continue;
            }

            match i {
                0 => {
                    // Latitude
                    match Self::parse_position(field) {
                        Ok((degrees, minutes)) => {
                            gpgll.latitude.degrees = degrees;
                            gpgll.latitude.minutes = minutes;
                        }
                        Err(_) => errors += 1,
                    }
                }
                1 => {
                    // Latitude direction
                    gpgll.latitude.cardinal =
                        CardinalDirection::from_char(field.chars().next().unwrap_or('\0'));
                }
                2 => {
                    // Longitude
                    match Self::parse_position(field) {
                        Ok((degrees, minutes)) => {
                            gpgll.longitude.degrees = degrees;
                            gpgll.longitude.minutes = minutes;
                        }
                        Err(_) => errors += 1,
                    }
                }
                3 => {
                    // Longitude direction
                    gpgll.longitude.cardinal =
                        CardinalDirection::from_char(field.chars().next().unwrap_or('\0'));
                }
                4 => {
                    // Time
                    match Self::parse_time(field) {
                        Ok(time) => gpgll.time = time,
                        Err(_) => errors += 1,
                    }
                }
                5 => {
                    // Status
                    gpgll.status = field == "A";
                }
                _ => {} // Ignore extra fields
            }
        }

        gpgll.base.errors = errors;
        Ok(gpgll)
    }
}
