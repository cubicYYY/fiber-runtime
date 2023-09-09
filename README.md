# Fiber Runtime

A light-weighted async runtime for multithreads. Like fibers/green threads.  

**Mainly for learning purposes**

## Usage

See examples for more.  
You can use crate `with_locals` to do Continuation-Passing Style(CPS) programming, like what you will do in JavaScript.  

## Examples

`cargo run --example min`  Minimal example  
`cargo run --example sleep`  Async sleep example  
`cargo run --example racing`  Multiple threads running light works, found no racing of Task  
`cargo run --example heavy_load`  Multiple threads running CPU intensive works, showing little idle time & overhead for a worker thread

## Advantages / Optimizations

- No Mutex
- Light-weighted (no long spinning)
- 

## Design

Signal wakeup via MPMC channel provided by crossbeam.  
Spawner(as producer) spawn tasks from futures, poll the channel dispatch tasks to each Executor(as consumer) running on each thread.  
You may wonder why there is no Signal to wait for, but it is actually an encapsulation of a real operating system SIG. In this implementaion, the SIG is handled by the thread::park(), called by channel receiver, more efficiently.

### Polling of the channel

Blocks the thread. It will spin for a very short time (**try to avoid expensive syscall**), then (if there's no new task in channel) call thread::park() to hang the thread until the OS wake it up.  
For Unix/FreeBSD/Android, this process is done by futex(fast userspace mutex) primitive, which is **faster than the version implemented by conditional variables(Condvar)**.  
Thread-per-core is recommended, as they are equivalently accepting tasks if available.

**That is to say, CPU resource is available for others when idling.**

### TODOs

- Non-void return value
- Convenient macros
  - block_on!
  - join!
  - async main

### Benchmarks

TODO