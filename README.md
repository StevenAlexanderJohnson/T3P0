# T3P0

T3P0 is short for Tic Tac Toe Protocol Version 0.
It's purpose is to establish an application layer protocol that allows for two computers to share the state of a Tic Tac Toe game.

## Overview

T3P0 uses TCP to establish communications between computers.
Packages are sent in binary and are unsigned 32 bit integers.


The protocol is in early development so many of the designated bits and unsigned bits are bound to change.

## The Protocol

### Transport Layer

T3P0 uses TCP as the transport layer protocol.
This allows for consistent communication between the client and the server.

### Handshake

1. The client will establish a connection with the server.
2. The server will respond with a player ID which is a 128 bit UUID.
3. Client Response
    - The client can return an Ok signal and that's the end of the handshake.
    - The client can respond with their own 128 bit UUID.
    If they do the server will return with an Ok signal if that uuid is accepted.

The reason why a client can request a client ID is it allows for future expansions.
Examples would be friends or reconnecting to disconnected sessions.

### Message Format

Messages are unsigned 32 bit integers and use big-endian bit numbering.

The left most bit is considered bit number one.

### Bit 1
There are two possible message types. Data and Ok.

### Bits 2-5

Turn number uses 4 buts for a max of 16 possible moves.
It only takes 9 at max for a game but 3 bits is too few.

### Bit 6

This big is a flag showing if it's player 1 or player 2s turn.
This flag comes into use when validating requests on the server.

### Bits 7-11

5 bits can represent up to 32 possible moves.
This opens the possibility of best of 3s which will use at most 27.

### Bits 12-23

These bits are currently not being used.

### Bits 24-32

These bits represent the board state.
The astute reader will think "tic tac toe has three states per position".
When receiving the board state from the server, it will always be from the opponents point of view.
You already know your own board state.

### Bit Table

|Bit | Meaning      |
|----|--------------|
| 1  | Message Type |
|----|--------------|
| 2  | Turn Number  |
| 3  |              |
| 4  |              |
| 5  |              |
|----|--------------|
| 6  | Is P2 Turn   |
|----|--------------|
| 7  |Message Number|
| 8  |              |
| 9  |              |
| 10 |              |
| 11 |              |
|----|--------------|
| 12 | Unused       |
| 13 |              |
| 14 |              |
| 15 |              |
| 16 |              |
| 17 |              |
| 18 |              |
| 19 |              |
| 20 |              |
| 21 |              |
| 22 |              |
| 23 |              |
|----|--------------|
| 24 | Board State  |
| 25 |              |
| 26 |              |
| 27 |              |
| 28 |              |
| 29 |              |
| 30 |              |
| 31 |              |
| 32 |              |
|----|--------------|


### State Management
- **Initial State**: Connection not established
- **Connected State**: Connection established
- **Not Matched State**: Player is not matched with opponent and is waiting.
- **Opponent Matched State**: Player is matched with a player.
- **Rocker Paper Scissors**: Players play rock paper scissors to determine who goes first.
- **Game State**: Players are in game.
- **End State**: Player has disconnected or best of three is complete.
Connection is reset and you have to reconnect to find another opponent.

### Security Considerations
This isn't a very serious protocol and is not secure.

Players are not connected directly which means that IPs are never exposed.