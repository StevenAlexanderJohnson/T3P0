# T3P0

T3P0 is short for Tic Tac Toe Protocol Version 0.
It's purpose is to establish an application layer protocol that allows for two computers to share the state of a Tic Tac Toe game asynchronously.

## Overview

T3P0 uses TCP to establish communications between computers.

Packages are sent in binary and are unsigned 32 bit integers.

The protocol is in early development so many of the designated bits and unsigned bits are bound to change.
