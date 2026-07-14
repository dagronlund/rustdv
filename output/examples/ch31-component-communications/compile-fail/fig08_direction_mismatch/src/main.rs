// Chapter 31, Figure 8: A direction mismatch is a type error
// This program FAILS TO COMPILE on purpose. See EXPECTED.txt.

use rustdv::{channel, Receiver};

struct Driver {
    items: Receiver<u32>, // the driver consumes items
}

fn main() {
    let (tx, _rx) = channel::<u32>(1);
    // pyuvm: connecting a port to a port raised UVMTLMConnectionError at
    // run time. Here, handing the driver the wrong end doesn't compile:
    let _driver = Driver { items: tx };
}
