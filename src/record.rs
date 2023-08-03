// Original work Copyright 2016 Alexander Stocko <as@coder.gg>.
// Modified work Copyright 2023 Daan Vanoverloop
// See the COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use paste::paste;
use std::ffi::c_void;
use std::str::Utf8Error;

use crate::raw::{
    tableGenRecordGetFirstValue, tableGenRecordGetLoc, tableGenRecordGetName,
    tableGenRecordGetValue, tableGenRecordIsAnonymous, tableGenRecordIsSubclassOf,
    tableGenRecordPrint, tableGenRecordValGetLoc, tableGenRecordValGetNameInit,
    tableGenRecordValGetValue, tableGenRecordValNext, tableGenRecordValPrint, TableGenRecordRef,
    TableGenRecordValRef,
};
use crate::RecordKeeper;

use crate::error::{SourceError, SourceLoc, SourceLocation, TableGenError, WithLocation};
use crate::init::{BitInit, DagInit, ListInit, StringInit, TypedInit};
use crate::string_ref::StringRef;
use crate::util::print_callback;
use std::fmt::{self, Debug, Display, Formatter};

/// An immutable reference to a TableGen record.
///
/// This reference cannot outlive the [`RecordKeeper`] from which it is
/// borrowed.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Record<'a> {
    raw: TableGenRecordRef,
    keeper: &'a RecordKeeper<'a>,
}

impl<'a> Display for Record<'a> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        let mut data = (formatter, Ok(()));

        unsafe {
            tableGenRecordPrint(
                self.raw,
                Some(print_callback),
                &mut data as *mut _ as *mut c_void,
            );
        }

        data.1
    }
}

impl<'a> Debug for Record<'a> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        writeln!(formatter, "Record(")?;
        Display::fmt(self, formatter)?;
        write!(formatter, ")")
    }
}

macro_rules! record_value {
    ($(#[$attr:meta])* $name:ident, $type:ty) => {
        paste! {
            $(#[$attr])*
            pub fn [<$name _value>]<'n>(self, name: &'n str) -> Result<
                $type,
                SourceError<'a, TableGenError<'a>>
            > {
                self.value(name)?.try_into()
            }
        }
    };
}

impl<'a> Record<'a> {
    /// Creates a record from a raw object.
    ///
    /// # Safety
    ///
    /// The raw object must be valid.
    pub unsafe fn from_raw(keeper: &'a RecordKeeper, ptr: TableGenRecordRef) -> Record<'a> {
        Record { raw: ptr, keeper }
    }

    /// Returns the name of the record.
    ///
    /// # Errors
    ///
    /// Returns an error if the name is not a valid UTF-8 string.
    pub fn name(self) -> Result<&'a str, SourceError<'a, TableGenError<'a>>> {
        unsafe { StringRef::from_raw(tableGenRecordGetName(self.raw)) }
            .try_into()
            .map_err(|e: Utf8Error| TableGenError::from(e))
            .map_err(|e| e.with_location(self))
    }

    record_value!(
        /// Returns the boolean value of the field with the given name if this
        /// field is of type [`BitInit`](crate::init::BitInit).
        bit,
        bool
    );
    record_value!(
        /// Returns the field with the given name converted to a [`Vec<bool>`]
        /// if this field is of type [`BitsInit`](crate::init::BitsInit).
        bits,
        Vec<bool>
    );
    record_value!(
        /// Returns the integer value of the field with the given name if this
        /// field is of type [`IntInit`](crate::init::IntInit).
        int,
        i64
    );
    record_value!(
        /// Returns the field with the given name converted to a [`String`]
        /// if this field is of type [`StringInit`](crate::init::StringInit).
        ///
        /// Note that this copies the string into a new string.
        code,
        String
    );
    record_value!(
        /// Returns the field with the given name converted to a [`&str`]
        /// if this field is of type [`StringInit`](crate::init::StringInit).
        code_str,
        &'a str
    );
    record_value!(
        /// Returns the field with the given name converted to a [`String`]
        /// if this field is of type [`StringInit`](crate::init::StringInit).
        ///
        /// Note that this copies the string into a new string.
        string,
        String
    );
    record_value!(
        /// Returns the field with the given name converted to a [`&str`]
        /// if this field is of type [`StringInit`](crate::init::StringInit).
        str,
        &'a str
    );
    record_value!(
        /// Returns the field with the given name converted to a [`Record`]
        /// if this field is of type [`DefInit`](crate::init::DefInit).
        def,
        Record<'a>
    );
    record_value!(
        /// Returns the field with the given name converted to a [`ListInit`]
        /// if this field is of type [`ListInit`].
        list,
        ListInit<'a>
    );
    record_value!(
        /// Returns the field with the given name converted to a [`DagInit`]
        /// if this field is of type [`DagInit`].
        dag,
        DagInit<'a>
    );

    /// Returns a [`RecordValue`] for the field with the given name.
    pub fn value<'n>(
        self,
        name: &'n str,
    ) -> Result<RecordValue<'a>, SourceError<'a, TableGenError<'a>>> {
        let value = unsafe { tableGenRecordGetValue(self.raw, StringRef::from(name).to_raw()) };
        if !value.is_null() {
            Ok(unsafe { RecordValue::from_raw(value, self.keeper) })
        } else {
            Err(TableGenError::MissingValue(String::from(name)).with_location(self))
        }
    }

    /// Returns true if the record is anonymous.
    pub fn anonymous(self) -> bool {
        unsafe { tableGenRecordIsAnonymous(self.raw) > 0 }
    }

    /// Returns true if the record is a subclass of the class with the given
    /// name.
    pub fn subclass_of(self, class: &str) -> bool {
        unsafe { tableGenRecordIsSubclassOf(self.raw, StringRef::from(class).to_raw()) > 0 }
    }

    /// Returns an iterator over the fields of the record.
    ///
    /// The iterator yields [`RecordValue`] structs
    pub fn values(self) -> RecordValueIter<'a> {
        RecordValueIter::new(self)
    }
}

