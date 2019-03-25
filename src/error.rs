use std::error::Error as StdError;
use custom_error::custom_error;

pub type Result<T> = std::result::Result<T, Error>;

custom_error! {pub Error
    Io{source: std::io::Error} = "I/O error",
    NotYielded = "No item was yielded",
    XML{quick_xml: quick_xml::Error} = "XML error"
}

impl From<quick_xml::Error> for Error {
    fn from(e: quick_xml::Error) -> Error {
        Error::XML { quick_xml: e }
    }
}