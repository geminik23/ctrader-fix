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
- [ ] MarketClient
  - [x] Internal Market data Callback 
  - [x] Parsing response message.
  - [x] Subscribe the symbol for spot 
  - [x] Implement the check the request has accepted method.
  - [x] Test for parsing market datas
  - [x] Unsubscribe the symbol for spot
  - [ ] Subscribe the symbol for depth 
  - [ ] Unsubscribe the symbol for depth 
  - [ ] Parsing the market data in callback
  - [ ] Market data handler.
  - [ ] Add quote spot data method
  - [ ] Test : handle wrong response.
- [ ] TradeClient
	



## License
This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.
