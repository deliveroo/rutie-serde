use log::debug;
use rutie::{AnyObject, Array, Boolean, Class, Fixnum, Float, NilClass, Object, RString};
use serde::de::{self, Deserialize, DeserializeSeed, MapAccess, Visitor};

use crate::{Error, ErrorKind, Result, ResultExt};

pub fn from_object<'a, T, O>(object: &O) -> Result<T>
where
    T: Deserialize<'a>,
    O: Object,
{
    let deserializer = Deserializer::new(object);
    let t = T::deserialize(deserializer)?;
    Ok(t)
}

fn object_class_name(object: &AnyObject) -> Result<String> {
    let class_name = object
        .protect_public_send("class", &[])?
        .protect_public_send("name", &[])?
        .try_convert_to::<RString>()?
        .to_string();
    Ok(class_name)
}

#[doc(hidden)]
macro_rules! try_convert_to {
    ($object:expr, $type:ty) => {{
        let object = &$object;
        $object
            .try_convert_to::<$type>()
            .map_err(Error::from)
            .chain_context(|| {
                let class_name =
                    object_class_name(object).unwrap_or_else(|_| "Unknown class".to_owned());
                format!(
                    "When deserializing '{}' as {}",
                    class_name,
                    stringify!($type)
                )
            })
    }};
}

pub struct Deserializer {
    object: AnyObject,
}

impl Deserializer {
    pub fn new<T>(object: &T) -> Self
    where
        T: Object,
    {
        Self {
            object: object.to_any_object(),
        }
    }

    fn protect_send(&self, method: &str, arguments: &[AnyObject]) -> Result<AnyObject> {
        Ok(self.object.protect_send(method, arguments)?)
    }

    fn deserialize_float(&self) -> Result<f64> {
        self.object
            .try_convert_to::<Float>()
            .map(|f| f.to_f64())
            .or_else(|_| self.deserialize_long().map(|n| n as f64))
            .map_err(Error::from)
            .chain_context(|| {
                let class_name =
                    object_class_name(&self.object).unwrap_or_else(|_| "Unknown class".to_owned());
                format!("When deserializing '{}' as Float", class_name)
            })
    }

    fn deserialize_long(&self) -> Result<i64> {
        debug!("deserialize_long");
        try_convert_to!(self.object, Fixnum).map(|fixnum| fixnum.to_i64())
    }
}

