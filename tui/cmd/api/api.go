package api

import (
	"encoding/json"
	"fmt"
	"net"
)

type TCPClient struct {
	conn net.Conn
}

type IPCMessage struct {
	MsgType string `json:"type"`
	Content string `json:"data"`
}

type Adduser struct {
	NodeId      string `json:"nodeId"`
	DisplayName string `json:"displayName"`
}

type UpdateStatus struct {
	NodeId     string `json:"nodeId"`
	UserStatus string `json:"userStatus"`
}

type SendMessage struct {
	NodeId  string `json:"userStatus"`
	Content string `json:"content"`
}

func NewTCPClient(address string) (*TCPClient, error) {
	conn, err := net.Dial("tcp", address)
	if err != nil {
		return nil, fmt.Errorf("Error occured connnecting to: ", address)
	}

	return &TCPClient{conn: conn}, nil
}

func (client *TCPClient) Send(message IPCMessage) error {
	jsonData, err := json.Marshal(message)
	if err != nil {
		return fmt.Errorf("Error marshaling data")
	}
	_, err = client.conn.Write(jsonData)
	if err != nil {
		return fmt.Errorf("Error writing data to client")
	}
	fmt.Println("Data sent succesfully")
	return nil
}
