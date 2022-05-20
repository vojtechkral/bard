//! These are helpers defined on top of `quick_xml` to make XML serialization
//! of `book` AST easier.
//!
//! The `xml_write!` macro is essentially a poor man's `Derive`.
//!
//! The code here was needed as no existing XML derive crate is complete enough to cover bard AST requirements.

use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Result as XmlResult;

pub type Writer = quick_xml::Writer<File>;

pub trait XmlWrite {
    fn write(&self, writer: &mut Writer) -> XmlResult<()>;
}

impl<'a, T> XmlWrite for &'a T
where
    T: XmlWrite,
{
    fn write(&self, writer: &mut Writer) -> XmlResult<()> {
        XmlWrite::write(*self, writer)
    }
}

impl<'a> XmlWrite for &'a str {
    fn write(&self, mut writer: &mut Writer) -> XmlResult<()> {
        writer.write_text(self)
    }
}

impl XmlWrite for Box<str> {
    fn write(&self, mut writer: &mut Writer) -> XmlResult<()> {
        writer.write_text(self)
    }
}

impl<I> XmlWrite for [I]
where
    I: XmlWrite,
{
    fn write(&self, writer: &mut Writer) -> XmlResult<()> {
        for item in self.iter() {
            XmlWrite::write(item, writer)?;
        }
        Ok(())
    }
}

impl<I> XmlWrite for Box<[I]>
where
    I: XmlWrite,
{
    fn write(&self, writer: &mut Writer) -> XmlResult<()> {
        XmlWrite::write(&**self, writer)
    }
}

impl<K, V> XmlWrite for HashMap<K, V>
where
    K: AsRef<str>,
    V: XmlWrite,
{
    fn write(&self, writer: &mut Writer) -> XmlResult<()> {
        for (k, v) in self.iter() {
            writer.tag(k.as_ref()).content()?.value(v)?.finish()?;
        }
        Ok(())
    }
}

impl XmlWrite for toml::Value {
    fn write(&self, mut w: &mut Writer) -> XmlResult<()> {
        use toml::Value::*;

        match self {
            String(s) => w.write_text(s),
            Integer(i) => w.write_text(i),
            Float(f) => w.write_text(f),
            Boolean(b) => w.write_text(b),
            Datetime(dt) => w.write_text(dt),
            Array(ar) => {
                for item in ar.iter() {
                    w.tag("item").content()?.value(item)?.finish()?;
                }
                Ok(())
            }
            Table(t) => {
                for (k, v) in t.iter() {
                    w.tag(k.as_ref()).content()?.value(v)?.finish()?;
                }
                Ok(())
            }
        }
    }
}

pub struct Attr(String, String);

impl<N, V> From<(N, V)> for Attr
where
    N: ToString,
    V: ToString,
{
    fn from((name, value): (N, V)) -> Self {
        Self(name.to_string(), value.to_string())
    }
}

pub struct Field<T> {
    name: &'static str,
    value: T,
}

impl<T> Field<T> {
    pub fn new(name: &'static str, value: T) -> Self {
        Self { name, value }
    }

    pub fn unwrap(self) -> T {
        self.value
    }
}

impl<T: ToString> From<Field<T>> for Attr {
    fn from(field: Field<T>) -> Self {
        Self(field.name.to_string(), field.value.to_string())
    }
}

impl<T, I> AsRef<[I]> for Field<T>
where
    T: AsRef<[I]>,
{
    fn as_ref(&self) -> &[I] {
        self.value.as_ref()
    }
}

impl<T> AsRef<str> for Field<T>
where
    T: AsRef<str>,
{
    fn as_ref(&self) -> &str {
        self.value.as_ref()
    }
}

impl<T> XmlWrite for Field<T>
where
    T: XmlWrite,
{
    fn write(&self, writer: &mut Writer) -> XmlResult<()> {
        XmlWrite::write(&self.value, writer)
    }
}

pub struct TagBuilder<'w> {
    writer: &'w mut Writer,
    name: String,
    attrs: HashMap<String, String>,
}

impl<'w> TagBuilder<'w> {
    pub fn attr(mut self, attr: impl Into<Attr>) -> Self {
        let Attr(name, value) = attr.into();
        self.attrs.insert(name, value);
        self
    }

    pub fn attr_opt(self, name: &str, attr: &Option<impl AsRef<str>>) -> Self {
        if let Some(attr) = attr {
            self.attr((name, attr.as_ref()))
        } else {
            self
        }
    }