#[allow(unused_variables)]
impl<'de, 'a> de::Deserializer<'de> for Deserializer {
    type Error = Error;

    // The purpose of this method is to make a best guess of what is the type of the object and call the appropriate visitor method,
    // Usually it's not call directly, but may be called in the case of untagged enums
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_any");
        if self.object.is_nil() {
            return self.deserialize_unit(visitor);
        }
        let class_name = object_class_name(&self.object)?;
        match &*class_name {
            "Array" => self.deserialize_seq(visitor),
            "Fixnum" | "Integer" => self.deserialize_i64(visitor),
            "Float" => self.deserialize_f64(visitor),
            "Hash" => self.deserialize_map(visitor),
            "NilClass" => visitor.visit_none(),
            "String" => self.deserialize_string(visitor),
            "TrueClass" | "FalseClass" => self.deserialize_bool(visitor),
            _ => Err(format!("No rules to deserialize {}", class_name).into()),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("Deserialize bool");
        let o = try_convert_to!(self.object, Boolean)?.to_bool();
        debug!("Deserialized: {}", o);
        visitor.visit_bool(o)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("Deserialize i8");
        self.deserialize_i32(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("Deserialize i16");
        self.deserialize_i32(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("Deserialize i32");
        // let o = try_convert_to!(self.object, Fixnum)?.to_i32();
        // visitor.visit_i32(o)
        let o = try_convert_to!(self.object, Fixnum)?.to_i64();
        visitor.visit_i64(o)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_i64");
        let num = self.deserialize_long()?;
        debug!("Deserialized: {}", num);
        visitor.visit_i64(num)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("Deserialize u8");
        self.deserialize_u32(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("Deserialize u16");
        self.deserialize_u32(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("Deserialize u32");
        let o = try_convert_to!(self.object, Fixnum)?.to_i64();
        visitor.visit_u32(o as u32)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_u64");
        let num = self.deserialize_long()?;
        visitor.visit_u64(num as u64)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("Deserialize f32");
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("Deserialize f64");
        let o = self.deserialize_float()?;
        debug!("Deserialized: {}", o);
        visitor.visit_f64(o)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_char");
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_str: {:?}", self.object);
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_string: {:?}", self.object);
        let s = self
            .object
            .protect_send("to_s", &[])?
            .try_convert_to::<RString>()?
            .to_string();
        debug!("deserialize_string: {}", s);
        visitor.visit_string(s)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_bytes: {:?}", self.object);
        let s = try_convert_to!(self.object, RString)?.to_string_unchecked();
        visitor.visit_bytes(&s.into_bytes())
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_byte_buf: {:?}", self.object);
        let s = try_convert_to!(self.object, RString)?.to_string_unchecked();
        visitor.visit_byte_buf(s.into_bytes())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.object.is_nil() {
            debug!("deserialize_option: visit_none");
            visitor.visit_none()
        } else {
            debug!("deserialize_option: visit_some");
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_unit");
        if self.object.is_nil() {
            visitor.visit_unit()
        } else {
            Err("not unit".into())
        }
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_unit_struct: {}", name);
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_newtype_struct: {}", name);
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_seq");
        let s = SeqAccess::new(self.object)?;
        visitor.visit_seq(s)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_tuple");
        let s = SeqAccess::new(self.object)?;
        visitor.visit_seq(s)
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(ErrorKind::NotImplemented("Deserializer::deserialize_tuple_struct").into())
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_map");
        visitor.visit_map(HashAccess::new(&mut self)?)
    }

    fn deserialize_struct<V>(
        mut self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_struct: {}, fields: {:?}", name, fields);
        if self
            .object
            .protect_send("is_a?", &[Class::from_existing("Hash").to_any_object()])?
            .try_convert_to::<Boolean>()?
            .to_bool()
        {
            debug!("deserialize_struct: as a Hash");
            visitor.visit_map(HashAccess::new(&mut self)?)
        } else {
            debug!("deserialize_struct: as an Object");
            visitor.visit_map(ObjectAccess::new(&mut self, fields))
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!(
            "deserialize_enum name: {:?}, variants: {:?}",
            name, variants
        );
        visitor.visit_enum(EnumAccess::new(self.object))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // TODO: Verify if all identifiers are strings.
        // Currently, if deserializing hash as an object, it will query this for a field name.
        // struct MyStruct {
        //     foo: u32,
        //     bar: u32
        // }
        // if we use hash to represent this structure: "{'foo' => 123, 'bar' => 456}", then serde will call deserialize_identifier for 'foo' and 'bar'
        debug!("deserialize_identifier");
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        debug!("deserialize_ignored_any");
        visitor.visit_none()
    }
}

struct ObjectAccess<'a> {
    de: &'a mut Deserializer,
    fields: &'a [&'a str],
    pos: usize,
}

impl<'a> ObjectAccess<'a> {
    fn new(de: &'a mut Deserializer, fields: &'a [&'a str]) -> Self {
        debug!("ObjectAccess fields: {:?}", fields);
        Self { de, fields, pos: 0 }
    }
}

impl<'de, 'a> MapAccess<'de> for ObjectAccess<'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        use serde::de::IntoDeserializer;
        // Check if there are no more entries.
        if self.pos == self.fields.len() {
            return Ok(None);
        }
        debug!("next_key_seed {} pos: {}", self.fields[self.pos], self.pos);

        let field_name = self.fields[self.pos].to_string();
        seed.deserialize(field_name.into_deserializer()).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let identifier = self.fields[self.pos];
        let field_object = self
            .de
            .protect_send(identifier, &[])
            .chain_context(|| format!("While deserializing {:?}", identifier))?;
        debug!(
            "next_value_seed: field: {} ({:?})",
            identifier, field_object
        );
        self.pos += 1;
        // Deserialize a map value.
        seed.deserialize(Deserializer::new(&field_object))
            .chain_context(|| format!("While deserializing {}", identifier))
    }
}

struct SeqAccess {
    arr: AnyObject,
    pos: usize,
    len: usize,
}

impl SeqAccess {
    fn new(arr: AnyObject) -> Result<Self> {
        let len = arr
            .protect_send("length", &[])?
            .try_convert_to::<Fixnum>()?
            .to_i64() as usize;
        Ok(Self { arr, len, pos: 0 })
    }
}

impl<'de> de::SeqAccess<'de> for SeqAccess {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        debug!("SeqAccess next_element_seed");
        if self.pos == self.len {
            return Ok(None);
        }
        let element = self
            .arr
            .protect_send("[]", &[Fixnum::new(self.pos as i64).to_any_object()])?;
        self.pos += 1;
        seed.deserialize(Deserializer::new(&element)).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len - self.pos)
    }
}

