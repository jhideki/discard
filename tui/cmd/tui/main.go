package main

import (
	"discardtui/cmd/api"
	"github.com/rivo/tview"
)

func main() {
	conn, _ := api.NewTCPClient("127.0.0.1:7878")
	var users []api.User
	app := tview.NewApplication()

	// Create primitives
	header := tview.NewTextView().
		SetText("Discard").
		SetTextAlign(tview.AlignCenter).
		SetBorder(true)

	users = conn.GetUsers()
	list := tview.NewList()
	for user, _ := range users {
		list.AddItem(user.DisplayName, user.NodeId, '1', nil)
	}

	content := tview.NewTextView().
		SetText("Content Area")

	footer := tview.NewTextView().
		SetText("Footer Section").
		SetTextAlign(tview.AlignCenter)

	// Layout using Flex
	layout := tview.NewFlex().SetDirection(tview.FlexRow).
		AddItem(header, 3, 1, false).
		AddItem(list, 0, 1, true).
		AddItem(content, 0, 2, false).
		AddItem(footer, 3, 1, false)

	// Run the application
	if err := app.SetRoot(layout, true).Run(); err != nil {
		panic(err)
	}
}
