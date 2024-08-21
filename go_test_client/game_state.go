package main

import (
	"fmt"
)

type GameState struct {
	board         [9]uint8
	turnNumber    uint8
	messageNumber uint8
	isPlayerTwo   bool
}

func NewGameState(request uint32) *GameState {
	board := [9]uint8{}
	board_state := request & 0b111111111
	for i := 0; i < 9; i++ {
		board[i] = uint8((board_state >> (8 - i))) & 1
	}

	turn_number := (request >> 27) & ((1 << 4) - 1)
	message_number := (request >> 21) & ((1 << 5) - 1)
	player_two := (request>>26)&1 == 1

	return &GameState{
		board:         board,
		turnNumber:    uint8(turn_number),
		messageNumber: uint8(message_number),
		isPlayerTwo:   player_two,
	}
}

func (gs *GameState) Board() [9]uint8 {
	return gs.board
}

func (gs *GameState) TurnNumber() uint8 {
	return gs.turnNumber
}

func (gs *GameState) MessageNumber() uint8 {
	return gs.messageNumber
}

func (gs *GameState) IsPlayerTwo() bool {
	return gs.isPlayerTwo
}

func (gs *GameState) ToRequest() uint32 {
	var request uint32
	for i, cell := range gs.board {
		if (cell == 1 && gs.isPlayerTwo) || (cell == 2 && !gs.isPlayerTwo) {
			request |= 1 << i
		}
	}

	request |= uint32(gs.turnNumber) << 27
	request |= uint32(gs.messageNumber) << 21
	if gs.isPlayerTwo {
		request |= 1 << 26
	}
	return request
}

func (gs *GameState) MakeMove(boardIndex int) error {
	if gs.board[boardIndex-1] != 0 {
		return fmt.Errorf("board index is already taken")
	}
	if gs.isPlayerTwo && gs.turnNumber%2 == 0 {
		return fmt.Errorf("it is player one's turn")
	}
	if !gs.isPlayerTwo && gs.turnNumber%2 == 1 {
		return fmt.Errorf("it is player two's turn")
	}
	if gs.isPlayerTwo {
		gs.board[boardIndex-1] = 2
		gs.isPlayerTwo = false
	} else {
		gs.board[boardIndex-1] = 1
		gs.isPlayerTwo = true
	}
	gs.turnNumber++
	gs.messageNumber++
	return nil
}

func (gs *GameState) UpdateState(newState *GameState) error {
	for i := 0; i < 9; i++ {
		if newState.board[i] != 0 {
			if newState.isPlayerTwo {
				gs.board[i] = 1
			} else {
				gs.board[i] = 2
			}
		}
	}

	gs.turnNumber = newState.turnNumber
	gs.messageNumber = newState.messageNumber
	gs.isPlayerTwo = newState.isPlayerTwo

	return nil
}
