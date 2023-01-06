use std::sync::Once;

use sqlite::{self, Connection};

fn get_connect(file: &str) -> Option<&mut Connection> {
    static mut CONNECT: Option<Connection> = None;
    static CONNECT_ONCE: Once = Once::new();
    CONNECT_ONCE.call_once(|| {
        unsafe {
            CONNECT = sqlite::open(file).ok();
        }
    });
    unsafe {
        CONNECT.as_mut()
    }
}

