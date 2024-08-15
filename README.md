<h1 align="center">
🤡joker_cli
</h1>

As [Block Joker](https://test2.blockjoker.org/home) said, no real use bot maybe.

## Caution

This is for learning and praticing, use it may cause you be banned.

## Features

- [✔] Support both v1 and v2.
- [✔] Multi cores calculate hash enable.

## Usage

```
Usage: joker_cli.exe [OPTIONS] <COMMAND>

Commands:
  mine     Start mining
  info     Account info
  records  Mine records
  help     Print this message or the help of the given subcommand(s)

Options:
  -c, --cookie <COOKIE>                  your website cookie
  -S, --session_cookie <SESSION_COOKIE>  session_cookie [default: ]
  -A, --authorization <AUTHORIZATION>    authorization.
  -P, --proxy <PROXY>                    proxy
      --version <VERSION>                joker version [default: 2]
  -h, --help                             Print help
  -V, --version                          Print version
```

```
Start mining

Usage: joker_cli.exe mine [OPTIONS]

Options:
  --cores <cores>                    Cpu core you use [default: 2]
```

1. Get your cookie token in the [Block Joker](https://test2.blockjoker.org/home)

2. Get your auth token in the [Block Joker](https://test2.blockjoker.org/home) localstorage.

3. Maybe you need also a session cookie which is genereted by `start mining` action. (Someone may not, but also work)

## Buy me a coffee

[![BTC](https://img.shields.io/badge/BTC-wallet-F7931A?logo=bitcoin)](https://btcscan.org/ "View BTC address") bc1qfsg983l9adyc6fq96v8qjzax6fk3a39muspers

[![GitHub tag](https://img.shields.io/badge/EVM-wallet-3C3C3D?logo=ethereum)](https://etherscan.io/ "View EVM address") 0x5022519902ffFb8EeA99A8E7Ae53586b879f89Fd0x5022519902ffFb8EeA99A8E7Ae53586b879f89Fd

[![GitHub tag](https://img.shields.io/badge/SOL-wallet-9945FF?logo=solana)](https://solscan.io/ "View SOL address") 6xFaxgz6tr8RV1bvcdfp97apf7usinXJQHtFUUTQ7Sir

## License
GPL-3.0
