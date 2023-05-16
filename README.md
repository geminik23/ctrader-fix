# cTrader FIX API in Rust

This project is currently in **active development** and is not yet ready for production use.

This is a Rust implementation of the FIX API for the cTrader trading platform. It's built using the async-std library, providing an asynchronous and simple interface for interacting with the cTrader platform through the Financial Information eXchange (FIX) protocol.


## Cargo Features

This crate allows you to use `tokio` runtime featured in `async-std` by specifying features in your `Cargo.toml`. By default, it uses `async-std` with the `attributes` feature. 

To use the crate with the default configuration, add the following line to your `Cargo.toml`:

```toml
ctrader-fix = "0.3"
```

To use a specific Tokio configuration, specify the feature like this:

```toml
ctrader-fix = { version = "0.3", features = ["tokio1"] }
```

### Available Features

- **default**: Uses `async-std` with the `unstable` feature.
- **tokio1**: Uses `async-std` with the `unstable` and `tokio1` features.
- **tokio02**: Uses `async-std` with the `unstable` and `tokio02` features.
- **tokio03**: Uses `async-std` with the `unstable` and `tokio03` features.

Please note that you should only enable one of these features at a time.


## To-Do 

- [x] Base FixApi Implementation
- [x] Base Requests
- [x] Example code
  - [x] Connect
  - [x] Send logon
  - [x] Send logout
  - [x] Disconnect
- [x] Handle responses
  - [x] Implement response structure
  - [x] Implement response handler - notify
- [x] Add Error struct using `thiserror`
- [x] MarketClient
  - [x] Internal Market data Callback 
  - [x] Parsing response message.
  - [x] Subscribe the symbol for spot 
  - [x] Implement the check the request has accepted method.
  - [x] Test for parsing market datas
  - [x] Unsubscribe the symbol for spot
  - [x] Subscribe the symbol for depth 
  - [x] Unsubscribe the symbol for depth 
  - [x] Parsing the spot market data in callback
  - [x] Add quote spot data method
  - [x] Parsing the depth market data in callback
  - [x] Parsing the incremental market data in callback
  - [x] Market data handler in example code.
  - [x] Fix callback method for subscription - ~~waiting constantly in subscription method when market is closed~~
- [x] FIXED identify with message type and id - ~~issue identify the response with sequence number~~
- [x] FIXED the issue of heartbeat.
- [x] TradeClient 
  - [x] Add fetch methods
  - [x] Implement fetch_security_list to fetch the security list
  - [x] Implement fetch_positions
  - [x] Implement fetch_all_order_status
  - [x] Implement new_market_order
  - [x] Implement new_limit_order
  - [x] Implement new_stop_order
  - [x] Implement parse_func for ExecutionReport
  - [x] Implement cancel_order
  - [x] Implement replace_order
  - [x] Implement adjust_position_size
  - [x] Implement close_position
  - [x] Added timeout in request methods.
  - [x] FIXED issue unhandled trade message (deadlock)
  - [x] Add handler for trade execution.
	



## License
This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.
