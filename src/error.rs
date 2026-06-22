use std::error::Error;
use std::fmt::{Display, Formatter, Result};
use std::io;

use ionic::ion::IonError;

#[derive(Debug)]
pub enum ImzmlError {
    Io {
        context: &'static str,
        source: io::Error,
    },
    Ion {
        context: &'static str,
        source: IonError,
    },
    UnknownDataType {
        group: String,
    },
    MissingArrayLength,
    ByteCountOverflow,
    Spectrum {
        index: u32,
        id: String,
        source: Box<ImzmlError>,
    },
    Chromatogram {
        index: u32,
        id: String,
        source: Box<ImzmlError>,
    },
}

impl ImzmlError {
    pub(crate) fn io(context: &'static str) -> impl FnOnce(io::Error) -> Self {
        move |source| Self::Io { context, source }
    }

    pub(crate) fn ion(context: &'static str) -> impl FnOnce(IonError) -> Self {
        move |source| Self::Ion { context, source }
    }
}

impl Display for ImzmlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Io { context, source } => write!(f, "{context}: {source}"),
            Self::Ion { context, source } => write!(f, "{context}: {source}"),
            Self::UnknownDataType { group } => {
                write!(f, "external array references param group '{group}' with no known data type")
            }
            Self::MissingArrayLength => write!(f, "external array has no array length"),
            Self::ByteCountOverflow => write!(f, "external array byte count overflow"),
            Self::Spectrum { index, id, source } => {
                write!(f, "cannot read spectrum index={index} id={id}: {source}")
            }
            Self::Chromatogram { index, id, source } => {
                write!(f, "cannot read chromatogram index={index} id={id}: {source}")
            }
        }
    }
}

impl Error for ImzmlError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Ion { source, .. } => Some(source),
            Self::Spectrum { source, .. } => Some(source),
            Self::Chromatogram { source, .. } => Some(source),
            Self::UnknownDataType { .. } | Self::MissingArrayLength | Self::ByteCountOverflow => {
                None
            }
        }
    }
}
