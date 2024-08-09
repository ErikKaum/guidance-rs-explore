// allow `\"`, `\\`, or any character which isn't a control sequence
pub static STRING_INNER: &str = r#"([^"\\\x00-\x1F\x7F-\x9F]|\\["\\])"#;
pub static STRING: &str = r#""([^"\\\x00-\x1F\x7F-\x9F]|\\["\\])*""#;

static INTEGER: &str = r#"(-)?(0|[1-9][0-9]*)"#;
static NUMBER: &str = r#"((-)?(0|[1-9][0-9]*))(\.[0-9]+)?([eE][+-][0-9]+)?"#;
static BOOLEAN: &str = r#"(true|false)"#;
static NULL: &str = r#"null"#;

pub static WHITESPACE: &str = r#"[ ]?"#;

#[derive(Debug, PartialEq)]
pub enum JsonType {
    String,
    Integer,
    Number,
    Boolean,
    Null,
}

impl JsonType {
    pub fn to_regex(&self) -> &'static str {
        match self {
            JsonType::String => STRING,
            JsonType::Integer => INTEGER,
            JsonType::Number => NUMBER,
            JsonType::Boolean => BOOLEAN,
            JsonType::Null => NULL,
        }
    }

    // fn from_str(s: &str) -> Option<JsonType> {
    //     match s {
    //         "string" => Some(JsonType::String),
    //         "integer" => Some(JsonType::Integer),
    //         "number" => Some(JsonType::Number),
    //         "boolean" => Some(JsonType::Boolean),
    //         "null" => Some(JsonType::Null),
    //         _ => None,
    //     }
    // }
}

static DATE_TIME: &str = r#""(-?(?:[1-9][0-9]*)?[0-9]{4})-(1[0-2]|0[1-9])-(3[01]|0[1-9]|[12][0-9])T(2[0-3]|[01][0-9]):([0-5][0-9]):([0-5][0-9])(\.[0-9]{3})?(Z)?""#;
static DATE: &str = r#""(?:\d{4})-(?:0[1-9]|1[0-2])-(?:0[1-9]|[1-2][0-9]|3[0-1])""#;
static TIME: &str = r#""(2[0-3]|[01][0-9]):([0-5][0-9]):([0-5][0-9])(\\.[0-9]+)?(Z)?""#;
static UUID: &str = r#""[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}""#;

#[derive(Debug, PartialEq)]
pub enum FormatType {
    DateTime,
    Date,
    Time,
    Uuid,
}

impl FormatType {
    pub fn to_regex(&self) -> &'static str {
        match self {
            FormatType::DateTime => DATE_TIME,
            FormatType::Date => DATE,
            FormatType::Time => TIME,
            FormatType::Uuid => UUID,
        }
    }

    pub fn from_str(s: &str) -> Option<FormatType> {
        match s {
            "date-time" => Some(FormatType::DateTime),
            "date" => Some(FormatType::Date),
            "time" => Some(FormatType::Time),
            "uuid" => Some(FormatType::Uuid),
            _ => None,
        }
    }
}
