<?hh

enum class E : int { int A = 42;
 }

function f(HH\EnumClass\Label<E, int> $_): void {}
function g(HH\EnumClass\Label<E, int> $_, int $_): void {}

class Chains {
  public function f(HH\EnumClass\Label<E, int> $_) : this { return $this; }
  public function g(HH\EnumClass\Label<E, int> $_, int $_) : this { return $this; }
}

function main(): void {
  f(#A);
  g(#A, 42);

  $c = new Chains();
  $c->f(#A);
  $c->f(#A)->g(#A, 42);
}
