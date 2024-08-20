package main

import (
	"bufio"
	"encoding/binary"
	"fmt"
	"net"
	"os"
	"strconv"

	"github.com/google/uuid"
)

func checkOkSignal(buffer []byte) bool {
	return binary.BigEndian.Uint32(buffer[:4]) == 1<<31
}

type Connection struct {
	conn        net.Conn
	playerId    *uuid.UUID
	opponentId  *uuid.UUID
	imPlayerTwo bool
	gameState   *GameState
}

func NewConnection(conn net.Conn) Connection {
	return Connection{
		conn:        conn,
		playerId:    nil,
		opponentId:  nil,
		imPlayerTwo: false,
		gameState:   nil,
	}
}

func (c *Connection) PlayerId() *uuid.UUID {
	return c.playerId
}

func (c *Connection) OpponentId() *uuid.UUID {
	return c.opponentId
}

func (c *Connection) GameState() *GameState {
	return c.gameState
}

func (c *Connection) SetOpponentId(opponentId *uuid.UUID) {
	c.opponentId = opponentId
}

func (c *Connection) CloseConnection() error {
	return c.conn.Close()
}

func (c *Connection) PerformHandshake(preferredId *uuid.UUID) error {
	playerId := uuid.New()
	if preferredId != nil {
		playerId = *preferredId
	}
	buffer := make([]byte, 16)

	// Send initial Hello message
	binary.BigEndian.PutUint32(buffer[:4], uint32(1<<31))
	c.conn.Write(buffer[:4])

	n, err := c.conn.Read(buffer)
	if err != nil || n != 16 {
		return fmt.Errorf("server did not send a valid request to ok message")
	}

	// If we have a preferred Id we should discard the next message and send our preferred Id
	// Else just send another ok message to say we received our Id
	if preferredId != nil {
		fmt.Println("Sending preferred player id")
		buffer, err := preferredId.MarshalBinary()
		if err != nil {
			panic("Unable to parse preferredId")
		}
		c.conn.Write(buffer)

		// Check that the response from the server is ok
		fmt.Println("Checking server response")
		n, err := c.conn.Read(buffer)
		if err != nil || n != 4 {
			fmt.Println("ERROR:", err, n, buffer)
			return fmt.Errorf("server did not send a valid response to requesting player id")
		}

		if !checkOkSignal(buffer) {
			return fmt.Errorf("server did not respond with ok request to player id")
		}
	} else {
		receivedUUID, err := uuid.FromBytes(buffer)
		if err != nil {
			return fmt.Errorf("server did not send a valid player id")
		}
		playerId = receivedUUID
		fmt.Println("Received player id:", playerId)
		binary.BigEndian.PutUint32(buffer[:4], uint32(1<<31))
		c.conn.Write(buffer[:4])
	}

	c.playerId = &playerId

	return nil
}

func (c *Connection) WaitForOpponent() error {
	buffer := make([]byte, 16)
	for {
		n, err := c.conn.Read(buffer)
		if err != nil {
			fmt.Println("ERROR:", err)
			return fmt.Errorf("error occurred while reading from connection: %v", err)
		}
		if n == 4 && checkOkSignal(buffer) {
			c.conn.Write(buffer[:4])
			continue
		}
		output, err := uuid.FromBytes(buffer)
		if err != nil {
			fmt.Println("ERROR:", err)
			return fmt.Errorf("error parsing opponent user id: %v", err)
		}
		fmt.Printf("Opponent: %v\n", output)
		c.opponentId = &output
		return nil
	}
}

func (c *Connection) MessageLoop() error {
	displayChannel := make(chan []byte)
	errorChannel := make(chan error)
	// This goroutine handles communicating with the server
	go func() {
		for {
			buffer := make([]byte, 4)
			n, err := c.conn.Read(buffer)
			if err != nil {
				errorChannel <- err
			}
			if n != 4 {
				errorChannel <- fmt.Errorf("the message received from the server was invalid")
			}

			// If it's a heartbeat reply without sending to display channel
			if checkOkSignal(buffer) {
				n, err := c.conn.Write(buffer)
				if err != nil || n != 4 {
					errorChannel <- fmt.Errorf("error writing ok response: %v", err)
				}
				continue
			}

			displayChannel <- buffer
		}
	}()

	// This goroutine handles user input
	inputChannel := make(chan string)
	go func() {
		for {
			scanner := bufio.NewScanner(os.Stdin)
			for scanner.Scan() {
				inputChannel <- scanner.Text()
			}

			if err := scanner.Err(); err != nil {
				errorChannel <- err
			}
		}
	}()

	for {
		select {
		case input := <-inputChannel:
			if input == "exit" {
				return nil
			}
			move, err := strconv.Atoi(input)
			if err != nil {
				fmt.Println("Invalid input, please enter a number between 1 and 9")
				continue
			}

			if move < 1 || move > 9 {
				fmt.Println("Invalid input, please enter a number between 1 and 9")
				continue
			}

			if c.gameState == nil {
				panic("game state is nil")
			}

			err = c.gameState.MakeMove(move)
			if err != nil {
				fmt.Println("Invalid move, please try again")
				continue
			}

			payload := make([]byte, 4)
			binary.BigEndian.PutUint32(payload, c.gameState.ToRequest())
			c.conn.Write(payload)
			c.DrawBoard()

		case message := <-displayChannel:
			request := binary.BigEndian.Uint32(message)
			newGameState := NewGameState(request)
			if c.gameState == nil {
				c.gameState = NewGameState(0)
				c.imPlayerTwo = newGameState.IsPlayerTwo()
				newGameState = c.gameState
			}
			c.gameState.UpdateState(newGameState)
			c.DrawBoard()

		case err := <-errorChannel:
			return err
		}
	}
}

func (c *Connection) DrawBoard() {
	board := c.gameState.Board()

	// clear the screen
	fmt.Print("\033[H\033[2J")
	fmt.Println("T3P0 - Tic Tac Toe")
	if c.imPlayerTwo {
		fmt.Printf("Player 1: X -> %v\n", c.opponentId)
		fmt.Printf("Player 2: O -> %v (you)\n\n", c.playerId)
	} else {
		fmt.Printf("Player 1: X -> %v (you)\n", c.playerId)
		fmt.Printf("Player 2: O -> %v\n\n", c.opponentId)
	}

	fmt.Printf("Turn: %d\n", c.gameState.TurnNumber())
	fmt.Printf("Message: %d\n\n", c.gameState.MessageNumber())
	for i := 0; i < 3; i++ {
		if i%3 != 0 {
			fmt.Println("-----------")
		}
		for j := 0; j < 3; j++ {
			value := strconv.Itoa(i*3 + j + 1)
			if board[i*3+j] != 0 {
				if board[i*3+j] == 1 {
					value = "X"
				} else {
					value = "O"
				}
			}
			if j != 2 {
				fmt.Printf(" %s |", value)
			} else {
				fmt.Printf(" %s ", value)
			}
		}
		fmt.Printf("\n")
	}
	fmt.Printf("Enter a move (1-9): ")
}
