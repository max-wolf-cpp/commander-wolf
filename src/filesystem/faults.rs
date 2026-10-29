//! Thread-local, scoped I/O failure injection; never compiled into the application.
use std::{cell::RefCell, io, path::Path};
type Hook = Box<dyn FnMut(&str, &Path) -> io::Result<()>>;
thread_local! { static HOOK: RefCell<Option<Hook>> = RefCell::new(None); }

pub(super) fn check(step: &str, path: &Path) -> io::Result<()> {
    HOOK.with(|hook| match hook.borrow_mut().as_mut() {
        Some(hook) => hook(step, path),
        None => Ok(()),
    })
}

pub(super) fn with<T>(
    hook: impl FnMut(&str, &Path) -> io::Result<()> + 'static,
    run: impl FnOnce() -> T,
) -> T {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            HOOK.with(|hook| hook.borrow_mut().take());
        }
    }
    HOOK.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(Box::new(hook));
    });
    let _reset = Reset;
    run()
}
