package main

import (
	"bufio"
	"flag"
	"fmt"
	"net"
	"os"
	"strings"

	filevault "github.com/SIMIIIII/api-gateway/internal/grpc"
	handlers "github.com/SIMIIIII/api-gateway/internal/handlers"
)

func main() {
	host := flag.String("host", "localhost", "Le host")
	port := flag.String("port", "8080", "le port")
	flag.Parse()

	addr := net.JoinHostPort(*host, *port)

	client, err := filevault.Connect(addr, "../pki/ca/ca.crt")
	if err != nil {
		fmt.Print("Error: ", err)
		os.Exit(1)
	}

	reader := bufio.NewReader(os.Stdin)

	for {
		fmt.Println("")
		fmt.Print("Filedrop> ")
		cmd, _ := reader.ReadString('\n')
		cmd = strings.TrimSpace(cmd)

		switch cmd {
		case "exit":
			os.Exit(0)
		case "login":
			err := handlers.Login(reader, client)
			if err != nil {
				fmt.Println("Error:", err)
				continue
			}
			fmt.Println("LOGGED IN")

		case "register":
			err := handlers.Register(reader, client)
			if err != nil {
				fmt.Println("Error:", err)
				continue
			}
			fmt.Println("Registration successful!")

		case "list":
			err := handlers.List(reader, client)
			if err != nil {
				fmt.Println("Error:", err)
				continue
			}

		case "upload":
			err := handlers.Upload(reader, client)
			if err != nil {
				fmt.Println("Error:", err)
				continue
			}

		case "download":
			err := handlers.Download(reader, client)
			if err != nil {
				fmt.Println("Error:", err)
				continue
			}

		case "delete":
			err := handlers.Delete(reader, client)
			if err != nil {
				fmt.Println("Error:", err)
				continue
			}

		case "get token":
			token, err := client.GetToken()
			if err != nil {
				fmt.Println("No token, please log in")
				continue
			}
			fmt.Println("TOKEN = ", token)

		case "help":
			fmt.Println("Available commands:")
			fmt.Println("  login    - Login to the server")
			fmt.Println("  register - Create a new account")
			fmt.Println("  list     - List your files")
			fmt.Println("  upload   - Upload a file")
			fmt.Println("  download - Download a file")
			fmt.Println("  delete   - Delete a file")
			fmt.Println("  help     - Show this help message")
			fmt.Println("  exit     - Exit the application")

		default:
			fmt.Println("Unknown command:", cmd)
			fmt.Println("Type 'help' for available commands")
		}
	}
}
