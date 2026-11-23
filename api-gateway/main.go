package main

import (
	"bufio"
	"context"
	"flag"
	"fmt"
	"net"
	"os"
	"strconv"
	"strings"

	filevault "github.com/SIMIIIII/api-gateway/internal/grpc"
)

func Login(reader *bufio.Reader, client *filevault.ClientFileVault) error {
	fmt.Print("email> ")
	email, _ := reader.ReadString('\n')
	email = strings.TrimSpace(email)

	fmt.Print("Password> ")
	password, _ := reader.ReadString('\n')
	password = strings.TrimSpace(password)

	err := client.Login(context.Background(), email, password)
	if err != nil {
		return err
	}

	token, _ := client.GetToken()
	fmt.Println("TOKEN = ", token)
	return nil
}

func Register(reader *bufio.Reader, client *filevault.ClientFileVault) error {
	fmt.Print("username> ")
	username, _ := reader.ReadString('\n')
	username = strings.TrimSpace(username)

	fmt.Print("fullname> ")
	fullname, _ := reader.ReadString('\n')
	fullname = strings.TrimSpace(fullname)

	fmt.Print("email> ")
	email, _ := reader.ReadString('\n')
	email = strings.TrimSpace(email)

	fmt.Print("password> ")
	password, _ := reader.ReadString('\n')
	password = strings.TrimSpace(password)

	return client.Register(context.Background(), username, fullname, email, password)
}

func List(reader *bufio.Reader, client *filevault.ClientFileVault) error {
	fmt.Print("page (default 1)> ")
	pageStr, _ := reader.ReadString('\n')
	pageStr = strings.TrimSpace(pageStr)
	page := int32(1)
	if pageStr != "" {
		if p, err := strconv.ParseInt(pageStr, 10, 32); err == nil {
			page = int32(p)
		}
	}

	fmt.Print("limit (default 20)> ")
	limitStr, _ := reader.ReadString('\n')
	limitStr = strings.TrimSpace(limitStr)
	limit := int32(20)
	if limitStr != "" {
		if l, err := strconv.ParseInt(limitStr, 10, 32); err == nil {
			limit = int32(l)
		}
	}

	listFiles, err := client.List(context.Background(), page, limit)
	if err != nil {
		return err
	}

	fmt.Printf("\nTotal files: %d\n", listFiles.Total)
	fmt.Println("\n--- Files ---")
	for _, file := range listFiles.Files {
		fmt.Printf("ID: %d | Name: %s | Size: %d bytes | Downloads: %d | SHA256: %s\n",
			file.Id, file.Name, file.Size, file.NbDownloads, file.Sha256[:16]+"...")
	}

	return nil
}

func Upload(reader *bufio.Reader, client *filevault.ClientFileVault) error {
	fmt.Print("file path> ")
	filePath, _ := reader.ReadString('\n')
	filePath = strings.TrimSpace(filePath)

	data, err := os.ReadFile(filePath)
	if err != nil {
		return fmt.Errorf("failed to read file: %w", err)
	}

	resp, err := client.Upload(context.Background(), filePath, data)
	if err != nil {
		return err
	}

	fmt.Printf("Upload successful! File ID: %d\n", resp.Id)
	return nil
}

func Delete(reader *bufio.Reader, client *filevault.ClientFileVault) error {
	fmt.Print("file id> ")
	fileIDStr, _ := reader.ReadString('\n')
	fileIDStr = strings.TrimSpace(fileIDStr)

	fileID, err := strconv.ParseInt(fileIDStr, 10, 64)
	if err != nil {
		return fmt.Errorf("invalid file id: %w", err)
	}

	err = client.Delete(context.Background(), fileID)
	if err != nil {
		return err
	}

	fmt.Println("File deleted successfully!")
	return nil
}

func Download(reader *bufio.Reader, client *filevault.ClientFileVault) error {
	fmt.Print("file id> ")
	fileIDStr, _ := reader.ReadString('\n')
	fileIDStr = strings.TrimSpace(fileIDStr)

	fileID, err := strconv.ParseInt(fileIDStr, 10, 64)
	if err != nil {
		return fmt.Errorf("invalid file id: %w", err)
	}

	fmt.Print("output file path> ")
	outputPath, _ := reader.ReadString('\n')
	outputPath = strings.TrimSpace(outputPath)

	data, err := client.Download(context.Background(), fileID)
	if err != nil {
		return err
	}

	err = os.WriteFile(outputPath, data, 0644)
	if err != nil {
		return fmt.Errorf("failed to write file: %w", err)
	}

	fmt.Printf("File downloaded successfully! (%d bytes)\n", len(data))
	return nil
}

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
			err := Login(reader, client)
			if err != nil {
				fmt.Println("Error:", err)
				continue
			}
			fmt.Println("LOGGED IN")

		case "register":
			err := Register(reader, client)
			if err != nil {
				fmt.Println("Error:", err)
				continue
			}
			fmt.Println("Registration successful!")

		case "list":
			err := List(reader, client)
			if err != nil {
				fmt.Println("Error:", err)
				continue
			}

		case "upload":
			err := Upload(reader, client)
			if err != nil {
				fmt.Println("Error:", err)
				continue
			}

		case "download":
			err := Download(reader, client)
			if err != nil {
				fmt.Println("Error:", err)
				continue
			}

		case "delete":
			err := Delete(reader, client)
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
