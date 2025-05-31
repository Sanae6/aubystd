# aubrey's standard library
a standard library alternative

## todos
### allocators
- questions
  - realloc?
  - should handles always be smart pointers?
- handle reserving slice dst structs in the dst allocator
  - zerocopy is quite useful for this
- require drop impl on handles
- handling frees when coercepointee only allows one field (transparent)
  - how does alloc::boxed::Box handle storing its allocator?
- create and test implementations of all allocator types
  - page allocator
  - c allocator
  - arena dst allocator (allocates and frees all memory in one action each)
  - dst allocator to item allocator wrapper
- document all exposed functions and types
- document unsafety
