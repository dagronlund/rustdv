// Rust for RTL Verification — Chapter 11, Figure 6
// "The shape of rustvm's driver (preview — signatures only)"


pub struct Driver<REQ, RSP = REQ> {
    pub seq_item_port: SeqItemPort<REQ, RSP>,
    // ...
}
