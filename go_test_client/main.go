package main

import (
	"encoding/binary"
	"fmt"
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

	uuid, err := performHandshake(conn, nil)
	if err != nil {
		panic(err)
	}

	fmt.Printf("%v\n", uuid)

	buffer := make([]byte, 16)

	for {
		_, err = conn.Read(buffer)
		if err != nil {
			fmt.Println("ERROR:", err)
			return
		}
		if checkOkSignal(buffer) {
			conn.Write(buffer[:4])
			continue
		}

		fmt.Printf("Received: %v\n", buffer)
	}

	var line string
	fmt.Scanf(line)
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
