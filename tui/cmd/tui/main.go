package main

import (
	"discardtui/cmd/api"
	"github.com/rivo/tview"
)

func main() {
	api.Connect()
	box := tview.NewBox().SetBorder(true).SetTitle("Hello, world!")
	if err := tview.NewApplication().SetRoot(box, true).Run(); err != nil {
		panic(err)
	}
}
