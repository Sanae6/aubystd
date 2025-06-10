# aubrey's standard library
a standard library alternative

## todos
### allocators
- questions
  - realloc/grow/shrink?
  - should handles always be smart pointers?
- require drop impl on handles
- handling frees when coercepointee only allows one field (transparent)
  - how does alloc::boxed::Box handle storing its allocator?
    - it doesn't. it's a lang item, so it ignores the rule
    - but it doesn't support dyn dispatch on custom allocators
  - solved by `FreeVtable` unfortunately and hopefully temporarily
  - still need to figure out implementing dyn dispatch for non pointer sized types
- create and test implementations of all allocator types
  - page allocator
  - ~~c allocator~~
  - ~~arena dst allocator (allocates and frees all memory in one action each)~~
  - dst allocator to item allocator wrapper
- document all exposed functions and types
  - important
- document unsafety
