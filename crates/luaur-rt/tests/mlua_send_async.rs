// Adapted from mlua (https://github.com/mlua-rs/mlua), MIT License,
// © 2019 Aleksandr Orlenko / mlua authors. See tests/ATTRIBUTION.md.
//
// `send` + `async` together. mlua supports both features at once; luaur-rt now
// does too. The two ingredients that used to force them apart:
//
//   1. MaybeSend — the type-erased async callback and the boxed `Future` it
//      returns carry a `+ Send` bound under `send` (exactly like the sync
//      `BoxedCallback`), so a future stashed inside the VM keeps the VM `Send`.
//   2. The "global-sentinel" store — the async bridge's per-VM waker and
//      implicit-thread ownership map live in a process-wide table keyed by the
//      VM's global-state pointer. Under `send` that table is a real `Mutex`
//      (not a thread-local), so the state travels with the VM when it is moved
//      to another thread and is found again by the executor that drives it
//      there.
//
// Gated on BOTH features; only built/run with `--features send,async`.

#![cfg(all(feature = "send", feature = "async"))]

use std::time::Duration;

use luaur_rt::{AsyncThread, Lua, Result};

fn assert_send<T: Send>() {}

// ---------------------------------------------------------------------------
// Compile-time: under `send` the VM, its handles, AND the async driver future
// are `Send`, so async work can be awaited on any thread / a multi-threaded
// executor.
// ---------------------------------------------------------------------------
#[test]
fn test_async_driver_is_send() {
    assert_send::<Lua>();
    assert_send::<AsyncThread<i64>>();
    assert_send::<AsyncThread<()>>();
}

// ---------------------------------------------------------------------------
// The whole reason this combination must be sound: construct the VM and
// register an async function on one thread, *move* the VM to a fresh OS thread,
// and drive a genuinely-pending async there on a runtime created on that
// thread. The `tokio::time::sleep` future is `Send` (exercising MaybeSend) and
// really parks (exercising the per-VM waker store under `send`: the executor's
// waker, stored in the global table, must fire the re-poll after the move).
// ---------------------------------------------------------------------------
#[test]
fn test_move_vm_then_drive_pending_async_on_worker_thread() -> Result<()> {
    let lua = Lua::new();

    let f = lua.create_async_function(|_lua, (a, b): (i64, i64)| async move {
        // A real await point that returns `Pending` and is woken by the timer.
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(a + b)
    })?;
    lua.globals().set("add", f)?;

    // Move the entire VM (and the registered async fn) to another thread, build
    // a fresh current-thread tokio runtime *there*, and await the eval on it.
    let handle = std::thread::spawn(move || -> Result<i64> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("failed to build tokio runtime on worker thread");
        rt.block_on(async move { lua.load("return add(40, 2)").eval_async().await })
    });

    let result = handle.join().expect("worker thread panicked")?;
    assert_eq!(result, 42);
    Ok(())
}

// ---------------------------------------------------------------------------
// An async closure may capture `Send` data moved across a thread boundary into
// its environment (the `MaybeSend` bound makes the stored async box `Send`).
// Mirrors `mlua_send.rs::test_callback_captures_send_data`, async edition.
// ---------------------------------------------------------------------------
#[test]
fn test_async_callback_captures_send_data() -> Result<()> {
    // Produce the payload on a worker thread, then move it into the async
    // closure registered on the main-thread VM.
    let payload: Vec<i64> = std::thread::spawn(|| vec![10, 20, 30])
        .join()
        .expect("worker panicked");

    let lua = Lua::new();
    let sum = lua.create_async_function(move |_lua, ()| {
        let payload = payload.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(1)).await;
            Ok(payload.iter().sum::<i64>())
        }
    })?;
    lua.globals().set("sum", sum)?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("failed to build tokio runtime");
    let total: i64 = rt.block_on(async { lua.load("return sum()").eval_async().await })?;
    assert_eq!(total, 60);
    Ok(())
}
