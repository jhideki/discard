package api

import (
	"encoding/json"
	"fmt"
	"log"
	"net"
)

type TCPClient struct {
	conn net.Conn
}

type User struct {
	DisplayName string `json:"displayName"`
	NodeId      string `json:"nodeId"`
}

type IPCMessage struct {
	MsgType string `json:"type"`
	Content string `json:"data"`
}

type IPCResponse struct {
	MsgType string `json:"type"`
	Content string `json:"data"`
}

type SendUsersResp struct {
	Users []User `json:"users"`
}

type AddUser struct {
	NodeId      string `json:"nodeId"`
	DisplayName string `json:"displayName"`
}

type UpdateStatus struct {
	NodeId     string `json:"nodeId"`
	UserStatus string `json:"userStatus"`
}

type SendMessage struct {
	DisplayName string `json:"displayName"`
	Content     string `json:"content"`
}

func (client *TCPClient) AddUser(nodeId string, displayName string) {
	user := AddUser{nodeId, displayName}
	content, err := json.Marshal(user)
	if err != nil {
		log.Fatal("Error serializing content")
	}
	ipcMessage := IPCMessage{"AddUser", string(content)}
	client.Send(ipcMessage)
}

func (client *TCPClient) GetUsers() []User {
	message := IPCMessage{MsgType: "GetUsers", Content: ""}
	client.Send(message)
	buffer := make([]byte, 1024)
	n, err := client.conn.Read(buffer)
	if err != nil {
		log.Fatal("Error reading bytes from client")
	} else {
		println("Succesfully read ", n, " bytes")
	}
	var response struct {
		Users []User `json:"users"`
	}
	err = json.Unmarshal(buffer[:n], &response)
	if err != nil {
		log.Fatal("Error deserializing response")
	}

	return response.Users

}

func (client *TCPClient) SendMessage(nodeId string, displayName string) {
	user := AddUser{nodeId, displayName}
	content, err := json.Marshal(user)
	if err != nil {
		log.Fatal("Error serializing content")
	}
	ipcMessage := IPCMessage{"AddUser", string(content)}
	client.Send(ipcMessage)
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
