# Syntax

Simple variables:
```ev
const example_var: Type = const_value;
const example_var| = const_value

const test: u64 = 0;
const test| = 0;
```
Functions are constant function pointers:
```ev
const example_function: fn(): u64 = fn(): u64: 1
```
Any part of this can be implied:
```ev
const example_function| = fn()|: 1
```
Block expressions exist:
```ev
const example_var |= {
    const a |= 2;
    const b |= 3;
    2 + 3
}
```
Operations:
```ev
const a |= 2;
const b |= 3;
const c |= a + b;
const d |= a - b * c;
const e |= a / b | c & d;
const f |= { !a ^ b } << (c >> d);
const g |= true;
const h |= !g;
const i |= g & h;
...
```
Assignment supports regular assignment as well as all operations:
```ev
mut a |= 2;
a = 3;
a -= 1;
```
Structs:
```ev
TODO: but field access with @, function access with . and convert to trait object (to call trait functions) with #
TODO: trait?
TODO: structs and enums
TODO: pointers
TODO: Arrays + slices + references
```
