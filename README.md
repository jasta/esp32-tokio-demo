# Tokio running on esp32!

This repo demonstrates a working hello world utilizing upstreamed [tokio support for esp32](https://github.com/tokio-rs/tokio/issues/5867).  The tokio executor and I/O reactor are both working fully with no known gotchas.  Third party libraries utilizing tokio can also be used freely, such as [coap-server-rs](https://github.com/jasta/coap-server-rs).

## Quickstart

Recommended that you use an ESP32C3 as upstream Rust support is currently better, though if you do choose any other in the ESP32 family be sure to check out the guides in the [Rust on ESP Book](https://esp-rs.github.io/book/installation/index.html).

```
git clone https://github.com/jasta/esp32-tokio-demo
cd esp32-tokio-demo
vim src/main.rs # <-- edit the WIFI_SSID/WIFI_PASS variables!
cargo run
```

After the board connects to Wi-Fi, you can test that things are working with:

```
$ echo hello | nc -N -v espressif 12345
hello
```

You may optionally use `cargo run --target xtensa-esp32-espidf` or any of the other supported targets which should work provided that you followed the [esp-rs](https://github.com/esp-rs) installation instructions above.

## Future work

esp32 support still requires the experimental `mio_unsupported_force_poll_poll` feature.  To mature this support, we'll need to address a few more uptsream issues:

- [ ] https://github.com/tokio-rs/mio/issues/1703
- [ ] https://github.com/tokio-rs/tokio/issues/5866

## Alternatives

To use async on esp32, you can also use a number of other executors and reactor combos, including smol and async-task.  ~~Upstream esp-rs support appears to be leaning toward [edge-executor](https://github.com/ivmarkov/edge-executor) which can be utilized to support ISR-based wakeups and other useful embedded features that tokio lacks.  See https://github.com/ivmarkov/rust-esp32-std-demo/issues/153#issuecomment-1633598784 for discussion of some of the advantages with this approach.~~ Upstream esp-rs is moving toward [support for all executors](https://github.com/esp-rs/esp-idf-hal/pull/315) generically, including tokio!

The tokio support, however, is still useful to broadly make libraries making use of tokio compatible with esp32, even ones that use the tokio I/O reactor.  You can also have multiple executors living side-by-side in a process by simply isolating them to separate pthreads.  This can actually be a useful strategy intentionally to support high and lower priority task queues simultaneously active to achieve realtime performance characteristics.
