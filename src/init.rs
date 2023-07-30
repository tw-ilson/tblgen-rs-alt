// Original work Copyright 2016 Alexander Stocko <as@coder.gg>.
// Modified work Copyright 2023 Daan Vanoverloop
// See the COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::raw::{
    tableGenBitArrayFree, tableGenBitInitGetValue, tableGenBitsInitGetValue,
    tableGenDagRecordArgName, tableGenDagRecordGet, tableGenDagRecordNumArgs,
    tableGenDefInitGetValue, tableGenInitRecType, tableGenIntInitGetValue, tableGenListRecordGet,
    tableGenListRecordNumElements, tableGenStringInitGetValueNewString, TableGenRecTyKind,
    TableGenTypedInitRef,
};
use paste::paste;

use crate::{
    error::{self, TableGenError},
    record::Record,
};
use std::ffi::CStr;

#[derive(Debug)]
pub enum TypedInit {
    Bit(i8),
    Bits(Vec<i8>),
    Code(String),
    Int(i64),
    String(String),
    List(ListInit),
    Dag(DagInit),
    Record(Record),
    Invalid,
}

macro_rules! as_inner {
    ($name:ident, $variant:ident, $type:ty) => {
        paste! {
            pub fn [<as_ $name>](&self) -> Option<&$type> {
                match &self {
                    Self::$variant(v) => Some(v),
                    _ => None
                }
            }
        }
    };
}

macro_rules! try_into {
    ($variant:ident, $type:ty) => {
        impl TryFrom<TypedInit> for $type {
            type Error = TableGenError;

            fn try_from(value: TypedInit) -> Result<Self, Self::Error> {
                match value {
                    TypedInit::$variant(v) => Ok(v),
                    _ => Err(Self::Error::IncorrectInitType(value)),
                }
            }
        }
    };
}

try_into!(Bit, i8);
try_into!(Bits, Vec<i8>);
try_into!(Int, i64);
try_into!(List, ListInit);
try_into!(Dag, DagInit);
try_into!(Record, Record);

impl TryFrom<TypedInit> for String {
    type Error = TableGenError;

    fn try_from(value: TypedInit) -> Result<Self, Self::Error> {
        match value {
            TypedInit::String(v) | TypedInit::Code(v) => Ok(v),
            _ => Err(Self::Error::IncorrectInitType(value)),
        }
    }
}

impl TypedInit {
    as_inner!(bit, Bit, i8);
    as_inner!(bits, Bits, Vec<i8>);
    as_inner!(code, Code, String);
    as_inner!(int, Int, i64);
    as_inner!(string, String, String);
    as_inner!(list, List, ListInit);
    as_inner!(dag, Dag, DagInit);
    as_inner!(def, Record, Record);

    #[allow(non_upper_case_globals)]
    pub unsafe fn from_raw(init: TableGenTypedInitRef) -> error::Result<Self> {
        let t = tableGenInitRecType(init);

        use TableGenRecTyKind::*;
        match t {
            TableGenBitRecTyKind => {
                let mut bit = -1;
                tableGenBitInitGetValue(init, &mut bit);

                if bit == 0 || bit == 1 {
                    Ok(TypedInit::Bit(bit))
                } else {
                    Err(TableGenError::InvalidBitRange)
                }
            }
            TableGenBitsRecTyKind => {
                let mut bits: Vec<_> = Vec::new();
                let mut len: usize = 0;
                let cbits = tableGenBitsInitGetValue(init, &mut len);
                let mut bits_ptr = cbits;
                for _ in 0..len {
                    bits.push(*bits_ptr);
                    bits_ptr = bits_ptr.offset(1);
                }
                tableGenBitArrayFree(cbits);
                if bits.is_empty() {
                    Err(TableGenError::NullPointer.into())
                } else {
                    Ok(TypedInit::Bits(bits))
                }
            }
            TableGenDagRecTyKind => Ok(TypedInit::Dag(DagInit::from_raw(init))),
            TableGenIntRecTyKind => {
                let mut int: i64 = 0;
                tableGenIntInitGetValue(init, &mut int);
                Ok(TypedInit::Int(int))
            }
            TableGenListRecTyKind => Ok(TypedInit::List(ListInit::from_raw(init))),
            TableGenRecordRecTyKind => Ok(TypedInit::Record(Record::from_raw(
                tableGenDefInitGetValue(init),
            ))),
            TableGenStringRecTyKind => {
                let cstr = tableGenStringInitGetValueNewString(init);
                Ok(TypedInit::String(
                    CStr::from_ptr(cstr).to_string_lossy().into_owned(),
                ))
            }
            _ => Ok(Self::Invalid),
        }
    }
}

#[derive(Debug)]
pub struct DagInit {
    raw: TableGenTypedInitRef,
}

impl DagInit {
    pub fn from_raw(val: TableGenTypedInitRef) -> DagInit {
        DagInit { raw: val }
    }

    pub fn args(&self) -> DagIter {
        DagIter {
            dag: self,
            index: 0,
        }
    }

    pub fn num_args(&self) -> usize {
        unsafe { tableGenDagRecordNumArgs(self.raw) }
    }

    pub fn name(&self, index: usize) -> Option<String> {
        let value = unsafe { tableGenDagRecordArgName(self.raw, index) };
        if !value.is_null() {
            Some(unsafe { CStr::from_ptr(value).to_string_lossy().into_owned() })
        } else {
            None
        }
    }

    pub fn get(&self, index: usize) -> Option<TypedInit> {
        let value = unsafe { tableGenDagRecordGet(self.raw, index) };
        if !value.is_null() {
            unsafe { TypedInit::from_raw(value).ok() }
        } else {
            None
        }
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> Option<TypedInit> {
        TypedInit::from_raw(tableGenDagRecordGet(self.raw, index)).ok()
    }
}

#[derive(Debug, Clone)]
pub struct DagIter<'a> {
    dag: &'a DagInit,
    index: usize,
}

impl<'a> Iterator for DagIter<'a> {
    type Item = (String, TypedInit);

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.dag.get(self.index);
        let name = self.dag.name(self.index);
        self.index += 1;
        if next.is_some() && name.is_some() {
            Some((name.unwrap(), next.unwrap()))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct ListInit {
    raw: TableGenTypedInitRef,
}

impl ListInit {
    pub fn from_raw(val: TableGenTypedInitRef) -> ListInit {
        ListInit { raw: val }
    }

    pub fn iter(&self) -> ListIter {
        ListIter {
            list: self,
            index: 0,
        }
    }

    pub fn len(&self) -> usize {
        unsafe { tableGenListRecordNumElements(self.raw) }
    }

    pub fn get(&self, index: usize) -> Option<TypedInit> {
        let value = unsafe { tableGenListRecordGet(self.raw, index) };
        if !value.is_null() {
            unsafe { TypedInit::from_raw(value).ok() }
        } else {
            None
        }
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> Option<TypedInit> {
        TypedInit::from_raw(tableGenListRecordGet(self.raw, index)).ok()
    }
}

#[derive(Clone)]
pub struct ListIter<'a> {
    list: &'a ListInit,
    index: usize,
}

impl<'a> Iterator for ListIter<'a> {
    type Item = TypedInit;

    fn next(&mut self) -> Option<TypedInit> {
        let next = unsafe { tableGenListRecordGet(self.list.raw, self.index) };
        self.index += 1;
        if !next.is_null() {
            unsafe { TypedInit::from_raw(next).ok() }
        } else {
            None
        }
    }
}