struct HashAccess<'a> {
    de: &'a mut Deserializer,
    keys: Array,
    current_key: AnyObject,
    pos: usize,
    len: usize,
}

impl<'a> HashAccess<'a> {
    fn new(de: &'a mut Deserializer) -> Result<Self> {
        let keys = de
            .object
            .protect_send("keys", &[])?
            .try_convert_to::<Array>()?;
        let len = keys.length();
        Ok(Self {
            de,
            keys,
            len,
            current_key: NilClass::new().to_any_object(),
            pos: 0,
        })
    }
}

impl<'de, 'a> MapAccess<'de> for HashAccess<'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        // Check if there are no more entries.
        if self.pos == self.len {
            return Ok(None);
        }
        self.current_key = self.keys.at(self.pos as i64);
        debug!("next_key_seed {:?} pos: {}", self.current_key, self.pos);
        seed.deserialize(Deserializer::new(&self.current_key))
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let field_object = self
            .de
            .protect_send("fetch", &[self.current_key.clone()])
            .chain_context(|| format!("While deserializing {:?}", self.current_key.clone()))?;
        debug!("next_value_seed: field ({:?})", field_object);
        self.pos += 1;
        // Deserialize a map value.
        seed.deserialize(Deserializer::new(&field_object))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len - self.pos)
    }
}

#[derive(Debug)]
struct EnumAccess {
    object: AnyObject,
}

impl<'a> EnumAccess {
    fn new(object: AnyObject) -> Self {
        Self { object }
    }
}

impl<'de> de::EnumAccess<'de> for EnumAccess {
    type Error = Error;
    type Variant = VariantAccess;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        use serde::de::IntoDeserializer;
        let class_name = object_class_name(&self.object)?;
        let (variant_name, variant_content) = match &*class_name {
            // { variant_name: variant_content } newtype variant or struct variant
            "Hash" => {
                debug!("deserialize_enum: assuming externally tagged hash enum");
                let variant_name_object = self
                    .object
                    .protect_send("keys", &[])?
                    .protect_send("first", &[])?
                    .protect_send("to_s", &[])?;
                let variant_name = try_convert_to!(variant_name_object, RString)?.to_string();
                let variant_content = self
                    .object
                    .protect_send("values", &[])?
                    .protect_send("first", &[])?;
                (variant_name, variant_content)
            }
            // "variant_name" unit variant
            _ => {
                debug!("deserialize_enum: assuming string like enum");
                (
                    self.object
                        .protect_send("to_s", &[])?
                        .try_convert_to::<RString>()?
                        .to_string(),
                    self.object,
                )
            }
        };
        debug!("variant_seed: {}", variant_name);
        seed.deserialize(variant_name.into_deserializer())
            .map(|variant| (variant, VariantAccess::new(variant_content)))
    }
}

#[derive(Debug)]
struct VariantAccess {
    object: AnyObject,
}

impl VariantAccess {
    fn new(object: AnyObject) -> Self {
        Self { object }
    }
}

impl<'de> de::VariantAccess<'de> for VariantAccess {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        debug!("unit_variant");
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        debug!("newtype_variant_seed");
        seed.deserialize(Deserializer::new(&self.object))
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        debug!("tuple_variant");
        Err(ErrorKind::NotImplemented("VariantAccess::tuple_variant").into())
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        debug!("struct_variant");
        Err(ErrorKind::NotImplemented("VariantAccess::struct_variant").into())
    }
}
