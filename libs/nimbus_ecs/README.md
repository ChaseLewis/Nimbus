# Features still needed
- Async Scheduler & Runner
- Quite a few spurious allocations in some hotpaths, should consider scratchbuffers that 
  get piped down through them to reduce required allocations.
- Likely need the ability to have some form of rudimentary reflection for components
- Systems maybe should be able to hold private state?

# POTENTIAL API Issues
- World.run does a weird thing with having to take scheduler then add back to cheat the borrow checker.
- Think we are not swapping component memory very efficiently currently for non-trivial types, need to investigate
- Probably should have a batch 'spawn_with' to efficiently initialize a lot of the same type.
- Likely can improve the actual data structures a bit and make the entity storage a bit faster. More efficient arena.
- Need to see what we can do to handle dynamic types, likely need a ComponentRegistry of some form

# POTENTIAL OPTIMIZATIONS
- Should probably check if we can optimize the query planning a bit more
- We had a regression in spawn_with of ~10% after removing some allocations ... so need to check that out
