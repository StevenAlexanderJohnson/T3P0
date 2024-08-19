package main

import "github.com/google/uuid"

type GameState struct {
	opp           *uuid.UUID
	me            *uuid.UUID
	board         [9]uint8
	turnNumber    uint8
	messageNumber uint8
	isPlayerTwo   bool
}

func NewGameState(request uint32) GameState {
	board := [9]uint8{}
	board_state := request & 0b111111111
	for i := 0; i < 9; i++ {
		board[i] = uint8(board_state>>i) & 1
	}

	turn_number := (request >> 27) & ((1 << 4) - 1)
	message_number := (request >> 21) & ((1 << 5) - 1)
	player_two := (request>>26)&1 == 1

	return GameState{
		board:         board,
		turnNumber:    uint8(turn_number),
		messageNumber: uint8(message_number),
		isPlayerTwo:   player_two,
	}
}

func (gs *GameState) GetOpponent() *uuid.UUID {
	return gs.opp
}

func (gs *GameState) GetMe() *uuid.UUID {
	return gs.me
}

func (gs *GameState) GetBoard() [9]uint8 {
	return gs.board
}

func (gs *GameState) GetTurnNumber() uint8 {
	return gs.turnNumber
}

func (gs *GameState) GetMessageNumber() uint8 {
	return gs.messageNumber
}

func (gs *GameState) IsPlayerTwo() bool {
	return gs.isPlayerTwo
}

func (gs *GameState) ToRequest() uint32 {
	request := uint32(0)
	for i := 0; i < 9; i++ {
		request |= uint32(gs.board[i]) << i
	}
	request |= uint32(gs.turnNumber) << 27
	request |= uint32(gs.messageNumber) << 21
	if gs.isPlayerTwo {
		request |= 1 << 26
	}
	return request
}
