package main

import (
	"discardtui/cmd/api"
	"github.com/rivo/tview"
)

func main() {
	conn, _ := api.NewTCPClient("127.0.0.1:7878")

	box := tview.NewBox().SetBorder(true).SetTitle("")
	if err := tview.NewApplication().SetRoot(box, true).Run(); err != nil {
		panic(err)
	}
}
