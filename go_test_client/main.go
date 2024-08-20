package main

import (
	"fmt"
	"io"
	"net"
)

func main() {
	conn, err := net.Dial("tcp", "localhost:8000")
	if err != nil {
		fmt.Println("ERROR:", err)
		return
	}
	connection := NewConnection(conn)
	defer func() {
		err := connection.CloseConnection()
		if err != nil {
			fmt.Println("ERROR:", err)
		}
	}()

	// preferredId, err := uuid.Parse("f7e0d1e9-079f-f242-a962-fe33ebabe275")
	// preferredId, err := uuid.Parse("6b2792ba-d807-4866-8326-4f7b2fd6c956")
	// if err != nil {
	// 	panic("Unable to parse preferredId")
	// }

	err = connection.PerformHandshake(nil)
	if err != nil {
		panic(err)
	}

	err = connection.WaitForOpponent()
	if err != nil {
		panic(err)
	}

	err = connection.MessageLoop()
	if err != nil {
		if err == io.EOF {
			fmt.Println("\n\nServer has ended the connection.")
		} else {
			panic(err)
		}
	}

	fmt.Println("Ending session, closing connection")
}
