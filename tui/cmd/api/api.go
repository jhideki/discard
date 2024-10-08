package api

import (
	"encoding/json"
	"fmt"
	"net"
	"os"
)

type IPCMessage struct {
	RunMessage string `json:"RunMessage"`
	Content    string `json:"Content"`
}

type TCPClient struct {
	conn net.Conn
}

func NewTCPClient(address string) (*TCPClient, error) {
	conn, err := net.Dial("tcp", address)
	if err != nil {
		return nil, fmt.Errorf("Error occured connnecting to: ", address)
	}

	return &TCPClient{conn: conn}, nil
}

func (client *TCPClient) Send(message IPCMessage) {
	jsonData, err := json.Marshal(message)
	if err != nil {
		return fmt.Errorf("Error marshaling data")
	}

}
