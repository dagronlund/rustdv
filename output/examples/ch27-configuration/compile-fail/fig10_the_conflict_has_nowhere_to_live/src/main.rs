// Chapter 27, Figure 10: The parent/child conflict has nowhere to live
// This program FAILS TO COMPILE on purpose. See EXPECTED.txt.

struct MsgEnvConfig {
    loga_msg: String,
}

fn main() {
    // The test says one thing, "the env" says another — in the ConfigDB,
    // a precedence rule decided. In a struct literal, there is no second slot:
    let config = MsgEnvConfig {
        loga_msg: "PARENT RULES!".to_string(),
        loga_msg: "CHILD RULES!".to_string(),
    };
    let _ = config;
}