impl<'a> SourceLoc<'a> for Record<'a> {
    fn source_location(self) -> SourceLocation<'a> {
        unsafe { SourceLocation::from_raw(tableGenRecordGetLoc(self.raw), &self.keeper.parser) }
    }
}

macro_rules! try_into {
    ($type:ty) => {
        impl<'a> TryFrom<RecordValue<'a>> for $type {
            type Error = SourceError<'a, TableGenError<'a>>;

            fn try_from(record_value: RecordValue<'a>) -> Result<Self, Self::Error> {
                Self::try_from(record_value.init).map_err(|e| e.with_location(record_value))
            }
        }
    };
}

try_into!(bool);
try_into!(Vec<bool>);
try_into!(Vec<BitInit<'a>>);
try_into!(i64);
try_into!(ListInit<'a>);
try_into!(DagInit<'a>);
try_into!(Record<'a>);
try_into!(String);
try_into!(&'a str);

impl<'a> From<RecordValue<'a>> for TypedInit<'a> {
    fn from(value: RecordValue<'a>) -> Self {
        value.init
    }
}

/// Struct that represents a field of a [`Record`].
///
/// Can be converted into a Rust type using the [`TryInto`] trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordValue<'a> {
    raw: TableGenRecordValRef,
    keeper: &'a RecordKeeper<'a>,
    pub name: StringInit<'a>,
    pub init: TypedInit<'a>,
}

impl<'a> Display for RecordValue<'a> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        let mut data = (formatter, Ok(()));

        unsafe {
            tableGenRecordValPrint(
                self.raw,
                Some(print_callback),
                &mut data as *mut _ as *mut c_void,
            );
        }

        data.1
    }
}

impl<'a> RecordValue<'a> {
    /// Creates a record from a raw object.
    ///
    /// # Safety
    ///
    /// The raw object must be valid.
    pub unsafe fn from_raw(ptr: TableGenRecordValRef, keeper: &'a RecordKeeper) -> Self {
        let name = StringInit::from_raw(tableGenRecordValGetNameInit(ptr), keeper);
        let value = TypedInit::from_raw(tableGenRecordValGetValue(ptr), keeper);
        Self {
            name,
            keeper,
            init: value,
            raw: ptr,
        }
    }
}

impl<'a> SourceLoc<'a> for RecordValue<'a> {
    fn source_location(self) -> SourceLocation<'a> {
        unsafe { SourceLocation::from_raw(tableGenRecordValGetLoc(self.raw), &self.keeper.parser) }
    }
}

#[derive(Debug, Clone)]
pub struct RecordValueIter<'a> {
    record: TableGenRecordRef,
    current: TableGenRecordValRef,
    keeper: &'a RecordKeeper<'a>,
}

