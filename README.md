# get_events

Using ethers to get events. Some RPC node might require block range. This app query events for all events from start to stop block.
If stop block didn't specify, It will query until last block.

```
Simple program to get events from rpc with specify block and block step

Usage: gather_data [OPTIONS] --rpc-url <RPC_URL> --target-address <TARGET_ADDRESS> --event-string <EVENT_STRING>

Options:
  -r, --rpc-url <RPC_URL>
  -t, --target-address <TARGET_ADDRESS>
  -e, --event-string <EVENT_STRING>      Example: "PairCreated(address,address,address,uint256)"
      --start-block <START_BLOCK>        [default: 0]
      --stop-block <STOP_BLOCK>
      --step-block <STEP_BLOCK>          [default: 2048]
      --block-number
      --block-hash
      --tx-hash
  -c, --client-number <CLIENT_NUMBER>    [default: 1]
  -l, --log
  -h, --help                             Print help
  -V, --version                          Print version
  ```
