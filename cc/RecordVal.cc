// Original work Copyright 2016 Alexander Stocko <as@coder.gg>.
// Modified work Copyright 2023 Daan Vanoverloop
// See the COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#include "TableGen.h"
#include "TableGen.hpp"
#include "Types.h"

const char *tableGenRecordValGetName(TableGenRecordValRef rv_ref) {
  return unwrap(rv_ref)->getName().data();
}

TableGenRecTyKind tableGenRecordValGetType(TableGenRecordValRef rv_ref) {
  return ctablegen::tableGenFromRecType(unwrap(rv_ref)->getType());
}

TableGenTypedInitRef tableGenRecordValGetValue(TableGenRecordValRef rv_ref) {
  return wrap(dyn_cast<TypedInit>(unwrap(rv_ref)->getValue()));
}

char *tableGenRecordValGetValAsNewString(TableGenRecordValRef rv_ref) {
  return tableGenStringInitGetValueNewString(
      wrap(dyn_cast<TypedInit>(unwrap(rv_ref)->getValue())));
}

TableGenBool tableGenRecordValGetValAsBit(TableGenRecordValRef rv_ref,
                                          int8_t *bit) {
  return tableGenBitInitGetValue(
      wrap(dyn_cast<TypedInit>(unwrap(rv_ref)->getValue())), bit);
}

int8_t *tableGenRecordValGetValAsBits(TableGenRecordValRef rv_ref,
                                      size_t *len) {
  return tableGenBitsInitGetValue(
      wrap(dyn_cast<TypedInit>(unwrap(rv_ref)->getValue())), len);
}

TableGenBool tableGenRecordValGetValAsInt(TableGenRecordValRef rv_ref,
                                          int64_t *integer) {
  return tableGenIntInitGetValue(
      wrap(dyn_cast<TypedInit>(unwrap(rv_ref)->getValue())), integer);
}

TableGenRecordRef
tableGenRecordValGetValAsDefRecord(TableGenRecordValRef rv_ref) {
  return tableGenDefInitGetValue(
      wrap(dyn_cast<TypedInit>(unwrap(rv_ref)->getValue())));
}