impl<'a> RecordValueIter<'a> {
    fn new(record: Record<'a>) -> RecordValueIter<'a> {
        unsafe {
            RecordValueIter {
                record: record.raw,
                current: tableGenRecordGetFirstValue(record.raw),
                keeper: record.keeper,
            }
        }
    }
}

impl<'a> Iterator for RecordValueIter<'a> {
    type Item = RecordValue<'a>;

    fn next(&mut self) -> Option<RecordValue<'a>> {
        let res = if self.current.is_null() {
            None
        } else {
            unsafe { Some(RecordValue::from_raw(self.current, self.keeper)) }
        };
        self.current = unsafe { tableGenRecordValNext(self.record, self.current) };
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TableGenParser;

    #[test]
    fn record() {
        let rk = TableGenParser::new()
            .add_source(
                r#"
                class A;
                class B;
                class C;

                def D1: A;
                def D2: A, B;
                def : B, C;
                "#,
            )
            .unwrap()
            .parse()
            .expect("valid tablegen");
        let d2 = rk.def("D2").expect("D2 exists");
        assert!(d2.subclass_of("A"));
        assert!(d2.subclass_of("B"));
        assert!(!d2.subclass_of("C"));
        assert!(!d2.subclass_of("D"));
        let anon = rk
            .defs()
            .map(|(_name, def)| def)
            .find(|d| d.anonymous())
            .expect("anonymous class exists");
        assert!(!anon.subclass_of("A"));
        assert!(anon.subclass_of("B"));
        assert!(anon.subclass_of("C"));
    }

    #[test]
    fn single_value() {
        let rk = TableGenParser::new()
            .add_source(
                r#"
                def A {
                    int size = 42;
                }
                "#,
            )
            .unwrap()
            .parse()
            .expect("valid tablegen");
        let a = rk.def("A").expect("def A exists");
        assert_eq!(a.name(), Ok("A"));
        assert_eq!(a.int_value("size"), Ok(42));
        assert_eq!(
            a.value("size")
                .and_then(|v| {
                    assert!(v.name.to_str() == Ok("size"));
                    v.init.as_int().map_err(|e| e.with_location(v))
                })
                .map(|i| i.into()),
            Ok(42)
        );
    }

    #[test]
    fn values() {
        let rk = TableGenParser::new()
            .add_source(
                r#"
                def A {
                    int a = 5;
                    string n = "hello";
                }
                "#,
            )
            .unwrap()
            .parse()
            .expect("valid tablegen");
        let a = rk.def("A").expect("def A exists");
        let values = a.values();
        assert_eq!(values.clone().count(), 2);
        for v in values {
            match v.init {
                TypedInit::Int(i) => {
                    assert_eq!(v.name.to_str(), Ok("a"));
                    assert_eq!(i64::from(i), 5);
                }
                TypedInit::String(i) => {
                    assert_eq!(v.name.to_str(), Ok("n"));
                    assert_eq!(i.to_str(), Ok("hello"));
                }
                _ => panic!("unexpected type"),
            }
        }
    }

    // #[test]
    // fn print_error() {
    //     let rk = TableGenParser::new()
    //         .add_source_file("test.td")
    //         // .add_source(
    //         //     r#"
    //         //     class C<int test> {
    //         //         int a = test;
    //         //     }
    //         //     def A : C<4> {
    //         //         string n = "hello";
    //         //     }
    //         //     "#,
    //         // )
    //         .unwrap()
    //         .parse()
    //         .expect("valid tablegen");
    //     let a = rk.def("A").expect("def A exists");
    //     let values = a.values();
    //     assert_eq!(values.clone().count(), 2);
    //     for v in values {
    //         // let string: String = v.try_into().unwrap();
    //         match TryInto::<String>::try_into(v) {
    //             Ok(s) => println!("{}", s),
    //             Err(e) => println!("{}", e),
    //         };
    //         match v.init {
    //             TypedInit::Int(i) => {
    //                 assert_eq!(v.name.to_str(), Ok("a"));
    //                 assert_eq!(i64::from(i), 4);
    //             }
    //             TypedInit::String(i) => {
    //                 assert_eq!(v.name.to_str(), Ok("n"));
    //                 assert_eq!(i.to_str(), Ok("hello"));
    //             }
    //             _ => panic!("unexpected type"),
    //         }
    //     }
    //     assert!(false)
    // }
}
