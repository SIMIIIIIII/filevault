package handlers

import (
	"bufio"
	"context"
	"fmt"
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

func Logout(reader *bufio.Reader, client *filevault.ClientFileVault) {
	client.InvalidateToken()
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
