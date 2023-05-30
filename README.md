# cTrader FIX API in Rust

This repository is an unofficial Rust implementation of the FIX API in Rust for the cTrader trading platform

Built using the async runtime library, it provides an asynchronous and simple interface for interacting with the cTrader platform through the Financial Information eXchange (FIX) protocol.

**This project is now ready for use. However, please note that it is still under active development and bugs may exist.**


## Cargo Features

This crate allows you to use `tokio` runtime featured in `async-std` by specifying features in your `Cargo.toml`. By default, it uses `async-std` with the `attributes` feature. 

To use the crate with the default configuration, add the following line to your `Cargo.toml`:

```toml
ctrader-fix = "0.4.4"
```

To use a specific Tokio configuration, specify the feature like this:

```toml
ctrader-fix = { version = "0.4.4", features = ["tokio1"] }
```

### Available Features

- **default**: Uses `async-std` with the `unstable` feature.
- **tokio1**: Uses `async-std` with the `unstable` and `tokio1` features.
- **tokio02**: Uses `async-std` with the `unstable` and `tokio02` features.
- **tokio03**: Uses `async-std` with the `unstable` and `tokio03` features.

Please note that you should only enable one of these features at a time.



## Progress

Below is the current progress on the development of this project:

- Base FixApi implementation :white_check_mark:
- Base requests :white_check_mark:
- Example code :white_check_mark:
    - Connect :white_check_mark:
    - Send logon :white_check_mark:
    - Send logout :white_check_mark:
    - Disconnect :white_check_mark:
- Handle responses :white_check_mark:
    - Implement response structure :white_check_mark:
    - Implement response handler - notify :white_check_mark:
- Add Error struct using `thiserror` :white_check_mark:
- MarketClient :white_check_mark:
    - Internal Market data Callback :white_check_mark:
    - Parsing response message :white_check_mark:
    - Subscribe the symbol for spot :white_check_mark:
    - Implement the check the request has accepted method :white_check_mark:
    - Test for parsing market datas :white_check_mark:
    - Unsubscribe the symbol for spot :white_check_mark:
    - Subscribe the symbol for depth :white_check_mark:
    - Unsubscribe the symbol for depth :white_check_mark:
    - Parsing the spot market data in callback :white_check_mark:
    - Add quote spot data method :white_check_mark:
    - Parsing the depth market data in callback :white_check_mark:
    - Parsing the incremental market data in callback :white_check_mark:
    - Market data handler in example code :white_check_mark:
    - Fix callback method for subscription :white_check_mark:
- FIXED identify with message type and id :white_check_mark:
- FIXED the issue of heartbeat :white_check_mark:
- TradeClient :white_check_mark:
    - Add fetch methods :white_check_mark:
    - Implement fetch_security_list to fetch the security list :white_check_mark:
    - Implement fetch_positions :white_check_mark:
    - Implement fetch_all_order_status :white_check_mark:
    - Implement new_market_order :white_check_mark:
    - Implement new_limit_order :white_check_mark:
    - Implement new_stop_order :white_check_mark:
    - Implement parse_func for ExecutionReport :white_check_mark:
    - Implement cancel_order :white_check_mark:
    - Implement replace_order :white_check_mark:
    - Implement adjust_position_size :white_check_mark:
    - Implement close_position :white_check_mark:
    - Added timeout in request methods :white_check_mark:
    - FIXED issue unhandled trade message (deadlock) :white_check_mark:
    - Add handler for trade execution :white_check_mark:
    - FIXED data parsing issue in Socket :white_check_mark:
	- Removed unnecessary arguments for new order methods :white_check_mark:


## License
This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.
