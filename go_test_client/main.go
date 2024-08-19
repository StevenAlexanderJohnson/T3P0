package main

import (
	"encoding/binary"
	"fmt"
	"io"
	"net"

	"github.com/google/uuid"
)

func main() {
	conn, err := net.Dial("tcp", "localhost:8000")
	if err != nil {
		fmt.Println("ERROR:", err)
		return
	}
	defer conn.Close()

	// preferredId, err := uuid.Parse("f7e0d1e9-079f-f242-a962-fe33ebabe275")
	// if err != nil {
	// 	panic("Unable to parse preferredId")
	// }

	id, err := performHandshake(conn, nil)
	if err != nil {
		panic(err)
	}
	fmt.Printf("%v\n", id)

	_, err = waitForOpponent(conn)
	if err != nil {
		panic(err)
	}

	err = messageLoop(conn)
	if err != nil {
		if err == io.EOF {
			fmt.Println("Server has ended the connection.")
		} else {
			panic(err)
		}
	}

	fmt.Println("Ending session, closing connection")
}

func checkOkSignal(buffer []byte) bool {
	return binary.BigEndian.Uint32(buffer[:4]) == 1<<31
}

func performHandshake(connection net.Conn, preferredId *uuid.UUID) (*uuid.UUID, error) {
	playerId := uuid.New()
	if preferredId != nil {
		playerId = *preferredId
	}
	buffer := make([]byte, 16)

	// Send initial Hello message
	binary.BigEndian.PutUint32(buffer[:4], uint32(1<<31))
	connection.Write(buffer[:4])

	n, err := connection.Read(buffer)
	if err != nil || n != 16 {
		return nil, fmt.Errorf("server did not send a valid request to ok message")
	}

	// If we have a preferred Id we should discard the next message and send our preferred Id
	// Else just send another ok message to say we received our Id
	if preferredId != nil {
		fmt.Println("Sending preferred player id")
		buffer, err := preferredId.MarshalBinary()
		if err != nil {
			panic("Unable to parse preferredId")
		}
		connection.Write(buffer)

		// Check that the response from the server is ok
		fmt.Println("Checking server response")
		n, err := connection.Read(buffer)
		if err != nil || n != 4 {
			fmt.Println("ERROR:", err, n, buffer)
			return nil, fmt.Errorf("server did not send a valid response to requesting player id")
		}

		if !checkOkSignal(buffer) {
			return nil, fmt.Errorf("server did not respond with ok request to player id")
		}
	} else {
		receivedUUID, err := uuid.FromBytes(buffer)
		if err != nil {
			return nil, fmt.Errorf("server did not send a valid player id")
		}
		playerId = receivedUUID
		fmt.Println("Received player id:", playerId)
		binary.BigEndian.PutUint32(buffer[:4], uint32(1<<31))
		connection.Write(buffer[:4])
	}

	return &playerId, nil
}

func waitForOpponent(connection net.Conn) (*uuid.UUID, error) {
	buffer := make([]byte, 16)
	for {
		n, err := connection.Read(buffer)
		if err != nil {
			fmt.Println("ERROR:", err)
			return nil, fmt.Errorf("error occurred while reading from connection: %v", err)
		}
		if n == 4 && checkOkSignal(buffer) {
			fmt.Println("Heartbeat")
			connection.Write(buffer[:4])
			continue
		}
		output, err := uuid.FromBytes(buffer)
		if err != nil {
			fmt.Println("ERROR:", err)
			return nil, fmt.Errorf("error parsing opponent user id: %v", err)
		}
		fmt.Printf("Received: %v\n", output)
		return &output, nil
	}
}

func messageLoop(connection net.Conn) error {
	displayChannel := make(chan []byte, 5)
	errorChannel := make(chan error)

	// This goroutine handles communicating with the server
	go func() {
		buffer := make([]byte, 4)
		for {
			n, err := connection.Read(buffer)
			if err != nil {
				errorChannel <- err
				return
			}
			if n != 4 {
				errorChannel <- fmt.Errorf("the message received from the server was invalid")
				return
			}

			// If it's a heartbeat reply without sending to display channel
			if checkOkSignal(buffer) {
				n, err := connection.Write(buffer)
				if err != nil || n != 4 {
					errorChannel <- fmt.Errorf("error writing ok response: %v", err)
					return
				}
				continue
			}

			displayChannel <- buffer
		}
	}()

	for {
		select {
		case message := <-displayChannel:
			fmt.Println(message)
		case err := <-errorChannel:
			return err
		}
	}
}
