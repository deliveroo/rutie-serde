// Must be defined first because of macro scoping rules.
#[macro_use]
mod macros;

mod de;
mod error;
pub mod panics;
mod ser;

pub use self::de::*;
pub use self::error::*;
pub use self::ser::*;

use rutie::{AnyObject, Object};
use serde::Deserialize;

/// A wrapper for `rutie::AnyObject` to allow it to be used in `rutie_serde` function signatures.
#[repr(C)]
pub struct RutieObject(pub AnyObject);

impl<T> From<T> for RutieObject
where
    T: Object,
{
    fn from(object: T) -> RutieObject {
        RutieObject(object.to_any_object())
    }
}

pub trait IntoAnyObject {
    fn into_any_object(self) -> Result<AnyObject>;
}

impl IntoAnyObject for RutieObject {
    fn into_any_object(self) -> Result<AnyObject> {
        Ok(self.0)
    }
}

impl<T> IntoAnyObject for T
where
    T: serde::ser::Serialize,
{
    fn into_any_object(self) -> Result<AnyObject> {
        new_ruby_object(self)
    }
}

/// Abstraction around deserialization from T: Object -> O: Deserialize
/// or from &AnyObject to RutieObject
pub trait DeserializeWrapper<T> {
    fn deserialize(data: T) -> Result<Self>
    where
        Self: Sized;
}

impl<'a> DeserializeWrapper<&'a AnyObject> for RutieObject {
    fn deserialize(data: &'a AnyObject) -> Result<RutieObject> {
        Ok(RutieObject(data.clone()))
    }
}

impl<'a, T, O> DeserializeWrapper<&'a T> for O
where
    O: Deserialize<'a>,
    T: Object,
{
    fn deserialize(data: &'a T) -> Result<O> {
        from_object(data)
    }
}

pub mod anyobject_serde {
    use rutie::{AnyObject, Class, Fixnum, Object};
    use serde::de::Error;
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<AnyObject, D::Error>
    where
        D: Deserializer<'de>,
    {
        let object_id = usize::deserialize(deserializer)?;
        let object = Class::from_existing("ObjectSpace").protect_public_send(
            "_id2ref",
            &[Fixnum::new(object_id as i64).to_any_object()],
        );
        object.map_err(|_e| D::Error::missing_field("_id2ref raised an error"))
    }
}
