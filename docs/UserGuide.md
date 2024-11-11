# Solana Bridge Guide

Sky uses Wormhole's native token transfer framework (NTT) to facilitate bridging tokens between Ethereum and Solana.

## Bridged Tokens

| Token | Address |
| ----- | ------- |
| USDS (Ethereum) | [0xdC035D45d973E3EC169d2276DDab16f1e407384F](https://etherscan.io/address/0xdC035D45d973E3EC169d2276DDab16f1e407384F)
| USDS (Solana)  | [USDSwr9ApdHk5bvJKMjzff41FfuX8bSxdKcR81vTwcA](https://explorer.solana.com/address/USDSwr9ApdHk5bvJKMjzff41FfuX8bSxdKcR81vTwcA) |

For end users, the [Portal Token Bridge](https://portalbridge.com/) is configured to use the Sky NTT deployment for transfers between networks.

## Using the Bridge

Wormhole provides some open source components to simplify the use of an NTT deployment in an application:

- [Wormhole Connect](https://github.com/wormhole-foundation/wormhole-connect) - A React component that can perform the cross-chain transfers using NTT
- [Wormhole Typescript SDK](https://github.com/wormhole-foundation/wormhole-sdk-ts) - A typescript SDK for using Wormhole for cross-chain messaging, including utilities for transfers with the NTT framework.

These components can be configured with the deployed addresses [detailed below](#wormhole-configuration) to perform cross-chain transfers.

## Using Contracts Directly

### Ethereum

They key functions for transfers are:

- `transfer` (NTT Manager) - Initiate an outbound transfer
- `receiveMessage` (Wormhole Transceiver) - Complete an inbound transfer

### Solana

They key instructions for transfer are:

- `transfer_burn` - Initiate an outbound transfer
- `receive_wormhole_message` - Initiate an inbound transfer
- `redeem` - Complete an inbound transfer


## Wormhole Configuration

### Limits

The bridge currently allows the transfer of 500M tokens per day (24 hours).

### Fees

Users are required to pay any fees on the source and destination networks, no additional fees are required.

### Deployed Addresses

| Component        | Address |
| ---------------- | ------- |
| **Ethereum**         |         |
| USDS NTT Manager          | [0x7d4958454a3f520bDA8be764d06591B054B0bf33](https://etherscan.io/address/0x7d4958454a3f520bDA8be764d06591B054B0bf33) |
| USDS Wormhole Transceiver | [0x16D2b6c87A18cB59DD59EFa3aa50055667cf481d](https://etherscan.io/address/0x16D2b6c87A18cB59DD59EFa3aa50055667cf481d) |
| **Solana**           |         |
| USDS NTT Manager          | [STTUVCMPuNbk21y1J6nqEGXSQ8HKvFmFBKnCvKHTrWn](https://explorer.solana.com/address/STTUVCMPuNbk21y1J6nqEGXSQ8HKvFmFBKnCvKHTrWn) |
| USDS Wormhole Transceiver | [4ZQYCg7ZiVeNp9DxUbgc4b9JpLXoX1RXYfMXS5saXpkC](https://explorer.solana.com/address/4ZQYCg7ZiVeNp9DxUbgc4b9JpLXoX1RXYfMXS5saXpkC) |



