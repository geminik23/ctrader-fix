# cTrader FIX API in Rust

This project is currently in **active development** and is not yet ready for production use.

This is a Rust implementation of the FIX API for the cTrader trading platform. It's built using the async-std library, providing an asynchronous and simple interface for interacting with the cTrader platform through the Financial Information eXchange (FIX) protocol.


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
  - [x] FIX callback method for subscription - ~~waiting constantly in subscription method when market is closed~~
- [x] FIX identify with message type and id - ~~issue identify the response with sequence number~~
- [ ] TradeClient
  - [x] Add fetch methods
  - [x] Implement fetch_security_list to fetch the security list
  - [x] Implement fetch_positions
  - [ ] Implement fetch_all_orders
  - [ ] Implement new_market_order
  - [ ] Implement new_limit_order
  - [ ] Implement replace_order
  - [ ] Implement close_position and close_all_position
  - [ ] Implement cancel_order and cancel_all_position


	



## License
This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.
