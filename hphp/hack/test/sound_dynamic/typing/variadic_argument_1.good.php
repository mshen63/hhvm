<?hh
// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved.

<<__SoundDynamicCallable>>
class C {
  public function foo(int ...$x) : int {
    return $x[0];
  }
}