    pub fn content(self) -> XmlResult<ContentBuilder<'w>> {
        let name = self.name.as_bytes();
        let attrs = self
            .attrs
            .iter()
            .map(|(k, v)| Attribute::from((k.as_str(), v.as_str())));
        let elem = BytesStart::borrowed_name(name).with_attributes(attrs);
        self.writer.write_event(Event::Start(elem))?;

        Ok(ContentBuilder {
            writer: self.writer,
            parent_name: self.name,
        })
    }

    /// Creates and `<empty/>` tag.
    pub fn finish(self) -> XmlResult<()> {
        let name = self.name.as_bytes();
        let attrs = self
            .attrs
            .iter()
            .map(|(k, v)| Attribute::from((k.as_str(), v.as_str())));
        let elem = BytesStart::borrowed_name(name).with_attributes(attrs);
        self.writer.write_event(Event::Empty(elem))
    }
}

pub struct ContentBuilder<'w> {
    writer: &'w mut Writer,
    parent_name: String,
}

impl<'w> ContentBuilder<'w> {
    pub fn value(mut self, value: impl XmlWrite) -> XmlResult<Self> {
        self.writer.write_value(&value)?;
        Ok(self)
    }

    pub fn value_wrap(self, tag_name: &str, value: impl XmlWrite) -> XmlResult<Self> {
        self.writer
            .tag(tag_name)
            .content()?
            .value(value)?
            .finish()?;
        Ok(self)
    }

    pub fn field<T>(self, field: Field<T>) -> XmlResult<Self>
    where
        T: XmlWrite,
    {
        self.writer
            .tag(field.name)
            .content()?
            .value(&field.value)?
            .finish()?;
        Ok(self)
    }

    pub fn many<I, T>(self, container: T) -> XmlResult<Self>
    where
        I: XmlWrite,
        T: AsRef<[I]>,
    {
        for item in container.as_ref().iter() {
            XmlWrite::write(item, self.writer)?;
        }

        Ok(self)
    }

    pub fn many_tags<I, T>(self, tag_name: &str, container: Field<T>) -> XmlResult<Self>
    where
        I: AsRef<str>,
        T: AsRef<[I]>,
    {
        for item in container.value.as_ref().iter() {
            self.writer
                .tag(tag_name)
                .content()?
                .text(item.as_ref())?
                .finish()?;
        }

        Ok(self)
    }

    pub fn text(self, text: impl AsRef<str>) -> XmlResult<Self> {
        let text = BytesText::from_plain_str(text.as_ref());
        self.writer.write_event(Event::Text(text))?;
        Ok(self)
    }

    /// Just for convenience and visiblity.
    pub fn skip<T>(self, _: T) -> Self {
        self
    }

    pub fn finish(self) -> XmlResult<()> {
        let elem = BytesEnd::borrowed(self.parent_name.as_bytes());
        self.writer.write_event(Event::End(elem))?;

        Ok(())
    }
}

pub trait WriterExt<'w> {
    fn tag(self, name: &str) -> TagBuilder<'w>;
    fn write_value(&mut self, value: &impl XmlWrite) -> XmlResult<()>;
    fn write_text(&mut self, text: &impl Display) -> XmlResult<()>;
}

impl<'w> WriterExt<'w> for &'w mut Writer {
    fn tag(self, name: &str) -> TagBuilder<'w> {
        TagBuilder {
            writer: self,
            name: name.to_string(),
            attrs: HashMap::new(),
        }
    }

    fn write_value(&mut self, value: &impl XmlWrite) -> XmlResult<()> {
        XmlWrite::write(value, self)
    }

    fn write_text(&mut self, text: &impl Display) -> XmlResult<()> {
        let text = format!("{}", text);
        self.write_event(Event::Text(BytesText::from_plain_str(&text)))
    }
}

#[macro_export]
macro_rules! xml_write {
    (struct $ty:ident $(<$life:lifetime>)? { $($field:ident ,)+ } -> |$writer:ident| $block:block) => {
        impl $(<$life>)? XmlWrite for $ty $(<$life>)? {
            fn write(&self, $writer: &mut Writer) -> quick_xml::Result<()> {
                let $ty { $($field,)+ } = self;
                $( let $field = Field::new(stringify!($field), $field); )+
                $block.finish()
            }
        }
    };

    (enum $ty:ident |$writer:ident| { $($var:pat => $block:block ,)+ } ) => {
        impl XmlWrite for $ty {
            fn write(&self, mut $writer: &mut Writer) -> quick_xml::Result<()> {
                use $ty::*;
                match self {
                    $($var => { $block })+
                }

                Ok(())
            }
        }
    };
}
