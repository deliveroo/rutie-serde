use std::fmt;

use rutie::{self, Exception, Object};
use serde;

pub enum ErrorKind {
    Message(String),
    RutieException(rutie::AnyException),
    NotImplemented(&'static str),
}
use self::ErrorKind::*;

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Message(ref msg) => write!(f, "{}", msg),
            RutieException(ref exception) => {
                let inspect = exception.protect_send("inspect", &[]);
                let msg = match inspect {
                    Ok(inspect) => inspect
                        .try_convert_to::<rutie::RString>()
                        .map(|rstring| rstring.to_string())
                        .unwrap_or_else(|_| "unexpected inspect result".to_owned()),
                    Err(_) => "error calling inspect".to_owned(),
                };
                write!(f, "{}", msg)
            }
            NotImplemented(ref description) => write!(f, "{}", description),
        }
    }
}

impl fmt::Debug for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub struct Error {
    kind: ErrorKind,
    context: Vec<String>,
}

impl Error {
    pub fn chain_context<F, S>(mut self, func: F) -> Self
    where
        F: FnOnce() -> S,
        S: Into<String>,
    {
        self.context.push(func().into());
        self
    }

    fn describe_context(&self) -> String {
        if self.context.is_empty() {
            "".to_owned()
        } else {
            format!("\nContext from Rust:\n - {}", self.context.join("\n - "))
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match self.kind {
            Message(_) => "Generic Error",
            RutieException(_) => "Rutie Exception",
            NotImplemented(description) => description,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}\n{}", self.kind, self.describe_context())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error {
            kind,
            context: vec![],
        }
    }
}

impl<'a> From<&'a str> for Error {
    fn from(string: &'a str) -> Self {
        ErrorKind::Message(string.into()).into()
    }
}

impl From<String> for Error {
    fn from(string: String) -> Self {
        ErrorKind::Message(string).into()
    }
}
impl From<rutie::AnyException> for Error {
    fn from(exception: rutie::AnyException) -> Self {
        ErrorKind::RutieException(exception).into()
    }
}

// Many Rutie methods return `Result<AnyObject, AnyObject>` - we should try
// treat the error `AnyObject` as an `AnyException`.
impl From<rutie::AnyObject> for Error {
    fn from(obj: rutie::AnyObject) -> Self {
        obj.try_convert_to::<rutie::AnyException>()
            .map(Into::into)
            .unwrap_or_else(|_| "Error coercing AnyObject into AnyException".into())
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        format!("{}", msg).into()
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        format!("{}", msg).into()
    }
}

pub trait IntoException {
    fn into_exception(self, default_class: rutie::Class) -> rutie::AnyException;
}

impl IntoException for Error {
    fn into_exception(self, default_class: rutie::Class) -> rutie::AnyException {
        match self.kind {
            RutieException(ref exception) => {
                let msg = format!("{}{}", exception.message(), self.describe_context());
                exception.exception(Some(&msg))
            }
            _ => {
                let msg = format!("{}", self);
                let obj = default_class
                    .new_instance(&[rutie::RString::new_utf8(&msg).to_any_object()]);
                rutie::AnyException::from(obj.value())
            }
        }
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;

/// This extension trait allows callers to call `.chain_context` to add extra
/// context to errors, in the same way as error-chain's `.chain_err`. The
/// provided context will be passed to Ruby with any Exception.
pub trait ResultExt {
    fn chain_context<F, S>(self, func: F) -> Self
    where
        F: FnOnce() -> S,
        S: Into<String>;
}

impl<T> ResultExt for Result<T> {
    fn chain_context<F, S>(self, func: F) -> Self
    where
        F: FnOnce() -> S,
        S: Into<String>,
    {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(err.chain_context(func)),
        }
    }
}
