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

func (gs *GameState) compareBoards(newState *GameState) error {
	differences := 0

	for i := 0; i < 9; i++ {
		if gs.board[i] != 0 && gs.board[i] != newState.board[i] {
			return fmt.Errorf("board state does not match")
		}
		if gs.board[i] != newState.board[i] {
			differences++
		}
	}
	if differences != 1 {
		return fmt.Errorf("new state has more than one difference")
	}
	return nil
}

func (gs *GameState) ValidateMove(newState *GameState) error {
	if gs.turnNumber+1 != newState.turnNumber {
		return fmt.Errorf("new game state has incorrect turn number")
	}
	if gs.messageNumber+1 != newState.messageNumber {
		return fmt.Errorf("new game state has incorrect message number")
	}
	if gs.isPlayerTwo == newState.isPlayerTwo {
		return fmt.Errorf("new game state has incorrect player two status")
	}
	if err := gs.compareBoards(newState); err != nil {
		return err
	}

	return nil
}

func (gs *GameState) MakeMove(boardIndex int) error {
	if gs.board[boardIndex] != 0 {
		return fmt.Errorf("board index is already taken")
	}
	if gs.isPlayerTwo && gs.turnNumber%2 == 0 {
		return fmt.Errorf("it is player one's turn")
	}
	if gs.isPlayerTwo {
		gs.board[boardIndex] = 2
		gs.isPlayerTwo = false
	} else {
		gs.board[boardIndex-1] = 1
		gs.isPlayerTwo = true
	}
	gs.turnNumber++
	gs.messageNumber++
	return nil
}
