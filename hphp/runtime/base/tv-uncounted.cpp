/*
   +----------------------------------------------------------------------+
   | HipHop for PHP                                                       |
   +----------------------------------------------------------------------+
   | Copyright (c) 2010-present Facebook, Inc. (http://www.facebook.com)  |
   +----------------------------------------------------------------------+
   | This source file is subject to version 3.01 of the PHP license,      |
   | that is bundled with this package in the file LICENSE, and is        |
   | available through the world-wide-web at the following url:           |
   | http://www.php.net/license/3_01.txt                                  |
   | If you did not receive a copy of the PHP license and are unable to   |
   | obtain it through the world-wide-web, please send a note to          |
   | license@php.net so we can mail you a copy immediately.               |
   +----------------------------------------------------------------------+
*/

#include "hphp/runtime/base/tv-uncounted.h"

#include "hphp/runtime/base/apc-stats.h"
#include "hphp/runtime/base/array-data.h"
#include "hphp/runtime/base/array-iterator.h"
#include "hphp/runtime/base/bespoke-array.h"
#include "hphp/runtime/base/mixed-array.h"
#include "hphp/runtime/base/packed-array.h"
#include "hphp/runtime/base/set-array.h"
#include "hphp/runtime/base/string-data.h"
#include "hphp/runtime/base/typed-value.h"
#include "hphp/runtime/vm/func.h"

namespace HPHP {

//////////////////////////////////////////////////////////////////////////////

void* AllocUncounted(size_t bytes) {
  if (APCStats::IsCreated()) {
    APCStats::getAPCStats().addAPCUncountedBlock();
  }
  return uncounted_malloc(bytes);
}

void FreeUncounted(void* ptr) {
  if (APCStats::IsCreated()) {
    APCStats::getAPCStats().removeAPCUncountedBlock();
  }
  return uncounted_free(ptr);
}

void FreeUncounted(void* ptr, size_t bytes) {
  if (APCStats::IsCreated()) {
    APCStats::getAPCStats().removeAPCUncountedBlock();
  }
  return uncounted_sized_free(ptr, bytes);
}

//////////////////////////////////////////////////////////////////////////////

void ConvertTvToUncounted(tv_lval source, DataWalker::PointerMap* seen) {
  auto& data = source.val();
  auto& type = source.type();

  switch (type) {
    case KindOfFunc:
      if (RO::EvalAPCSerializeFuncs) {
        assertx(data.pfunc->isPersistent());
        break;
      }
      invalidFuncConversion("string");

    case KindOfClass:
      if (data.pclass->isPersistent()) break;
      data.plazyclass = LazyClassData::create(data.pclass->name());
      type = KindOfLazyClass;
      break;

    case KindOfString:
      type = KindOfPersistentString;
      // Intentional fall-through.
    case KindOfPersistentString:
      data.pstr = MakeUncountedString(data.pstr, seen);
      break;

    case KindOfVec:
    case KindOfDict:
    case KindOfKeyset:
      type = dt_with_persistence(type);
      // Intentional fall-through.
    case KindOfPersistentVec:
    case KindOfPersistentDict:
    case KindOfPersistentKeyset:
      data.parr = MakeUncountedArray(data.parr, seen);
      break;

    case KindOfClsMeth: {
      if (RO::EvalAPCSerializeClsMeth) {
        assertx(use_lowptr);
        assertx(data.pclsmeth->getCls()->isPersistent());
        break;
      }
      tvCastToVecInPlace(source);
      type = KindOfPersistentVec;
      data.parr = MakeUncountedArray(data.parr, seen);
      break;
    }

    case KindOfUninit:
      type = KindOfNull;
      break;

    case KindOfLazyClass:
    case KindOfNull:
    case KindOfBoolean:
    case KindOfInt64:
    case KindOfDouble:
      break;

    case KindOfRecord:
      raise_error(Strings::RECORD_NOT_SUPPORTED);

    // DataWalker excludes these cases when it analyzes a value.
    case KindOfObject:
    case KindOfResource:
    case KindOfRFunc:
    case KindOfRClsMeth:
      always_assert(false);
  }
}

//////////////////////////////////////////////////////////////////////////////

namespace {
ArrayData* MakeUncountedArrayWithoutEscalation(
    ArrayData* in, DataWalker::PointerMap* seen, bool hasApcTv) {
  HeapObject** seenArr = nullptr;
  if (seen && in->hasMultipleRefs()) {
    seenArr = &(*seen)[in];
    if (auto const arr = static_cast<ArrayData*>(*seenArr)) {
      arr->uncountedIncRef();
      return arr;
    }
  }

  auto const result = in->makeUncounted(seen, hasApcTv);
  if (seenArr) *seenArr = result;
  return result;
}
}

ArrayData* MakeUncountedArray(
    ArrayData* in, DataWalker::PointerMap* seen, bool hasApcTv) {
  if (in->empty()) {
    auto const legacy = in->isLegacyArray();
    switch (in->toDataType()) {
      case KindOfVec: return ArrayData::CreateVec(legacy);
      case KindOfDict: return ArrayData::CreateDict(legacy);
      case KindOfKeyset: return ArrayData::CreateKeyset();
      default: always_assert(false);
    }
  }

  if (in->isVanilla()) {
    if (in->persistentIncRef()) return in;
    return MakeUncountedArrayWithoutEscalation(in, seen, hasApcTv);
  }

  auto const vad = BespokeArray::ToVanilla(in, "MakeUncountedArray");
  if (vad->persistentIncRef()) return vad;
  SCOPE_EXIT { decRefArr(vad); };
  return MakeUncountedArrayWithoutEscalation(vad, seen, hasApcTv);
}

StringData* MakeUncountedString(StringData* in, DataWalker::PointerMap* seen) {
  if (in->persistentIncRef()) return in;
  if (in->empty()) return staticEmptyString();
  if (auto const st = lookupStaticString(in)) return st;

  HeapObject** seenStr = nullptr;
  if (seen && in->hasMultipleRefs()) {
    seenStr = &(*seen)[in];
    if (auto const st = static_cast<StringData*>(*seenStr)) {
      st->uncountedIncRef();
      return st;
    }
  }

  auto const st = StringData::MakeUncounted(in->slice());
  if (seenStr) *seenStr = st;
  return st;
}

//////////////////////////////////////////////////////////////////////////////

void DecRefUncounted(TypedValue tv) {
  if (tvIsString(tv)) return DecRefUncountedString(val(tv).pstr);
  if (tvIsArrayLike(tv)) return DecRefUncountedArray(val(tv).parr);
  assertx(!isRefcountedType(type(tv)));
}

void DecRefUncountedArray(ArrayData* ad) {
  assertx(!ad->isRefCounted());
  if (ad->isUncounted() && ad->uncountedDecRef()) {
    ad->uncountedFixCountForRelease();
    ad->releaseUncounted();
  }
}

void DecRefUncountedString(StringData* sd) {
  assertx(!sd->isRefCounted());
  if (sd->isUncounted() && sd->uncountedDecRef()) {
    sd->uncountedFixCountForRelease();
    StringData::ReleaseUncounted(sd);
  }
}

//////////////////////////////////////////////////////////////////////////////

}