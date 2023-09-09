# Fiber Runtime

A Lightweighted Async Runtime for Multithreads. Like fibers/green threads.
**Mainly for studying purposes**

## Tests

`cargo run --example sleep`  Minimal example
`cargo run --example racing`  Multiple threads running light works
`cargo run --example heavy_load`  Multiple threads running CPU intensive works


## Design

MPMC channel provided by crossbeam.
Spawner(as producer) spawn tasks from futures, poll the channel dispatch tasks to each Executor(as consumer) running on each thread.  
You may wonder why there is no Signal to wait for, but it is actually an encapsulation of a real system SIG. In this implementaion, the SIG is handled by the thread::park(), called by channel receiver, more efficiently.

### Polling of the channel

Blocking the thread. It will do spin for a very short time, then (if no new task in channel) call thread::park() to hang the thread until the OS wake it up.  
For Unix/FreeBSD/Android, this process is done by futex(fast userspace mutex) primitive, which is **faster than the version implemented by conditional variables(Condvar)**.  

### TODOs

- Less Mutex locks
- Convenient macros
  - block_on!
  - join!
  - main

### Benchmarks

TODO