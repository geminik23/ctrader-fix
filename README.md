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
  - [x] Market data Callback 
  - [x] parsing response message.
  - [ ] subscribe the symbol.
  - [ ] unsubscribe the symbol.

	





## License
This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.
