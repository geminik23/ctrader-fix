# cTrader FIX API in Rust

This repository is an unofficial Rust implementation of the FIX API in Rust for the cTrader trading platform

Built using the async runtime library, it provides an asynchronous and simple interface for interacting with the cTrader platform through the Financial Information eXchange (FIX) protocol.

**This project is now ready for use. However, please note that it is still under active development and bugs may exist.**

## Cargo Features

This crate allows you to use `tokio` runtime featured in `async-std` by specifying features in your `Cargo.toml`. By default, it uses `async-std` with the `attributes` feature. 

To use the crate with the default configuration, add the following line to your `Cargo.toml`:

```toml
ctrader-fix = "0.5.2"
```

To use a specific Tokio configuration, specify the feature like this:

```toml
ctrader-fix = { version = "0.5.2", features = ["tokio1"] }
```

### Available Features

- **default**: Uses `async-std` with the `unstable` feature.
- **tokio1**: Uses `async-std` with the `unstable` and `tokio1` features.
- **tokio02**: Uses `async-std` with the `unstable` and `tokio02` features.
- **tokio03**: Uses `async-std` with the `unstable` and `tokio03` features.

Please note that you should only enable one of these features at a time.


## Progress Records

For details on the progress achieved, check the [PROGRESS.md](./PROGRESS.md) file.


## Related Works

- **quant-suite** [link](https://github.com/geminik23/quant-suite): Aimed at creating tools for auto-trading, back-testing, and market analysis. 

## License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.


