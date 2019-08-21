use std;
use std::cell::RefCell;
use std::panic::UnwindSafe;

use rutie::{self, Class, Object, RString, VM};

thread_local! {
    static RUTIE_SERDE_PANIC_MESSAGE: RefCell<Option<String>> = RefCell::new(None);
}

pub fn save_panic_message(message: String) {
    RUTIE_SERDE_PANIC_MESSAGE.with(|cell| cell.replace(Some(message)));
}

pub fn catch_and_raise<T, F>(exception_class: Class, f: F) -> T
where
    F: FnOnce() -> T,
    F: UnwindSafe,
{
    let res = std::panic::catch_unwind(f);
    match res {
        Ok(v) => v,
        Err(_) => {
            let msg = RUTIE_SERDE_PANIC_MESSAGE.with(|panic_cell| match panic_cell.replace(None) {
                Some(message) => message,
                None => "Unknown error".to_owned(),
            });
            let instance =
                exception_class.new_instance(&[RString::new_utf8(&msg).to_any_object()]);
            let exception = rutie::AnyException::from(instance.value());
            VM::raise_ex(exception);
            unreachable!("VM::raise_ex");
        }
    }
}
