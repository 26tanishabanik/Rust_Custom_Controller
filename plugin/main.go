package main


import (
	"github.com/aquasecurity/table"
	"os"
	"fmt"
)

func main(){
	fmt.Println("")
	fmt.Println(".....Welcome to KuBeverage Cafe....")
	fmt.Println("")
	fmt.Println("")
	t := table.New(os.Stdout)
	t.SetHeaders("Beverage Name", "Beverage Type")
	t.AddRow("Tea", "Milk")
	t.AddRow("Tea", "Black")
	t.AddRow("Tea", "Green")
	t.AddRow("Tea", "Herbal")
	t.AddRow("Coffee", "Cappucino")
	t.AddRow("Coffee", "Decaf")
	t.AddRow("Coffee", "Macchiato")
	t.AddRow("Coffee", "Americano")
	t.AddRow("Coffee", "Expresso")
	t.AddRow("Coffee", "Mocha Latte")
	t.AddRow("Coffee", "Iced")
	t.AddRow("Coffee", "Frappuccino")
	t.Render()
}
